use super::FileEntry;
use anyhow::{Context, bail, ensure};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

/// Explicit execution target supplied by the admitted edge agent. The CLI
/// binary's compile target cannot identify the libc of the runtime rootfs.
pub(crate) struct RuntimePlatform {
    os: String,
    cpu: String,
    libc: String,
}

impl RuntimePlatform {
    pub(crate) fn from_environment() -> anyhow::Result<Self> {
        Self::from_values(
            &std::env::var("ONREZA_RUNTIME_OS").context("missing edge runtime OS")?,
            &std::env::var("ONREZA_RUNTIME_ARCH").context("missing edge runtime architecture")?,
            &std::env::var("ONREZA_RUNTIME_LIBC").context("missing edge runtime libc")?,
        )
    }

    pub(super) fn from_values(os: &str, arch: &str, libc: &str) -> anyhow::Result<Self> {
        ensure!(
            os == "linux" && matches!(libc, "glibc" | "musl"),
            "unsupported edge runtime platform"
        );
        let cpu = match arch {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            _ => bail!("unsupported edge runtime architecture"),
        };
        Ok(Self {
            os: os.into(),
            cpu: cpu.into(),
            libc: libc.into(),
        })
    }

    fn accepts(&self, package: &Value) -> bool {
        [("os", &self.os), ("cpu", &self.cpu), ("libc", &self.libc)]
            .into_iter()
            .all(|(field, value)| accepts_list(package.get(field), value))
    }
}

// npm-install-checks checkList semantics: strings and arrays, negative veto,
// positive allow list, and the singleton "any". Unknown metadata is retained.
fn accepts_list(value: Option<&Value>, target: &str) -> bool {
    let Some(value) = value else {
        return true;
    };
    let values = match value {
        Value::String(value) => vec![value.as_str()],
        Value::Array(values) => {
            let Some(values) = values.iter().map(Value::as_str).collect::<Option<Vec<_>>>() else {
                return true;
            };
            values
        }
        _ => return true,
    };
    if values == ["any"] {
        return true;
    }
    !values
        .iter()
        .any(|value| value.strip_prefix('!') == Some(target))
        && (values.iter().all(|value| value.starts_with('!')) || values.contains(&target))
}

pub(crate) struct PrunedDependencies {
    pub(crate) files: Vec<FileEntry>,
    pub(crate) packages: usize,
    pub(crate) bytes: u64,
}

pub(crate) fn prune_optional_native_dependencies(
    root: &Path,
    files: Vec<FileEntry>,
    target: &RuntimePlatform,
) -> anyhow::Result<PrunedDependencies> {
    let root = root.canonicalize()?;
    let mut packages = BTreeMap::<PathBuf, Value>::new();
    for file in &files {
        let path = Path::new(&file.path);
        if file.size <= 1024 * 1024 && path.file_name().is_some_and(|name| name == "package.json") {
            let full = root.join(path).canonicalize()?;
            ensure!(
                full.starts_with(&root),
                "package metadata escaped the runtime artifact"
            );
            // Invalid/non-package JSON is not evidence authorizing removal.
            if let Ok(value) = serde_json::from_slice::<Value>(&std::fs::read(&full)?) {
                packages.insert(
                    full.parent()
                        .context("package directory missing")?
                        .to_owned(),
                    value,
                );
            }
        }
    }
    let incompatible: BTreeSet<_> = packages
        .iter()
        .filter(|(path, package)| {
            path.strip_prefix(&root).is_ok_and(|path| {
                path.components()
                    .any(|part| part.as_os_str() == "node_modules")
            }) && !target.accepts(package)
        })
        .map(|(path, _)| path.clone())
        .collect();
    let mut optional = BTreeSet::new();
    let mut required = BTreeSet::new();
    for (parent, package) in &packages {
        if incompatible.contains(parent) {
            continue;
        }
        let optional_names = package
            .get("optionalDependencies")
            .and_then(Value::as_object);
        for field in ["dependencies", "optionalDependencies", "peerDependencies"] {
            let Some(dependencies) = package.get(field).and_then(Value::as_object) else {
                continue;
            };
            for name in dependencies.keys() {
                let is_optional = optional_names.is_some_and(|names| names.contains_key(name))
                    || (field == "peerDependencies"
                        && package
                            .get("peerDependenciesMeta")
                            .and_then(|meta| meta.get(name))
                            .and_then(|meta| meta.get("optional"))
                            .and_then(Value::as_bool)
                            == Some(true));
                let Some(dependency) = resolve_dependency(&root, parent, name, &packages)? else {
                    continue;
                };
                if is_optional {
                    optional.insert(dependency);
                } else {
                    required.insert(dependency);
                }
            }
        }
    }
    if let Some(path) = incompatible.intersection(&required).next() {
        return Err(crate::output::coded_error(
            "RUNTIME_DEPENDENCY_INCOMPATIBLE",
            format!(
                "required runtime dependency {} is incompatible with {}/{}/{}",
                path.strip_prefix(&root)?.display(),
                target.os,
                target.cpu,
                target.libc
            ),
        ));
    }
    // Only explicit optional edges authorize pruning. Unreferenced traced files
    // and packages without usable metadata are kept, rather than guessed at.
    let removed: Vec<_> = incompatible
        .intersection(&optional)
        .filter(|candidate| {
            !required
                .iter()
                .chain(optional.iter())
                .any(|reference| reference != *candidate && reference.starts_with(candidate))
        })
        .cloned()
        .collect();
    if removed.is_empty() {
        return Ok(PrunedDependencies {
            files,
            packages: 0,
            bytes: 0,
        });
    }
    let mut retained = Vec::with_capacity(files.len());
    let mut bytes = 0_u64;
    for file in files {
        let path = root.join(&file.path);
        let canonical = path.canonicalize()?;
        let remove = removed
            .iter()
            .any(|removed| path.starts_with(removed) || canonical.starts_with(removed));
        if remove {
            if !Path::new(&file.path)
                .components()
                .any(|part| part.as_os_str() == "node_modules")
            {
                return Err(crate::output::coded_error(
                    "RUNTIME_DEPENDENCY_INCOMPATIBLE",
                    format!(
                        "incompatible optional dependency is also referenced by application path {}",
                        file.path
                    ),
                ));
            }
            bytes = bytes.saturating_add(file.size);
        } else {
            retained.push(file);
        }
    }
    Ok(PrunedDependencies {
        files: retained,
        packages: removed.len(),
        bytes,
    })
}

fn resolve_dependency(
    root: &Path,
    parent: &Path,
    name: &str,
    packages: &BTreeMap<PathBuf, Value>,
) -> anyhow::Result<Option<PathBuf>> {
    let parts: Vec<_> = name.split('/').collect();
    if !(parts.len() == 1 || (parts.len() == 2 && parts[0].starts_with('@')))
        || parts
            .iter()
            .any(|part| part.is_empty() || matches!(*part, "." | "..") || part.contains('\\'))
    {
        return Ok(None);
    }
    for ancestor in parent.ancestors().take_while(|path| path.starts_with(root)) {
        if ancestor
            .file_name()
            .is_some_and(|name| name == "node_modules")
        {
            continue;
        }
        let candidate = ancestor.join("node_modules").join(name);
        if let Ok(canonical) = candidate.canonicalize() {
            ensure!(
                canonical.starts_with(root),
                "runtime dependency escaped the artifact"
            );
            if packages.contains_key(&canonical) {
                return Ok(Some(canonical));
            }
            // A present but untraced dependency shadows higher ancestors.
            return Ok(None);
        }
    }
    Ok(None)
}
