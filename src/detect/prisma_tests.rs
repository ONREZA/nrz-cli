use super::package_json::PackageJson;
use super::prisma::detect_prisma;
use super::types::{PackageManagerType, ToolSlug};

fn pkg_from_json(json: &str) -> PackageJson {
    serde_json::from_str(json).unwrap()
}

#[test]
fn detect_with_client_dep() {
    let pkg = pkg_from_json(r#"{"dependencies":{"@prisma/client":"^6.0.0"}}"#);
    let tool = detect_prisma(&pkg, PackageManagerType::Npm).unwrap();
    assert_eq!(tool.slug, ToolSlug::Prisma);
    assert_eq!(tool.name, "Prisma ORM");
    assert_eq!(tool.version.as_deref(), Some("^6.0.0"));
    assert_eq!(tool.pre_build_command, "npx prisma generate");
}

#[test]
fn detect_with_cli_dev_dep() {
    let pkg = pkg_from_json(r#"{"devDependencies":{"prisma":"^5.19.0"}}"#);
    let tool = detect_prisma(&pkg, PackageManagerType::Npm).unwrap();
    assert_eq!(tool.slug, ToolSlug::Prisma);
    assert_eq!(tool.version.as_deref(), Some("^5.19.0"));
}

#[test]
fn detect_with_both_deps() {
    let pkg = pkg_from_json(
        r#"{"dependencies":{"@prisma/client":"^6.0.0"},"devDependencies":{"prisma":"^6.0.0"}}"#,
    );
    let tool = detect_prisma(&pkg, PackageManagerType::Bun).unwrap();
    assert_eq!(tool.version.as_deref(), Some("^6.0.0"));
    assert_eq!(tool.pre_build_command, "bunx prisma generate");
}

#[test]
fn no_prisma_without_deps() {
    let pkg = pkg_from_json(r#"{"dependencies":{"next":"^14.0.0"}}"#);
    assert!(detect_prisma(&pkg, PackageManagerType::Npm).is_none());
}

#[test]
fn no_prisma_empty_pkg() {
    let pkg = pkg_from_json(r#"{}"#);
    assert!(detect_prisma(&pkg, PackageManagerType::Npm).is_none());
}

#[test]
fn runner_per_package_manager() {
    let pkg = pkg_from_json(r#"{"dependencies":{"@prisma/client":"^6.0.0"}}"#);

    let npm = detect_prisma(&pkg, PackageManagerType::Npm).unwrap();
    assert_eq!(npm.pre_build_command, "npx prisma generate");

    let yarn = detect_prisma(&pkg, PackageManagerType::Yarn).unwrap();
    assert_eq!(yarn.pre_build_command, "yarn prisma generate");

    let pnpm = detect_prisma(&pkg, PackageManagerType::Pnpm).unwrap();
    assert_eq!(pnpm.pre_build_command, "pnpm exec prisma generate");

    let bun = detect_prisma(&pkg, PackageManagerType::Bun).unwrap();
    assert_eq!(bun.pre_build_command, "bunx prisma generate");
}

#[test]
fn prisma_cli_in_deps_not_dev_deps() {
    let pkg = pkg_from_json(r#"{"dependencies":{"prisma":"^6.0.0"}}"#);
    let tool = detect_prisma(&pkg, PackageManagerType::Npm).unwrap();
    assert_eq!(tool.slug, ToolSlug::Prisma);
}
