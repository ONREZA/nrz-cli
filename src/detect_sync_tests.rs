use crate::detect::types::PackageManagerType;

#[test]
fn detection_package_manager_sync_uses_platform_enum_values() {
    assert_eq!(
        super::detect_sync::detection_package_manager_to_platform(PackageManagerType::Npm)
            .to_string(),
        "NPM"
    );
    assert_eq!(
        super::detect_sync::detection_package_manager_to_platform(PackageManagerType::Yarn)
            .to_string(),
        "YARN"
    );
    assert_eq!(
        super::detect_sync::detection_package_manager_to_platform(PackageManagerType::Pnpm)
            .to_string(),
        "PNPM"
    );
    assert_eq!(
        super::detect_sync::detection_package_manager_to_platform(PackageManagerType::Bun)
            .to_string(),
        "BUN"
    );
    assert_eq!(
        super::detect_sync::detection_package_manager_to_platform(PackageManagerType::Pip)
            .to_string(),
        "PIP"
    );
}
