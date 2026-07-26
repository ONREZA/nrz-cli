use std::borrow::Cow;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    AssignmentExpression, AssignmentTarget, AssignmentTargetProperty, BindingPattern,
    CallExpression, ComputedMemberExpression, ExportAllDeclaration, ExportNamedDeclaration,
    Expression, ImportDeclaration, ImportExpression, NewExpression, PropertyKey,
    StaticMemberExpression, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;

/// A denied runtime capability detected statically in a single module.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanCapability {
    BunAmbient,
    Worker,
    ParentMessageChannel,
    ProcessControl,
    CommonJsExports,
    CommonJsRequire,
}

/// Static analysis result for one module: discovered import specifiers, whether
/// a computed (non-literal) dynamic import was used, and the denied capabilities
/// present anywhere in the module.
pub(crate) struct ModuleScan {
    pub imports: Vec<String>,
    pub computed_dynamic_import: bool,
    pub capabilities: Vec<ScanCapability>,
    pub parse_failed: bool,
}

pub(crate) fn scan_module(
    path: &str,
    source: &str,
    allowed_bun_properties: &[String],
) -> ModuleScan {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts());
    let parsed = Parser::new(&allocator, source, source_type).parse();

    if parsed.panicked {
        return ModuleScan {
            imports: Vec::new(),
            computed_dynamic_import: false,
            capabilities: Vec::new(),
            parse_failed: true,
        };
    }

    let mut visitor = Visitor::new(allowed_bun_properties);
    visitor.visit_program(&parsed.program);
    ModuleScan {
        imports: visitor.imports,
        computed_dynamic_import: visitor.computed_dynamic_import,
        capabilities: visitor.capabilities.into_vec(),
        parse_failed: false,
    }
}

struct Visitor<'p> {
    allowed_bun_properties: &'p [String],
    imports: Vec<String>,
    computed_dynamic_import: bool,
    capabilities: CapabilitySet,
}

impl<'p> Visitor<'p> {
    fn new(allowed_bun_properties: &'p [String]) -> Self {
        Self {
            allowed_bun_properties,
            imports: Vec::new(),
            computed_dynamic_import: false,
            capabilities: CapabilitySet::default(),
        }
    }
}

impl<'a> Visit<'a> for Visitor<'_> {
    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        self.imports.push(it.source.value.to_string());
        walk::walk_import_declaration(self, it);
    }

    fn visit_export_named_declaration(&mut self, it: &ExportNamedDeclaration<'a>) {
        if let Some(source) = &it.source {
            self.imports.push(source.value.to_string());
        }
        walk::walk_export_named_declaration(self, it);
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

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::Identifier(callee) = &it.callee {
            match callee.name.as_str() {
                "require" => self.capabilities.insert(ScanCapability::CommonJsRequire),
                "Worker" => self.capabilities.insert(ScanCapability::Worker),
                "postMessage" => self
                    .capabilities
                    .insert(ScanCapability::ParentMessageChannel),
                _ => {}
            }
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        if let Expression::Identifier(callee) = &it.callee
            && callee.name == "Worker"
        {
            self.capabilities.insert(ScanCapability::Worker);
        }
        walk::walk_new_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(init) = &it.init {
            if is_bun_ambient(init)
                && binding_pattern_exposes_denied_bun_property(&it.id, self.allowed_bun_properties)
            {
                self.capabilities.insert(ScanCapability::BunAmbient);
            }

            if is_process_ambient(init) && binding_pattern_exposes_process_control(&it.id) {
                self.capabilities.insert(ScanCapability::ProcessControl);
            }
        }
        walk::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if is_bun_ambient(&it.right)
            && assignment_target_exposes_denied_bun_property(&it.left, self.allowed_bun_properties)
        {
            self.capabilities.insert(ScanCapability::BunAmbient);
        }

        if is_process_ambient(&it.right) && assignment_target_exposes_process_control(&it.left) {
            self.capabilities.insert(ScanCapability::ProcessControl);
        }
        walk::walk_assignment_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if static_member_is_parent_message_channel(it) {
            self.capabilities
                .insert(ScanCapability::ParentMessageChannel);
        }

        let property = it.property.name.as_str();
        if is_bun_ambient(&it.object)
            && is_denied_bun_property(property, self.allowed_bun_properties)
        {
            self.capabilities.insert(ScanCapability::BunAmbient);
        }

        if is_process_ambient(&it.object) && is_process_control_property(property) {
            self.capabilities.insert(ScanCapability::ProcessControl);
        }

        if let Expression::Identifier(object) = &it.object {
            match object.name.as_str() {
                "module" if property == "exports" => {
                    self.capabilities.insert(ScanCapability::CommonJsExports);
                }
                "exports" => {
                    self.capabilities.insert(ScanCapability::CommonJsExports);
                }
                _ => {}
            }
        }
        walk::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if computed_member_is_parent_message_channel(it) {
            self.capabilities
                .insert(ScanCapability::ParentMessageChannel);
        }

        if is_bun_ambient(&it.object)
            && computed_bun_property_may_be_denied(&it.expression, self.allowed_bun_properties)
        {
            self.capabilities.insert(ScanCapability::BunAmbient);
        }

        if is_process_ambient(&it.object)
            && computed_process_property_may_be_control(&it.expression)
        {
            self.capabilities.insert(ScanCapability::ProcessControl);
        }
        walk::walk_computed_member_expression(self, it);
    }
}

fn static_member_is_parent_message_channel(expression: &StaticMemberExpression<'_>) -> bool {
    expression.property.name == "postMessage" && is_parent_message_global(&expression.object)
}

fn computed_member_is_parent_message_channel(expression: &ComputedMemberExpression<'_>) -> bool {
    is_parent_message_global(&expression.object)
        && static_expression_name(&expression.expression)
            .map(|property| property == "postMessage")
            .unwrap_or(false)
}

fn is_parent_message_global(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::Identifier(identifier) => {
            matches!(identifier.name.as_str(), "globalThis" | "self")
        }
        Expression::ParenthesizedExpression(expression) => {
            is_parent_message_global(&expression.expression)
        }
        Expression::TSAsExpression(expression) => is_parent_message_global(&expression.expression),
        Expression::TSSatisfiesExpression(expression) => {
            is_parent_message_global(&expression.expression)
        }
        Expression::TSTypeAssertion(expression) => is_parent_message_global(&expression.expression),
        Expression::TSNonNullExpression(expression) => {
            is_parent_message_global(&expression.expression)
        }
        _ => false,
    }
}

fn is_allowed_bun_property(property: &str, allowed_bun_properties: &[String]) -> bool {
    allowed_bun_properties
        .iter()
        .any(|allowed| allowed == property)
}

fn is_denied_bun_property(property: &str, allowed_bun_properties: &[String]) -> bool {
    !is_allowed_bun_property(property, allowed_bun_properties)
}

fn is_bun_ambient(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::Identifier(identifier) => identifier.name == "Bun",
        Expression::StaticMemberExpression(expression) => {
            is_global_object(&expression.object) && expression.property.name == "Bun"
        }
        Expression::ComputedMemberExpression(expression) => {
            is_global_object(&expression.object)
                && static_expression_name(&expression.expression)
                    .map(|property| property == "Bun")
                    .unwrap_or(false)
        }
        Expression::ParenthesizedExpression(expression) => is_bun_ambient(&expression.expression),
        Expression::TSAsExpression(expression) => is_bun_ambient(&expression.expression),
        Expression::TSSatisfiesExpression(expression) => is_bun_ambient(&expression.expression),
        Expression::TSTypeAssertion(expression) => is_bun_ambient(&expression.expression),
        Expression::TSNonNullExpression(expression) => is_bun_ambient(&expression.expression),
        _ => false,
    }
}

fn is_process_ambient(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::Identifier(identifier) => identifier.name == "process",
        Expression::StaticMemberExpression(expression) => {
            is_global_object(&expression.object) && expression.property.name == "process"
        }
        Expression::ComputedMemberExpression(expression) => {
            is_global_object(&expression.object)
                && static_expression_name(&expression.expression)
                    .map(|property| property == "process")
                    .unwrap_or(false)
        }
        Expression::ParenthesizedExpression(expression) => {
            is_process_ambient(&expression.expression)
        }
        Expression::TSAsExpression(expression) => is_process_ambient(&expression.expression),
        Expression::TSSatisfiesExpression(expression) => is_process_ambient(&expression.expression),
        Expression::TSTypeAssertion(expression) => is_process_ambient(&expression.expression),
        Expression::TSNonNullExpression(expression) => is_process_ambient(&expression.expression),
        _ => false,
    }
}

fn is_global_object(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::Identifier(identifier) => {
            matches!(identifier.name.as_str(), "globalThis" | "self")
        }
        Expression::ParenthesizedExpression(expression) => is_global_object(&expression.expression),
        Expression::TSAsExpression(expression) => is_global_object(&expression.expression),
        Expression::TSSatisfiesExpression(expression) => is_global_object(&expression.expression),
        Expression::TSTypeAssertion(expression) => is_global_object(&expression.expression),
        Expression::TSNonNullExpression(expression) => is_global_object(&expression.expression),
        _ => false,
    }
}

fn computed_bun_property_may_be_denied(
    expression: &Expression<'_>,
    allowed_bun_properties: &[String],
) -> bool {
    static_expression_name(expression)
        .map(|property| is_denied_bun_property(property.as_ref(), allowed_bun_properties))
        .unwrap_or(true)
}

fn computed_process_property_may_be_control(expression: &Expression<'_>) -> bool {
    static_expression_name(expression)
        .map(|property| is_process_control_property(property.as_ref()))
        .unwrap_or(true)
}

fn is_process_control_property(property: &str) -> bool {
    property == "exit" || property == "kill"
}

fn static_expression_name<'a>(expression: &Expression<'a>) -> Option<Cow<'a, str>> {
    match expression {
        Expression::StringLiteral(literal) => Some(Cow::Borrowed(literal.value.as_str())),
        Expression::TemplateLiteral(literal) => literal
            .single_quasi()
            .map(|quasi| Cow::Owned(quasi.to_string())),
        Expression::ParenthesizedExpression(expression) => {
            static_expression_name(&expression.expression)
        }
        Expression::TSAsExpression(expression) => static_expression_name(&expression.expression),
        Expression::TSSatisfiesExpression(expression) => {
            static_expression_name(&expression.expression)
        }
        Expression::TSTypeAssertion(expression) => static_expression_name(&expression.expression),
        Expression::TSNonNullExpression(expression) => {
            static_expression_name(&expression.expression)
        }
        _ => None,
    }
}

fn binding_pattern_exposes_denied_bun_property(
    pattern: &BindingPattern<'_>,
    allowed_bun_properties: &[String],
) -> bool {
    match pattern {
        BindingPattern::ObjectPattern(pattern) => {
            pattern.rest.is_some()
                || pattern.properties.iter().any(|property| {
                    property_key_name(&property.key)
                        .map(|name| is_denied_bun_property(name.as_ref(), allowed_bun_properties))
                        .unwrap_or(true)
                })
        }
        BindingPattern::AssignmentPattern(pattern) => {
            binding_pattern_exposes_denied_bun_property(&pattern.left, allowed_bun_properties)
        }
        BindingPattern::ArrayPattern(_) => false,
        BindingPattern::BindingIdentifier(_) => true,
    }
}

fn binding_pattern_exposes_process_control(pattern: &BindingPattern<'_>) -> bool {
    match pattern {
        BindingPattern::ObjectPattern(pattern) => {
            pattern.rest.is_some()
                || pattern.properties.iter().any(|property| {
                    property_key_name(&property.key)
                        .map(|name| is_process_control_property(name.as_ref()))
                        .unwrap_or(true)
                })
        }
        BindingPattern::AssignmentPattern(pattern) => {
            binding_pattern_exposes_process_control(&pattern.left)
        }
        BindingPattern::ArrayPattern(_) => false,
        BindingPattern::BindingIdentifier(_) => true,
    }
}

fn assignment_target_exposes_denied_bun_property(
    target: &AssignmentTarget<'_>,
    allowed_bun_properties: &[String],
) -> bool {
    match target {
        AssignmentTarget::ObjectAssignmentTarget(target) => {
            target.rest.is_some()
                || target.properties.iter().any(|property| {
                    assignment_target_property_name(property)
                        .map(|name| is_denied_bun_property(name.as_ref(), allowed_bun_properties))
                        .unwrap_or(true)
                })
        }
        AssignmentTarget::ArrayAssignmentTarget(_) => false,
        _ => true,
    }
}

fn assignment_target_exposes_process_control(target: &AssignmentTarget<'_>) -> bool {
    match target {
        AssignmentTarget::ObjectAssignmentTarget(target) => {
            target.rest.is_some()
                || target.properties.iter().any(|property| {
                    assignment_target_property_name(property)
                        .map(|name| is_process_control_property(name.as_ref()))
                        .unwrap_or(true)
                })
        }
        AssignmentTarget::ArrayAssignmentTarget(_) => false,
        _ => true,
    }
}

fn assignment_target_property_name<'a>(
    property: &AssignmentTargetProperty<'a>,
) -> Option<Cow<'a, str>> {
    match property {
        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(property) => {
            Some(Cow::Borrowed(property.binding.name.as_str()))
        }
        AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) => {
            property_key_name(&property.name)
        }
    }
}

fn property_key_name<'a>(key: &PropertyKey<'a>) -> Option<Cow<'a, str>> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(Cow::Borrowed(identifier.name.as_str())),
        PropertyKey::StringLiteral(literal) => Some(Cow::Borrowed(literal.value.as_str())),
        PropertyKey::TemplateLiteral(literal) => literal
            .single_quasi()
            .map(|quasi| Cow::Owned(quasi.to_string())),
        _ => None,
    }
}

/// Deduplicates capabilities so each denied capability is reported at most once
/// per module, preserving a stable emission order.
#[derive(Default)]
struct CapabilitySet {
    bun_ambient: bool,
    worker: bool,
    parent_message_channel: bool,
    process_control: bool,
    common_js_exports: bool,
    common_js_require: bool,
}

impl CapabilitySet {
    fn insert(&mut self, capability: ScanCapability) {
        match capability {
            ScanCapability::BunAmbient => self.bun_ambient = true,
            ScanCapability::Worker => self.worker = true,
            ScanCapability::ParentMessageChannel => self.parent_message_channel = true,
            ScanCapability::ProcessControl => self.process_control = true,
            ScanCapability::CommonJsExports => self.common_js_exports = true,
            ScanCapability::CommonJsRequire => self.common_js_require = true,
        }
    }

    fn into_vec(self) -> Vec<ScanCapability> {
        let mut capabilities = Vec::new();
        if self.bun_ambient {
            capabilities.push(ScanCapability::BunAmbient);
        }
        if self.worker {
            capabilities.push(ScanCapability::Worker);
        }
        if self.parent_message_channel {
            capabilities.push(ScanCapability::ParentMessageChannel);
        }
        if self.process_control {
            capabilities.push(ScanCapability::ProcessControl);
        }
        if self.common_js_exports {
            capabilities.push(ScanCapability::CommonJsExports);
        }
        if self.common_js_require {
            capabilities.push(ScanCapability::CommonJsRequire);
        }
        capabilities
    }
}
