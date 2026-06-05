/// Outcome of resolving a local import specifier against the in-memory source set.
pub(crate) enum LocalResolution {
    /// Resolved to a known module path inside the bundle.
    Module(String),
    /// Local specifier that does not match any provided source (skipped, matching
    /// the publish-time behaviour of treating absent optional files as no-ops).
    NotFound,
    /// Specifier escapes the bundle root (e.g. `../../etc`).
    Escapes,
}

pub(crate) fn is_local_specifier(specifier: &str) -> bool {
    specifier.starts_with('.') || specifier.starts_with('/')
}

/// Resolve a local import the way the runtime loader would: relative to the
/// importer (or the bundle root for `/`-rooted specifiers), trying the supported
/// extensions and `index` files, against the keys present in the source set.
pub(crate) fn resolve_local_import(
    importer: &str,
    specifier: &str,
    extensions: &[String],
    sources: &dyn Fn(&str) -> bool,
) -> LocalResolution {
    let base_segments = if let Some(rooted) = specifier.strip_prefix('/') {
        split_segments(rooted)
    } else {
        let mut segments = parent_segments(importer);
        segments.extend(split_segments(specifier));
        segments
    };

    let Some(normalized) = normalize_segments(&base_segments) else {
        return LocalResolution::Escapes;
    };
    let base = normalized.join("/");

    for candidate in import_candidates(&base, extensions) {
        if sources(&candidate) {
            return LocalResolution::Module(candidate);
        }
    }

    LocalResolution::NotFound
}

fn import_candidates(base: &str, extensions: &[String]) -> Vec<String> {
    if has_extension(base) {
        return vec![base.to_string()];
    }

    let mut candidates = Vec::with_capacity(extensions.len() * 2);
    for extension in extensions {
        candidates.push(format!("{base}{extension}"));
    }
    for extension in extensions {
        candidates.push(format!("{base}/index{extension}"));
    }
    candidates
}

fn has_extension(path: &str) -> bool {
    match path.rsplit_once('/') {
        Some((_, last)) => last.contains('.'),
        None => path.contains('.'),
    }
}

fn parent_segments(path: &str) -> Vec<&str> {
    let mut segments = split_segments(path);
    segments.pop();
    segments
}

fn split_segments(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

/// Collapse `.`/`..` segments. Returns `None` if the path escapes the root.
fn normalize_segments(segments: &[&str]) -> Option<Vec<String>> {
    let mut stack: Vec<String> = Vec::with_capacity(segments.len());
    for segment in segments {
        match *segment {
            "." => {}
            ".." => {
                stack.pop()?;
            }
            other => stack.push(other.to_string()),
        }
    }
    Some(stack)
}
