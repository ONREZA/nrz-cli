use crate::detect::types::PackageManagerType;

#[test]
fn detection_package_manager_sync_uses_platform_enum_values() {
    assert_eq!(
        super::detect_sync::detection_package_manager_to_platform(PackageManagerType::Npm),
        "NPM"
    );
    assert_eq!(
        super::detect_sync::detection_package_manager_to_platform(PackageManagerType::Yarn),
        "YARN"
    );
    assert_eq!(
        super::detect_sync::detection_package_manager_to_platform(PackageManagerType::Pnpm),
        "PNPM"
    );
    assert_eq!(
        super::detect_sync::detection_package_manager_to_platform(PackageManagerType::Bun),
        "BUN"
    );
}
