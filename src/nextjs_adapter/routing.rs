use super::*;

impl AdapterRoutingCounts {
    pub(super) fn has_effects(&self) -> bool {
        self.redirects > 0 || self.rewrites > 0 || self.header_rules > 0
    }
}

pub(super) fn routing_platform_status(
    routing: &AdapterRoutingCounts,
    edge_rule_lowering: &AdapterEdgeRuleLoweringCounts,
) -> &'static str {
    if !routing.has_effects() {
        return "absent";
    }
    if edge_rule_lowering.generated == 0 {
        return "pending_edge_rules";
    }
    if edge_rule_lowering.unsupported == 0 {
        return "edge_rules_generated";
    }
    "partial_edge_rules"
}

impl AdapterRouting {
    pub(super) fn counts(&self) -> AdapterRoutingCounts {
        let routes = self.routes().collect::<Vec<_>>();
        AdapterRoutingCounts {
            before_middleware: self.before_middleware.len(),
            before_files: self.before_files.len(),
            after_files: self.after_files.len(),
            dynamic_routes: self.dynamic_routes.len(),
            on_match: self.on_match.len(),
            fallback: self.fallback.len(),
            redirects: routes.iter().filter(|route| route.is_redirect()).count(),
            rewrites: routes.iter().filter(|route| route.is_rewrite()).count(),
            header_rules: routes
                .iter()
                .filter(|route| route.has_headers() && !route.is_redirect())
                .count(),
            priority_rules: routes.iter().filter(|route| route.is_priority()).count(),
            source_rules: routes.iter().filter(|route| route.source.is_some()).count(),
            source_regex_rules: routes
                .iter()
                .filter(|route| route.source_regex.is_some())
                .count(),
        }
    }

    pub(super) fn routes(&self) -> impl Iterator<Item = &AdapterRoute> {
        self.before_middleware
            .iter()
            .chain(self.before_files.iter())
            .chain(self.after_files.iter())
            .chain(self.dynamic_routes.iter())
            .chain(self.on_match.iter())
            .chain(self.fallback.iter())
    }

    pub(super) fn has_exact_redirect_for_pathname(&self, pathname: &str) -> bool {
        self.routes()
            .any(|route| route.is_redirect() && route.source.as_deref() == Some(pathname))
    }

    pub(super) fn indexed_routes(
        &self,
    ) -> impl Iterator<Item = (&'static str, usize, &AdapterRoute)> {
        self.before_middleware
            .iter()
            .enumerate()
            .map(|(index, route)| ("beforeMiddleware", index, route))
            .chain(
                self.before_files
                    .iter()
                    .enumerate()
                    .map(|(index, route)| ("beforeFiles", index, route)),
            )
            .chain(
                self.after_files
                    .iter()
                    .enumerate()
                    .map(|(index, route)| ("afterFiles", index, route)),
            )
            .chain(
                self.dynamic_routes
                    .iter()
                    .enumerate()
                    .map(|(index, route)| ("dynamicRoutes", index, route)),
            )
            .chain(
                self.on_match
                    .iter()
                    .enumerate()
                    .map(|(index, route)| ("onMatch", index, route)),
            )
            .chain(
                self.fallback
                    .iter()
                    .enumerate()
                    .map(|(index, route)| ("fallback", index, route)),
            )
    }

    pub(super) fn has_compute_effect_for_path(&self, pathname: &str, has_middleware: bool) -> bool {
        self.indexed_routes().any(|(bucket, index, route)| {
            route.has_effect()
                && route.to_edge_rule(bucket, index, has_middleware).is_none()
                && route.may_match_path(pathname)
        })
    }
}

impl AdapterRoute {
    pub(super) fn to_edge_rule(
        &self,
        bucket: &'static str,
        index: usize,
        has_middleware: bool,
    ) -> Option<serde_json::Value> {
        if !self.phase_is_edge_safe(bucket, has_middleware) {
            return None;
        }
        let (condition_path, captures) = self.edge_rule_path_condition()?;
        let mut condition = self.edge_rule_request_condition()?;
        condition.insert("path".to_string(), condition_path);
        let mut action = self.edge_rule_action(&captures)?;
        let kind = action.get("type")?.as_str()?.to_string();
        if matches!(bucket, "beforeMiddleware" | "beforeFiles")
            && matches!(kind.as_str(), "redirect" | "rewrite")
        {
            action
                .as_object_mut()?
                .insert("ifNoFile".to_string(), serde_json::Value::Bool(false));
        }
        let rule = serde_json::json!({
            "id": format!("next.{kind}.{bucket}.{index}"),
            "condition": condition,
            "action": action,
        });
        let candidate = serde_json::json!({
            "schemaVersion": "EDGE_RULE_SET_V1",
            "rules": [rule.clone()],
        });
        crate::functions::validate_edge_rules_value(
            "Next.js adapter generated Edge Rule",
            &candidate,
        )
        .ok()?;
        Some(rule)
    }

    fn phase_is_edge_safe(&self, bucket: &str, has_middleware: bool) -> bool {
        match bucket {
            "beforeMiddleware" => self.is_redirect() || (self.has_headers() && !self.is_rewrite()),
            "beforeFiles" => !has_middleware,
            "onMatch" => self.is_immutable_next_static_header_rule(),
            _ => false,
        }
    }

    fn is_immutable_next_static_header_rule(&self) -> bool {
        if !self.has_headers()
            || self.is_redirect()
            || self.is_rewrite()
            || value_is_present(&self.has)
            || value_is_present(&self.missing)
        {
            return false;
        }
        self.edge_rule_path_condition()
            .and_then(|(condition, _)| {
                condition
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .is_some_and(|path| path == "/_next/static/{path...}")
    }

    fn may_match_path(&self, pathname: &str) -> bool {
        if let Some(source_regex) = self.source_regex.as_deref() {
            let source_regex = normalize_next_source_regex_for_matching(source_regex);
            return Regex::new(&source_regex).map_or(true, |regex| regex.is_match(pathname));
        }
        let Some((condition, _)) = self.edge_rule_path_condition() else {
            return true;
        };
        let Some(condition_type) = condition.get("type").and_then(serde_json::Value::as_str) else {
            return true;
        };
        let Some(pattern) = condition.get("value").and_then(serde_json::Value::as_str) else {
            return true;
        };
        match condition_type {
            "exact" => pathname == pattern,
            "glob" => edge_glob_may_match_path(pattern, pathname),
            _ => true,
        }
    }

    pub(super) fn edge_rule_path_condition(
        &self,
    ) -> Option<(serde_json::Value, Vec<NextSourceCapture>)> {
        if let Some(source) = self.source.as_deref() {
            return next_source_to_edge_path_condition(source);
        }

        if let Some(source_regex) = self.source_regex.as_deref() {
            return next_source_regex_to_edge_path_condition(source_regex);
        }

        None
    }

    pub(super) fn edge_rule_action(
        &self,
        captures: &[NextSourceCapture],
    ) -> Option<serde_json::Value> {
        if self.is_redirect() {
            if self.headers.len() != 1 {
                return None;
            }
            let status = self.status?;
            if !matches!(status, 301 | 302 | 307 | 308) {
                return None;
            }
            let target = rewrite_next_target(self.location_header()?, captures)?;
            return Some(serde_json::json!({
                "type": "redirect",
                "target": target,
                "statusCode": status,
            }));
        }

        if self.is_rewrite() {
            if self.has_headers() || self.status.is_some() {
                return None;
            }
            let target = rewrite_next_target(self.destination.as_deref()?, captures)?;
            let lower_target = target.to_ascii_lowercase();
            if lower_target.starts_with("http://") {
                return None;
            }
            let external = lower_target.starts_with("https://");
            return Some(serde_json::json!({
                "type": "rewrite",
                "target": target,
                "external": external,
            }));
        }

        if self.has_headers() {
            if self.status.is_some() {
                return None;
            }
            return Some(serde_json::json!({
                "type": "set_headers",
                "headers": self.headers,
            }));
        }

        None
    }

    pub(super) fn has_effect(&self) -> bool {
        self.is_redirect() || self.is_rewrite() || self.has_headers()
    }

    pub(super) fn edge_rule_request_condition(
        &self,
    ) -> Option<serde_json::Map<String, serde_json::Value>> {
        let mut condition = serde_json::Map::new();
        lower_next_route_conditions(self.has.as_ref(), &mut condition)?;

        if value_is_present(&self.missing) {
            let mut not = serde_json::Map::new();
            lower_next_route_conditions(self.missing.as_ref(), &mut not)?;
            if !not.is_empty() {
                condition.insert("not".to_string(), serde_json::Value::Object(not));
            }
        }

        Some(condition)
    }

    pub(super) fn is_redirect(&self) -> bool {
        self.status.is_some() && self.location_header().is_some()
    }

    pub(super) fn is_rewrite(&self) -> bool {
        self.destination.is_some()
    }

    pub(super) fn has_headers(&self) -> bool {
        !self.headers.is_empty()
    }

    pub(super) fn is_priority(&self) -> bool {
        self.priority == Some(true)
    }

    pub(super) fn location_header(&self) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case("location"))
            .map(|(_, value)| value.as_str())
    }
}

fn edge_glob_may_match_path(pattern: &str, pathname: &str) -> bool {
    let pattern_parts = pattern.trim_matches('/').split('/').collect::<Vec<_>>();
    let pathname_parts = pathname.trim_matches('/').split('/').collect::<Vec<_>>();
    let mut pathname_index = 0usize;
    for pattern_part in pattern_parts {
        if pattern_part.starts_with('{') && pattern_part.ends_with("...}") {
            return true;
        }
        let Some(pathname_part) = pathname_parts.get(pathname_index) else {
            return false;
        };
        if !(pattern_part.starts_with('{') && pattern_part.ends_with('}'))
            && pattern_part != *pathname_part
        {
            return false;
        }
        pathname_index += 1;
    }
    pathname_index == pathname_parts.len()
}

pub(super) fn value_is_present(value: &Option<serde_json::Value>) -> bool {
    match value {
        None | Some(serde_json::Value::Null) => false,
        Some(serde_json::Value::Array(items)) => !items.is_empty(),
        Some(_) => true,
    }
}

pub(super) fn next_source_to_edge_path_condition(
    source: &str,
) -> Option<(serde_json::Value, Vec<NextSourceCapture>)> {
    if !source.starts_with('/') || source.contains(['?', '#', '(', ')', '$']) {
        return None;
    }

    if !source.contains(':') && !source.contains(['*', '+', '[', ']', '{', '}']) {
        return Some((
            serde_json::json!({ "type": "exact", "value": source }),
            Vec::new(),
        ));
    }

    let trailing_slash = source.len() > 1 && source.ends_with('/');
    let source = if trailing_slash {
        source.trim_end_matches('/')
    } else {
        source
    };
    let mut captures: Vec<NextSourceCapture> = Vec::new();
    let mut glob_segments = Vec::new();
    let parts = source
        .trim_start_matches('/')
        .split('/')
        .collect::<Vec<_>>();
    if parts.iter().any(|part| part.is_empty()) {
        return None;
    }

    for (index, part) in parts.iter().enumerate() {
        if let Some(param) = part.strip_prefix(':') {
            let is_last = index + 1 == parts.len();
            let (name, splat) = parse_next_source_param(param)?;
            if splat && !is_last {
                return None;
            }
            if captures.iter().any(|capture| capture.name == name) {
                return None;
            }
            captures.push(NextSourceCapture {
                name: name.to_string(),
            });
            if splat {
                glob_segments.push(format!("{{{name}...}}"));
            } else {
                glob_segments.push(format!("{{{name}}}"));
            }
            continue;
        }

        if !literal_source_segment_supported(part) {
            return None;
        }
        glob_segments.push((*part).to_string());
    }

    let mut value = format!("/{}", glob_segments.join("/"));
    if trailing_slash {
        value.push('/');
    }
    Some((
        serde_json::json!({
            "type": "glob",
            "value": value,
        }),
        captures,
    ))
}

pub(super) fn next_source_regex_to_edge_path_condition(
    source_regex: &str,
) -> Option<(serde_json::Value, Vec<NextSourceCapture>)> {
    if next_source_regex_targets_next_static(source_regex) {
        return Some((
            serde_json::json!({
                "type": "glob",
                "value": "/_next/static/{path...}",
            }),
            Vec::new(),
        ));
    }

    let normalized = normalize_next_source_regex_for_rust(source_regex);
    let source = normalized.strip_prefix('^')?.strip_suffix('$')?;
    if source.is_empty() || !source.starts_with('/') {
        return None;
    }
    if source.contains([
        '\\', '^', '$', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '<', '>',
    ]) {
        return None;
    }

    Some((
        serde_json::json!({ "type": "exact", "value": source }),
        Vec::new(),
    ))
}

pub(super) fn lower_next_route_conditions(
    value: Option<&serde_json::Value>,
    condition: &mut serde_json::Map<String, serde_json::Value>,
) -> Option<()> {
    let Some(value) = value else {
        return Some(());
    };
    match value {
        serde_json::Value::Null => return Some(()),
        serde_json::Value::Array(items) if items.is_empty() => return Some(()),
        serde_json::Value::Array(items) => {
            for item in items {
                lower_next_route_condition(item, condition)?;
            }
        }
        _ => return None,
    }
    Some(())
}

pub(super) fn lower_next_route_condition(
    value: &serde_json::Value,
    condition: &mut serde_json::Map<String, serde_json::Value>,
) -> Option<()> {
    let object = value.as_object()?;
    let kind = object.get("type")?.as_str()?;
    let raw_value = object.get("value")?.as_str()?;
    let value = next_route_condition_literal_value(raw_value)?;

    match kind {
        "header" => {
            insert_condition_map_value(condition, "headers", object.get("key")?.as_str()?, value)
        }
        "cookie" => {
            insert_condition_map_value(condition, "cookies", object.get("key")?.as_str()?, value)
        }
        "query" => {
            insert_condition_map_value(condition, "query", object.get("key")?.as_str()?, value)
        }
        "host" => {
            if condition.contains_key("host") || value.is_empty() {
                return None;
            }
            condition.insert(
                "host".to_string(),
                serde_json::Value::String(value.to_string()),
            );
            Some(())
        }
        _ => None,
    }
}

pub(super) fn insert_condition_map_value(
    condition: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
    key: &str,
    value: &str,
) -> Option<()> {
    if key.is_empty() {
        return None;
    }
    let entry = condition
        .entry(field.to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let map = entry.as_object_mut()?;
    if map.contains_key(key) {
        return None;
    }
    map.insert(
        key.to_string(),
        serde_json::Value::String(value.to_string()),
    );
    Some(())
}

pub(super) fn next_route_condition_literal_value(value: &str) -> Option<&str> {
    if value.contains([
        '\\', '^', '$', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '<', '>',
    ]) {
        return None;
    }
    Some(value)
}

pub(super) fn parse_next_source_param(param: &str) -> Option<(&str, bool)> {
    let (name, splat) = if let Some(name) = param.strip_suffix('*') {
        (name, true)
    } else if let Some(name) = param.strip_suffix('+') {
        (name, true)
    } else {
        (param, false)
    };

    if !valid_capture_name(name) {
        return None;
    }
    Some((name, splat))
}

pub(super) fn literal_source_segment_supported(segment: &str) -> bool {
    !segment.is_empty()
        && !segment.contains([':', '*', '+', '(', ')', '[', ']', '{', '}', '?', '#', '$'])
}

pub(super) fn valid_capture_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub(super) fn rewrite_next_target(target: &str, captures: &[NextSourceCapture]) -> Option<String> {
    let param = Regex::new(r":([A-Za-z][A-Za-z0-9_]*)([*+]?)").ok()?;
    let mut supported = true;
    let rewritten = param.replace_all(target, |captures_match: &regex::Captures<'_>| {
        let name = captures_match.get(1).expect("capture exists").as_str();
        if !captures.iter().any(|capture| capture.name == name) {
            supported = false;
        }
        format!("{{{name}}}")
    });
    if !supported {
        return None;
    }

    let numeric_capture = Regex::new(r"\$(\d+)").ok()?;
    let mut supported = true;
    let rewritten =
        numeric_capture.replace_all(&rewritten, |capture_match: &regex::Captures<'_>| {
            let raw_index = capture_match.get(1).expect("capture exists").as_str();
            let Some(index) = raw_index.parse::<usize>().ok().filter(|index| *index > 0) else {
                supported = false;
                return String::new();
            };
            let Some(capture) = captures.get(index - 1) else {
                supported = false;
                return String::new();
            };
            format!("{{{}}}", capture.name)
        });
    if !supported || rewritten.contains('$') {
        return None;
    }

    Some(rewritten.into_owned())
}
