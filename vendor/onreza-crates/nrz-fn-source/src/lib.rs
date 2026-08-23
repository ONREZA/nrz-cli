use std::error::Error;
use std::fmt;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ArrayExpression, ArrayExpressionElement, BindingPattern, Declaration, ExportAllDeclaration,
    ExportFromDeclaration, Expression, ImportDeclaration, ImportExpression, ObjectExpression,
    ObjectPropertyKind, PropertyKey, PropertyKind, Statement, VariableDeclaration,
    VariableDeclarationKind,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

const CONFIG_EXPORT_NAME: &str = "config";

pub const FUNCTION_ENTRY_SUFFIXES: &[&str] = &[
    ".nrz-fn.ts",
    ".nrz-fn.tsx",
    ".nrz-fn.js",
    ".nrz-fn.jsx",
    ".nrz-fn.mjs",
];
pub const FUNCTION_SOURCE_EXTENSIONS: &[&str] = &[".ts", ".tsx", ".js", ".jsx", ".mjs"];
pub const FUNCTION_SOURCE_DENIED_PATH_SEGMENT: &str = "node_modules";
pub const MAX_FUNCTIONS_PER_PUBLISH: usize = 1000;
pub const MAX_FUNCTION_SOURCE_FILE_BYTES: u64 = 128 * 1024;
pub const MAX_FUNCTION_SOURCE_FILES_PER_FUNCTION: usize = 1;
pub const MAX_FUNCTION_NAME_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionEntryAnalysis {
    pub declaration: FunctionConfigDeclaration,
    pub imports: Vec<String>,
    pub computed_dynamic_import: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FunctionConfigDeclaration {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub triggers: Vec<DeclaredFunctionTrigger>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeclaredFunctionTrigger {
    pub name: String,
    #[serde(rename = "type")]
    pub trigger_type: String,
    #[serde(default)]
    pub matchers: Vec<String>,
    #[serde(default)]
    pub methods: Option<Vec<String>>,
    #[serde(default, alias = "on_failure")]
    pub on_failure: Option<String>,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub config: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionConfigError {
    message: String,
}

impl FunctionConfigError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for FunctionConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for FunctionConfigError {}

/// Parse an ONREZA Function entry file without executing user code and extract
/// the serializable `export const config = { ... }` declaration plus imports.
pub fn analyze_function_entry(
    path: &str,
    source: &str,
) -> Result<FunctionEntryAnalysis, FunctionConfigError> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts());
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if parsed.panicked || !parsed.diagnostics.is_empty() {
        return Err(FunctionConfigError::new(
            "function entry could not be parsed as ESM TypeScript/JavaScript",
        ));
    }

    let Some(config_value) = find_exported_config(&parsed.program.body)? else {
        return Err(FunctionConfigError::new(
            "function entry must declare `export const config = { ... }` before executable code",
        ));
    };
    let declaration = parse_config_declaration(config_value)?;

    let mut visitor = ImportVisitor::default();
    visitor.visit_program(&parsed.program);

    Ok(FunctionEntryAnalysis {
        declaration,
        imports: visitor.imports,
        computed_dynamic_import: visitor.computed_dynamic_import,
    })
}

fn find_exported_config<'a>(
    statements: &'a oxc_allocator::Vec<'a, Statement<'a>>,
) -> Result<Option<&'a Expression<'a>>, FunctionConfigError> {
    for statement in statements {
        match statement {
            Statement::EmptyStatement(_) => continue,
            Statement::TSTypeAliasDeclaration(_)
            | Statement::TSInterfaceDeclaration(_)
            | Statement::TSEnumDeclaration(_)
            | Statement::TSExternalModuleDeclaration(_)
            | Statement::TSNamespaceDeclaration(_)
            | Statement::TSGlobalDeclaration(_) => continue,
            Statement::ExportDeclaration(export) => {
                if let Declaration::VariableDeclaration(declaration) = &export.declaration
                    && let Some(config) = config_declarator(declaration)?
                {
                    return Ok(Some(config));
                }
                return Err(FunctionConfigError::new(
                    "`export const config` must be the first runtime declaration in a function entry",
                ));
            }
            Statement::ExportFromDeclaration(_) | Statement::ExportAllDeclaration(_) => {
                return Err(FunctionConfigError::new(
                    "ONREZA Functions v1 does not support re-export imports",
                ));
            }
            Statement::ImportDeclaration(_) => {
                return Err(FunctionConfigError::new(
                    "ONREZA Functions v1 does not support user imports",
                ));
            }
            _ => {
                return Err(FunctionConfigError::new(
                    "function entry must declare `export const config = { ... }` before executable code",
                ));
            }
        }
    }
    Ok(None)
}

fn config_declarator<'a>(
    declaration: &'a VariableDeclaration<'a>,
) -> Result<Option<&'a Expression<'a>>, FunctionConfigError> {
    if declaration.kind != VariableDeclarationKind::Const {
        return Ok(None);
    }
    for declarator in &declaration.declarations {
        let BindingPattern::BindingIdentifier(identifier) = &declarator.id else {
            continue;
        };
        if identifier.name != CONFIG_EXPORT_NAME {
            continue;
        }
        let Some(init) = &declarator.init else {
            return Err(FunctionConfigError::new(
                "`export const config` must initialize a literal object",
            ));
        };
        return Ok(Some(init));
    }
    Ok(None)
}

fn parse_config_declaration(
    expression: &Expression<'_>,
) -> Result<FunctionConfigDeclaration, FunctionConfigError> {
    let value = expression_to_json(unwrap_ts_expression(expression))?;
    let declaration: FunctionConfigDeclaration =
        serde_json::from_value(value).map_err(|error| {
            FunctionConfigError::new(format!("invalid function config declaration: {error}"))
        })?;
    validate_config_declaration(declaration)
}

fn validate_config_declaration(
    mut declaration: FunctionConfigDeclaration,
) -> Result<FunctionConfigDeclaration, FunctionConfigError> {
    if let Some(name) = &declaration.name {
        validate_function_name(name)?;
    }
    for trigger in &mut declaration.triggers {
        if trigger.name.trim().is_empty() {
            return Err(FunctionConfigError::new(
                "function trigger name must not be empty",
            ));
        }
        match trigger.trigger_type.as_str() {
            "scheduled" | "queue" | "manual" => {}
            "http" | "middleware" => {
                return Err(FunctionConfigError::new(
                    "HTTP route wiring must be declared as an EdgeRuleSet pipeline action in onreza.rules.toml, not as a function trigger",
                ));
            }
            other => {
                return Err(FunctionConfigError::new(format!(
                    "unsupported function trigger type: {other}"
                )));
            }
        }
        if !trigger.matchers.is_empty()
            || trigger.methods.is_some()
            || trigger.on_failure.is_some()
            || trigger.priority.is_some()
        {
            return Err(FunctionConfigError::new(
                "function trigger routing fields are only valid in EdgeRuleSet pipeline actions",
            ));
        }
    }
    Ok(declaration)
}

pub fn is_function_entry_path(path: &str) -> bool {
    FUNCTION_ENTRY_SUFFIXES
        .iter()
        .any(|suffix| path.ends_with(suffix))
}

pub fn is_supported_function_source_path(path: &str) -> bool {
    !path
        .split(['/', '\\'])
        .any(|segment| segment == FUNCTION_SOURCE_DENIED_PATH_SEGMENT)
        && FUNCTION_SOURCE_EXTENSIONS
            .iter()
            .any(|extension| path.ends_with(extension))
}

pub fn function_name_from_entrypoint(entrypoint: &str) -> Result<&str, FunctionConfigError> {
    let file_name = entrypoint.rsplit(['/', '\\']).next().unwrap_or(entrypoint);
    let Some(name) = FUNCTION_ENTRY_SUFFIXES
        .iter()
        .find_map(|suffix| file_name.strip_suffix(suffix))
    else {
        return Err(FunctionConfigError::new(
            "function entry must use *.nrz-fn.ts/js/mjs suffix",
        ));
    };
    validate_function_name(name)?;
    Ok(name)
}

pub fn validate_function_name(name: &str) -> Result<(), FunctionConfigError> {
    if name.is_empty() || name.len() > MAX_FUNCTION_NAME_LENGTH {
        return Err(FunctionConfigError::new(format!(
            "function name must be 1..={MAX_FUNCTION_NAME_LENGTH} characters"
        )));
    }
    let valid = name
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-');
    if !valid {
        return Err(FunctionConfigError::new(
            "function name must use lowercase letters, digits, and '-'",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{analyze_function_entry, function_name_from_entrypoint};

    #[test]
    fn rejects_http_route_trigger_declarations() {
        let error = analyze_function_entry(
            "api.nrz-fn.ts",
            r#"
export const config = { triggers: [{ name: "api", type: "http" }] } as const;
export default {};
"#,
        )
        .unwrap_err();

        assert!(error.message().contains("EdgeRuleSet pipeline"));
    }

    #[test]
    fn derives_and_validates_function_name_from_entrypoint() {
        assert_eq!(
            function_name_from_entrypoint("functions/billing-webhook.nrz-fn.ts").unwrap(),
            "billing-webhook"
        );
        assert!(function_name_from_entrypoint("functions/Billing.nrz-fn.ts").is_err());
        assert!(function_name_from_entrypoint("functions/api.ts").is_err());
    }
}

fn unwrap_ts_expression<'a>(expression: &'a Expression<'a>) -> &'a Expression<'a> {
    match expression {
        Expression::ParenthesizedExpression(inner) => unwrap_ts_expression(&inner.expression),
        Expression::TSAsExpression(inner) => unwrap_ts_expression(&inner.expression),
        Expression::TSSatisfiesExpression(inner) => unwrap_ts_expression(&inner.expression),
        Expression::TSNonNullExpression(inner) => unwrap_ts_expression(&inner.expression),
        Expression::TSInstantiationExpression(inner) => unwrap_ts_expression(&inner.expression),
        _ => expression,
    }
}

fn expression_to_json(expression: &Expression<'_>) -> Result<Value, FunctionConfigError> {
    match unwrap_ts_expression(expression) {
        Expression::StringLiteral(literal) => Ok(Value::String(literal.value.to_string())),
        Expression::BooleanLiteral(literal) => Ok(Value::Bool(literal.value)),
        Expression::NullLiteral(_) => Ok(Value::Null),
        Expression::NumericLiteral(literal) => number_to_json(literal.value),
        Expression::ArrayExpression(array) => array_to_json(array),
        Expression::ObjectExpression(object) => object_to_json(object),
        _ => Err(FunctionConfigError::new(
            "function config supports literal JSON-compatible values only",
        )),
    }
}

fn number_to_json(value: f64) -> Result<Value, FunctionConfigError> {
    if value.is_finite()
        && value.fract() == 0.0
        && value >= i64::MIN as f64
        && value <= i64::MAX as f64
    {
        return Ok(Value::Number(Number::from(value as i64)));
    }
    let Some(number) = Number::from_f64(value) else {
        return Err(FunctionConfigError::new(
            "function config numeric values must be finite",
        ));
    };
    Ok(Value::Number(number))
}

fn array_to_json(array: &ArrayExpression<'_>) -> Result<Value, FunctionConfigError> {
    let mut values = Vec::with_capacity(array.elements.len());
    for element in &array.elements {
        match element {
            ArrayExpressionElement::SpreadElement(_) | ArrayExpressionElement::Elision(_) => {
                return Err(FunctionConfigError::new(
                    "function config arrays must not use spread elements or holes",
                ));
            }
            _ => values.push(array_element_to_json(element)?),
        }
    }
    Ok(Value::Array(values))
}

fn array_element_to_json(
    element: &ArrayExpressionElement<'_>,
) -> Result<Value, FunctionConfigError> {
    match element {
        ArrayExpressionElement::StringLiteral(value) => Ok(Value::String(value.value.to_string())),
        ArrayExpressionElement::BooleanLiteral(value) => Ok(Value::Bool(value.value)),
        ArrayExpressionElement::NullLiteral(_) => Ok(Value::Null),
        ArrayExpressionElement::NumericLiteral(value) => number_to_json(value.value),
        ArrayExpressionElement::ArrayExpression(value) => array_to_json(value),
        ArrayExpressionElement::ObjectExpression(value) => object_to_json(value),
        ArrayExpressionElement::ParenthesizedExpression(value) => {
            expression_to_json(&value.expression)
        }
        ArrayExpressionElement::TSAsExpression(value) => expression_to_json(&value.expression),
        ArrayExpressionElement::TSSatisfiesExpression(value) => {
            expression_to_json(&value.expression)
        }
        ArrayExpressionElement::TSNonNullExpression(value) => expression_to_json(&value.expression),
        ArrayExpressionElement::TSInstantiationExpression(value) => {
            expression_to_json(&value.expression)
        }
        _ => Err(FunctionConfigError::new(
            "function config arrays support literal JSON-compatible values only",
        )),
    }
}

fn object_to_json(object: &ObjectExpression<'_>) -> Result<Value, FunctionConfigError> {
    let mut map = Map::new();
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return Err(FunctionConfigError::new(
                "function config objects must not use spread properties",
            ));
        };
        if property.kind != PropertyKind::Init || property.method || property.computed {
            return Err(FunctionConfigError::new(
                "function config objects support plain literal properties only",
            ));
        }
        let key = property_key_to_string(&property.key)?;
        let value = expression_to_json(&property.value)?;
        if map.insert(key.clone(), value).is_some() {
            return Err(FunctionConfigError::new(format!(
                "duplicate function config key: {key}"
            )));
        }
    }
    Ok(Value::Object(map))
}

fn property_key_to_string(key: &PropertyKey<'_>) -> Result<String, FunctionConfigError> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Ok(identifier.name.to_string()),
        PropertyKey::StringLiteral(literal) => Ok(literal.value.to_string()),
        PropertyKey::NumericLiteral(literal) => Ok(literal.value.to_string()),
        _ => Err(FunctionConfigError::new(
            "function config object keys must be static identifiers or string literals",
        )),
    }
}

#[derive(Default)]
struct ImportVisitor {
    imports: Vec<String>,
    computed_dynamic_import: bool,
}

impl<'a> Visit<'a> for ImportVisitor {
    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        self.imports.push(it.source.value.to_string());
        walk::walk_import_declaration(self, it);
    }

    fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
        self.imports.push(it.source.value.to_string());
        walk::walk_export_from_declaration(self, it);
    }

    fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
        self.imports.push(it.source.value.to_string());
        walk::walk_export_all_declaration(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        match &it.source {
            Expression::StringLiteral(literal) => self.imports.push(literal.value.to_string()),
            _ => self.computed_dynamic_import = true,
        }
        walk::walk_import_expression(self, it);
    }
}
