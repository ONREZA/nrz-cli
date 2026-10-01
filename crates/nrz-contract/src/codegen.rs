//! Shared deterministic Rust generation for the supported contract schema subset.
//!
//! Root `$defs` and `definitions` references are consumed directly by schemars
//! and typify. This is not a general draft-2020-12 schema implementation.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail, ensure};
use quote::quote;
use schemars08::schema::{RootSchema, SchemaObject};
use schemars08::visit::{Visitor, visit_schema_object};
use typify::{TypeSpace, TypeSpaceSettings};

/// A caller-owned schema and its generated Rust names.
pub struct SchemaDocument<'a> {
    pub module: &'a str,
    pub schema: &'a str,
    /// Optional top-level `&str` constant containing the exact input schema.
    pub embedded_constant: Option<&'a str>,
}

/// Generate and format all documents without writing any files.
pub fn generate(documents: &[SchemaDocument<'_>], header: &str) -> Result<String> {
    let mut items = proc_macro2::TokenStream::new();
    let mut names = BTreeSet::new();
    for document in documents {
        let module: syn::Ident = syn::parse_str(document.module)
            .with_context(|| format!("invalid module name: {}", document.module))?;
        ensure!(
            names.insert(document.module),
            "duplicate Rust name: {module}"
        );
        let mut root: RootSchema = serde_json::from_str(document.schema)
            .with_context(|| format!("deserialize root schema {}", document.module))?;
        validate(&mut root).with_context(|| format!("validate schema {}", document.module))?;

        let mut settings = TypeSpaceSettings::default();
        settings.with_derive("PartialEq".to_owned());
        let mut type_space = TypeSpace::new(&settings);
        type_space
            .add_root_schema(root)
            .with_context(|| format!("typify {}", document.module))?;
        let body = type_space.to_stream();
        items.extend(quote! {
            pub mod #module {
                #body
            }
        });
        if let Some(name) = document.embedded_constant {
            let constant: syn::Ident =
                syn::parse_str(name).with_context(|| format!("invalid constant name: {name}"))?;
            ensure!(names.insert(name), "duplicate Rust name: {name}");
            let schema = document.schema;
            items.extend(quote! {
                pub const #constant: &str = #schema;
            });
        }
    }

    let file: syn::File = syn::parse2(quote! {
        #![allow(dead_code, unused, clippy::all)]
        #items
    })
    .context("parse generated token stream")?;
    format_rust(&format!("{header}{}", prettyplease::unparse(&file)))
}

fn format_rust(source: &str) -> Result<String> {
    let mut formatter = Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("start rustfmt")?;
    let write_result = formatter
        .stdin
        .take()
        .context("rustfmt stdin missing")?
        .write_all(source.as_bytes());
    let output = formatter
        .wait_with_output()
        .context("format generated Rust")?;
    ensure!(
        output.status.success(),
        "rustfmt failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    write_result.context("write rustfmt input")?;
    String::from_utf8(output.stdout).context("rustfmt output is not UTF-8")
}

fn validate(root: &mut RootSchema) -> Result<()> {
    let definitions = root.definitions.keys().cloned().collect();
    let mut visitor = SchemaValidator {
        definitions,
        errors: Vec::new(),
        at_root: true,
    };
    visitor.visit_root_schema(root);
    if !visitor.errors.is_empty() {
        bail!("{}", visitor.errors.join("; "));
    }
    Ok(())
}

struct SchemaValidator {
    definitions: BTreeSet<String>,
    errors: Vec<String>,
    at_root: bool,
}

impl Visitor for SchemaValidator {
    fn visit_schema_object(&mut self, schema: &mut SchemaObject) {
        // visit_root_schema visits the document root first, then descendants
        // and definitions. Nested IDs change reference scope, which typify
        // cannot resolve against its single root definitions map.
        let at_root = std::mem::replace(&mut self.at_root, false);
        if !at_root && schema.metadata.as_ref().is_some_and(|m| m.id.is_some()) {
            self.errors
                .push("unsupported nested schema keyword: $id".to_owned());
        }
        // Inspect only schema keywords. Defaults, examples, enums and other
        // literal JSON payloads must never be treated as schema documents.
        for keyword in schema.extensions.keys() {
            if keyword == "prefixItems"
                || keyword.starts_with("dependent")
                || keyword.starts_with("unevaluated")
                || matches!(
                    keyword.as_str(),
                    "dependencies"
                        | "$dynamicRef"
                        | "$dynamicAnchor"
                        | "$recursiveRef"
                        | "$recursiveAnchor"
                        | "$anchor"
                        | "minContains"
                        | "maxContains"
                        | "$defs"
                        | "definitions"
                )
            {
                self.errors
                    .push(format!("unsupported schema keyword: {keyword}"));
            }
        }
        if let Some(reference) = &schema.reference
            && let Err(error) = validate_reference(reference, &self.definitions)
        {
            self.errors.push(error.to_string());
        }
        visit_schema_object(self, schema);
    }
}

fn validate_reference(reference: &str, definitions: &BTreeSet<String>) -> Result<()> {
    if reference == "#" {
        return Ok(());
    }
    let segment = reference
        .strip_prefix("#/$defs/")
        .or_else(|| reference.strip_prefix("#/definitions/"))
        .with_context(|| format!("unsupported schema reference: {reference}"))?;
    ensure!(
        !segment.contains('/'),
        "unsupported nested schema reference: {reference}"
    );
    let mut chars = segment.chars();
    while let Some(character) = chars.next() {
        if character == '~' {
            ensure!(
                matches!(chars.next(), Some('0' | '1')),
                "invalid JSON pointer escape in schema reference: {reference}"
            );
        }
    }
    let name = segment.replace("~1", "/").replace("~0", "~");
    ensure!(
        definitions.contains(&name),
        "unresolved schema reference: {reference}"
    );
    Ok(())
}
