use std::path::Path;

use anyhow::{Context, bail};

use crate::detect::fs::LocalFs;
use crate::detect::package_json::PackageJson;
use crate::detect::package_manager::{YarnGeneration, detect_yarn_generation_if_configured};

const YARN_PACKAGE_MANAGER: &str = "YARN";
const YARN_CLASSIC_BIN_DIR_ENV: &str = "NRZ_PLATFORM_YARN_CLASSIC_BIN_DIR";
const YARN_MODERN_BIN_DIR_ENV: &str = "NRZ_PLATFORM_YARN_MODERN_BIN_DIR";

pub(super) struct YarnToolchainPaths<'a> {
    pub classic_bin_dir: &'a str,
    pub modern_bin_dir: &'a str,
    pub base_path: &'a str,
}

pub(super) fn environment_from_process(
    package_manager: &str,
    project_dir: &Path,
    source_root: &Path,
) -> anyhow::Result<Vec<(String, String)>> {
    if package_manager != YARN_PACKAGE_MANAGER {
        return Ok(Vec::new());
    }
    let classic_bin_dir = std::env::var(YARN_CLASSIC_BIN_DIR_ENV)
        .with_context(|| format!("{YARN_CLASSIC_BIN_DIR_ENV} is not configured"))?;
    let modern_bin_dir = std::env::var(YARN_MODERN_BIN_DIR_ENV)
        .with_context(|| format!("{YARN_MODERN_BIN_DIR_ENV} is not configured"))?;
    let base_path = std::env::var("PATH").context("PATH is not configured")?;
    validate_toolchain_executables(&classic_bin_dir, "Yarn Classic")?;
    validate_toolchain_executables(&modern_bin_dir, "Yarn Modern")?;
    select_environment(
        package_manager,
        project_dir,
        source_root,
        YarnToolchainPaths {
            classic_bin_dir: &classic_bin_dir,
            modern_bin_dir: &modern_bin_dir,
            base_path: &base_path,
        },
    )
}

pub(super) fn select_environment(
    package_manager: &str,
    project_dir: &Path,
    source_root: &Path,
    paths: YarnToolchainPaths<'_>,
) -> anyhow::Result<Vec<(String, String)>> {
    if package_manager != YARN_PACKAGE_MANAGER {
        return Ok(Vec::new());
    }
    validate_bin_dir(paths.classic_bin_dir, YARN_CLASSIC_BIN_DIR_ENV)?;
    validate_bin_dir(paths.modern_bin_dir, YARN_MODERN_BIN_DIR_ENV)?;
    if paths.base_path.is_empty() {
        bail!("PATH must not be empty");
    }
    let generation = configured_yarn_generation(project_dir).or_else(|| {
        (source_root != project_dir && project_dir.starts_with(source_root))
            .then(|| configured_yarn_generation(source_root))
            .flatten()
    });
    let bin_dir = match generation.unwrap_or(YarnGeneration::Modern) {
        YarnGeneration::Classic => paths.classic_bin_dir,
        YarnGeneration::Modern => paths.modern_bin_dir,
    };
    Ok(vec![(
        "PATH".to_string(),
        format!("{bin_dir}:{}", paths.base_path),
    )])
}

fn configured_yarn_generation(project_dir: &Path) -> Option<YarnGeneration> {
    let fs = LocalFs::new(project_dir);
    let package_json = PackageJson::load_from_fs(&fs);
    detect_yarn_generation_if_configured(&fs, package_json.as_ref())
}

fn validate_bin_dir(value: &str, name: &str) -> anyhow::Result<()> {
    if value.is_empty() || value.contains(':') || !Path::new(value).is_absolute() {
        bail!("{name} must be an absolute PATH directory");
    }
    Ok(())
}

pub(super) fn validate_toolchain_executables(bin_dir: &str, label: &str) -> anyhow::Result<()> {
    for name in ["yarn", "yarnpkg"] {
        let path = Path::new(bin_dir).join(name);
        let metadata = std::fs::metadata(&path)
            .with_context(|| format!("{label} executable is unavailable: {}", path.display()))?;
        if !metadata.is_file() {
            bail!("{label} executable is not a file: {}", path.display());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            if metadata.permissions().mode() & 0o111 == 0 {
                bail!("{label} executable is not executable: {}", path.display());
            }
        }
    }
    Ok(())
}
