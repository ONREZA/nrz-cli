/// Shared CPython site initialization used by serving and build launchers.
///
/// Defines `onreza_add_site_packages(directory, source_paths=())`: process the
/// installed tree's `.pth` files once, ahead of application source fallbacks.
pub const PYTHON_SITE_PACKAGES_INITIALIZER: &str =
    include_str!("../assets/python-site-packages.py");

/// Resolve a published Python script using the same installed-site initialization
/// as generated module/framework bootstraps. File identity and literal argv stay
/// in the immutable launch; no file discovery or installer runs in Compute.
#[must_use]
pub fn python_script_launch_arguments(
    entry: &str,
    dependency_root: &str,
    application_root: &str,
) -> Vec<String> {
    vec![
        "-S".into(),
        "-c".into(),
        format!(
            "{}\n{}",
            PYTHON_SITE_PACKAGES_INITIALIZER,
            include_str!("../assets/python-script-launcher.py")
        ),
        application_root.into(),
        dependency_root.into(),
        entry.into(),
    ]
}
