use super::*;

#[test]
fn test_compute_checksum_deterministic() {
    let sql = "CREATE TABLE users (id INTEGER PRIMARY KEY);";
    let c1 = compute_checksum(sql);
    let c2 = compute_checksum(sql);
    assert_eq!(c1, c2);
    assert_eq!(c1.len(), 64); // SHA256 hex
}

#[test]
fn test_compute_checksum_different_sql() {
    let c1 = compute_checksum("SELECT 1");
    let c2 = compute_checksum("SELECT 2");
    assert_ne!(c1, c2);
}

#[test]
fn test_scan_migrations_dir_empty() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("migrations")).unwrap();
    let result = scan_migrations_dir(dir.path()).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_scan_migrations_dir_no_dir() {
    let dir = tempfile::tempdir().unwrap();
    // no migrations/ subdir
    let result = scan_migrations_dir(dir.path()).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_scan_migrations_dir_sorted() {
    let dir = tempfile::tempdir().unwrap();
    let mig_dir = dir.path().join("migrations");
    std::fs::create_dir(&mig_dir).unwrap();

    std::fs::write(mig_dir.join("0002_add_users.sql"), "CREATE TABLE users;").unwrap();
    std::fs::write(mig_dir.join("0001_init.sql"), "CREATE TABLE init;").unwrap();
    std::fs::write(mig_dir.join("0003_add_posts.sql"), "CREATE TABLE posts;").unwrap();

    let result = scan_migrations_dir(dir.path()).unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].name, "0001_init");
    assert_eq!(result[1].name, "0002_add_users");
    assert_eq!(result[2].name, "0003_add_posts");
}

#[test]
fn test_scan_migrations_ignores_non_sql() {
    let dir = tempfile::tempdir().unwrap();
    let mig_dir = dir.path().join("migrations");
    std::fs::create_dir(&mig_dir).unwrap();

    std::fs::write(mig_dir.join("0001_init.sql"), "CREATE TABLE init;").unwrap();
    std::fs::write(mig_dir.join("readme.md"), "some notes").unwrap();
    std::fs::write(mig_dir.join("backup.bak"), "backup").unwrap();

    let result = scan_migrations_dir(dir.path()).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "0001_init");
}

#[test]
fn test_scan_migrations_computes_checksum() {
    let dir = tempfile::tempdir().unwrap();
    let mig_dir = dir.path().join("migrations");
    std::fs::create_dir(&mig_dir).unwrap();

    let sql = "CREATE TABLE foo (id INTEGER);";
    std::fs::write(mig_dir.join("0001_foo.sql"), sql).unwrap();

    let result = scan_migrations_dir(dir.path()).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].checksum, compute_checksum(sql));
}

#[test]
fn test_next_migration_number_empty() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(next_migration_number(dir.path()).unwrap(), 1);
}

#[test]
fn test_next_migration_number_existing() {
    let dir = tempfile::tempdir().unwrap();
    let mig_dir = dir.path().join("migrations");
    std::fs::create_dir(&mig_dir).unwrap();

    std::fs::write(mig_dir.join("0001_init.sql"), "").unwrap();
    std::fs::write(mig_dir.join("0003_skip.sql"), "").unwrap();

    assert_eq!(next_migration_number(dir.path()).unwrap(), 4);
}
