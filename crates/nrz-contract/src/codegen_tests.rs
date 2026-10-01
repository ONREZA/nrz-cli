use proc_macro2::{Group, TokenStream, TokenTree};
use quote::ToTokens as _;
use serde_json::json;

use crate::codegen::{SchemaDocument, generate};

fn generated(schema: &str, embedded_constant: Option<&str>) -> anyhow::Result<String> {
    generate(
        &[SchemaDocument {
            module: "contract",
            schema,
            embedded_constant,
        }],
        "// Test contract.\n",
    )
}

// Generated documentation includes the input schema. Compare parsed Rust
// tokens without doc attributes so dialect spelling cannot mask type drift.
fn without_docs(tokens: TokenStream) -> TokenStream {
    let mut tokens = tokens.into_iter().peekable();
    let mut result = TokenStream::new();
    while let Some(token) = tokens.next() {
        if matches!(&token, TokenTree::Punct(p) if p.as_char() == '#')
            && let Some(TokenTree::Group(group)) = tokens.peek()
            && group.delimiter() == proc_macro2::Delimiter::Bracket
            && matches!(group.stream().into_iter().next(), Some(TokenTree::Ident(i)) if i == "doc")
        {
            tokens.next();
            continue;
        }
        result.extend([match token {
            TokenTree::Group(group) => {
                TokenTree::Group(Group::new(group.delimiter(), without_docs(group.stream())))
            }
            token => token,
        }]);
    }
    result
}

fn rust_structure(source: &str) -> String {
    let file: syn::File = syn::parse_str(source).unwrap();
    without_docs(file.to_token_stream()).to_string()
}

fn embedded_schema(source: &str) -> String {
    let file: syn::File = syn::parse_str(source).unwrap();
    let constant = file
        .items
        .into_iter()
        .find_map(|item| match item {
            syn::Item::Const(item) if item.ident == "SCHEMA_JSON" => Some(item),
            _ => None,
        })
        .expect("embedded schema constant");
    let syn::Expr::Lit(expression) = *constant.expr else {
        panic!("schema constant must be a string literal");
    };
    let syn::Lit::Str(value) = expression.lit else {
        panic!("schema constant must contain a string");
    };
    value.value()
}

#[test]
fn direct_defs_preserve_named_reference_output() {
    let modern = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "Envelope",
        "type": "object",
        "required": ["primary", "secondary"],
        "properties": {
            "primary": {"$ref": "#/$defs/Shared~1Thing~0"},
            "secondary": {"$ref": "#/$defs/Shared~1Thing~0"}
        },
        "$defs": {
            "Shared/Thing~": {
                "type": "object",
                "required": ["label"],
                "properties": {"label": {"type": "string"}},
                "additionalProperties": false
            }
        },
        "additionalProperties": false
    });
    let legacy = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Envelope",
        "type": "object",
        "required": ["primary", "secondary"],
        "properties": {
            "primary": {"$ref": "#/definitions/Shared~1Thing~0"},
            "secondary": {"$ref": "#/definitions/Shared~1Thing~0"}
        },
        "definitions": modern["$defs"],
        "additionalProperties": false
    });
    let modern_output = generated(&modern.to_string(), None).unwrap();
    let legacy_output = generated(&legacy.to_string(), None).unwrap();
    assert_eq!(
        rust_structure(&modern_output),
        rust_structure(&legacy_output)
    );

    let file: syn::File = syn::parse_str(&modern_output).unwrap();
    let syn::Item::Mod(module) = &file.items[0] else {
        panic!("generated contract module");
    };
    let structs: Vec<_> = module
        .content
        .as_ref()
        .unwrap()
        .1
        .iter()
        .filter_map(|item| match item {
            syn::Item::Struct(item) => Some(item),
            _ => None,
        })
        .collect();
    assert_eq!(structs.len(), 2, "references must reuse one named type");
    let envelope = structs
        .iter()
        .find(|item| item.ident == "Envelope")
        .unwrap();
    let fields: Vec<_> = envelope.fields.iter().collect();
    assert_eq!(
        fields[0].ty.to_token_stream().to_string(),
        fields[1].ty.to_token_stream().to_string()
    );
}

#[test]
fn embedded_schema_preserves_exact_input() {
    let schema = r#"{
  "title": "Payload",
  "type": "object",
  "description": "Unicode: привет, escaped quote: \" and slash: \\"
}
"#;
    let first = generated(schema, Some("SCHEMA_JSON")).unwrap();
    assert_eq!(embedded_schema(&first), schema);
    assert_eq!(first, generated(schema, Some("SCHEMA_JSON")).unwrap());
}

#[test]
fn rejects_draft_2020_tuple_instead_of_dropping_prefix_items() {
    let schema = json!({
        "title": "Tuple",
        "type": "array",
        "prefixItems": [{"type": "string"}, {"type": "integer"}],
        "items": false,
        "minItems": 2,
        "maxItems": 2
    });
    let error = generated(&schema.to_string(), None).unwrap_err();
    assert!(format!("{error:#}").contains("unsupported schema keyword: prefixItems"));
}

#[test]
fn rejects_unsupported_keywords_in_nested_schema_positions() {
    for keyword in [
        "prefixItems",
        "dependentRequired",
        "dependentSchemas",
        "dependencies",
        "unevaluatedItems",
        "unevaluatedProperties",
        "$dynamicRef",
        "$dynamicAnchor",
        "$recursiveRef",
        "$recursiveAnchor",
        "$anchor",
        "minContains",
        "maxContains",
    ] {
        for position in ["property", "definition"] {
            let mut nested = json!({"type": "object"});
            nested[keyword] = json!(false);
            let schema = if position == "property" {
                json!({"title": "Root", "type": "object", "properties": {"nested": nested}})
            } else {
                json!({"title": "Root", "type": "object", "$defs": {"Nested": nested}})
            };
            let error = generated(&schema.to_string(), None).unwrap_err();
            let message = format!("{error:#}");
            assert!(message.contains("validate schema contract"), "{message}");
            assert!(
                message.contains(&format!("unsupported schema keyword: {keyword}")),
                "{position}: {message}"
            );
        }
    }
}

#[test]
fn validates_references_before_typify_resolution() {
    for (reference, expected) in [
        ("#/$defs/Missing", "unresolved schema reference"),
        ("#/definitions/Missing", "unresolved schema reference"),
        (
            "https://example.com/schema.json#/$defs/Present",
            "unsupported schema reference",
        ),
        ("#anchor", "unsupported schema reference"),
        ("#/properties/Present", "unsupported schema reference"),
        (
            "#/$defs/Present/properties/label",
            "unsupported nested schema reference",
        ),
        ("#/$defs/Present~2", "invalid JSON pointer escape"),
    ] {
        let schema = json!({
            "title": "Root",
            "type": "object",
            "properties": {"value": {"$ref": reference}},
            "$defs": {"Present": {"type": "string"}}
        });
        let error = generated(&schema.to_string(), None).unwrap_err();
        assert!(
            format!("{error:#}").contains(expected),
            "{reference}: {error:#}"
        );
    }
}

#[test]
fn root_id_is_supported_but_nested_ids_cannot_change_reference_scope() {
    let root = json!({
        "$id": "https://example.com/root.schema.json",
        "title": "Root",
        "type": "object",
        "properties": {"value": {"$ref": "#/$defs/Value"}},
        "$defs": {"Value": {"type": "string"}}
    });
    generated(&root.to_string(), None).unwrap();
    for position in ["property", "definition"] {
        let mut schema = root.clone();
        let nested = if position == "property" {
            &mut schema["properties"]["value"]
        } else {
            &mut schema["$defs"]["Value"]
        };
        nested["$id"] = json!("nested.schema.json");
        let error = generated(&schema.to_string(), None).unwrap_err();
        assert!(
            format!("{error:#}").contains("unsupported nested schema keyword: $id"),
            "{position}: {error:#}"
        );
    }
}

#[test]
fn schema_like_literal_payloads_and_property_names_are_not_keywords() {
    let schema = json!({
        "title": "Payload",
        "type": "object",
        "properties": {
            "prefixItems": {"type": "integer"},
            "$ref": {"type": "string"},
            "data": {
                "type": "object",
                "default": {"prefixItems": [], "$ref": "external", "$defs": {"Literal": {}}},
                "examples": [{"dependentSchemas": {}, "unevaluatedItems": false}]
            }
        },
        "examples": [{"prefixItems": [], "$dynamicRef": "literal"}]
    });
    let input = serde_json::to_string_pretty(&schema).unwrap();
    let source = generated(&input, Some("SCHEMA_JSON")).unwrap();
    assert_eq!(embedded_schema(&source), input);
    syn::parse_str::<syn::File>(&source).unwrap();
}
