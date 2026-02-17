use rusqlite::Connection;

use crate::migrations::{self, tracking};

use super::db_migrate_handler::resolve_sql;

#[test]
fn test_resolve_sql_from_argument() {
    let result = resolve_sql(Some("SELECT 1".into()), None).unwrap();
    assert_eq!(result, "SELECT 1");
}

#[test]
fn test_resolve_sql_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.sql");
    std::fs::write(&path, "CREATE TABLE test;").unwrap();

    let result = resolve_sql(None, Some(path.to_string_lossy().into_owned())).unwrap();
    assert_eq!(result, "CREATE TABLE test;");
}

#[test]
fn test_resolve_sql_empty_fails() {
    let result = resolve_sql(Some("  ".into()), None);
    assert!(result.is_err());
}

#[test]
fn test_resolve_sql_no_input_fails() {
    // When stdin is a terminal (which it is in tests), this should fail
    let result = resolve_sql(None, None);
    assert!(result.is_err());
}

#[test]
fn test_resolve_sql_nonexistent_file_fails() {
    let result = resolve_sql(None, Some("/nonexistent/path.sql".into()));
    assert!(result.is_err());
}

// ── apply_local transactional integrity tests ───────────────
// These test the migration + tracking interaction at module level

#[test]
fn test_apply_migrations_records_in_tracking() {
    let dir = tempfile::tempdir().unwrap();
    let mig_dir = dir.path().join("migrations");
    std::fs::create_dir(&mig_dir).unwrap();
    std::fs::write(
        mig_dir.join("0001_init.sql"),
        "CREATE TABLE t (id INTEGER);",
    )
    .unwrap();
    std::fs::write(
        mig_dir.join("0002_users.sql"),
        "CREATE TABLE users (name TEXT);",
    )
    .unwrap();

    let all = migrations::scan_migrations_dir(dir.path()).unwrap();
    assert_eq!(all.len(), 2);

    let mut conn = Connection::open_in_memory().unwrap();
    tracking::init_tracking_table(&conn).unwrap();

    // Apply each migration in a transaction
    for m in &all {
        let tx = conn.transaction().unwrap();
        tx.execute_batch(&m.sql).unwrap();
        tracking::record_applied(&tx, &m.name, &m.checksum).unwrap();
        tx.commit().unwrap();
    }

    let applied = tracking::get_applied(&conn).unwrap();
    assert_eq!(applied.len(), 2);
    assert_eq!(applied[0].name, "0001_init");
    assert_eq!(applied[1].name, "0002_users");
}

#[test]
fn test_apply_idempotent_skips_already_applied() {
    let dir = tempfile::tempdir().unwrap();
    let mig_dir = dir.path().join("migrations");
    std::fs::create_dir(&mig_dir).unwrap();
    std::fs::write(
        mig_dir.join("0001_init.sql"),
        "CREATE TABLE t (id INTEGER);",
    )
    .unwrap();

    let all = migrations::scan_migrations_dir(dir.path()).unwrap();

    let mut conn = Connection::open_in_memory().unwrap();
    tracking::init_tracking_table(&conn).unwrap();

    // Apply first time
    let tx = conn.transaction().unwrap();
    tx.execute_batch(&all[0].sql).unwrap();
    tracking::record_applied(&tx, &all[0].name, &all[0].checksum).unwrap();
    tx.commit().unwrap();

    // Check that it's applied
    let applied = tracking::get_applied(&conn).unwrap();
    let applied_names: std::collections::HashSet<_> = applied.iter().map(|a| &a.name).collect();
    let pending: Vec<_> = all
        .iter()
        .filter(|m| !applied_names.contains(&m.name))
        .collect();
    assert!(pending.is_empty());
}

#[test]
fn test_failed_migration_rolls_back_transaction() {
    let mut conn = Connection::open_in_memory().unwrap();
    tracking::init_tracking_table(&conn).unwrap();

    // Apply a valid migration first
    let tx = conn.transaction().unwrap();
    tx.execute_batch("CREATE TABLE t (id INTEGER);").unwrap();
    tracking::record_applied(&tx, "0001_init", "checksum1").unwrap();
    tx.commit().unwrap();

    // Attempt a migration with invalid SQL — should fail
    let tx = conn.transaction().unwrap();
    let result = tx.execute_batch("INVALID SQL STATEMENT");
    assert!(result.is_err());
    // Transaction is rolled back automatically on drop (not committed)
    drop(tx);

    // Verify only the first migration is recorded
    let applied = tracking::get_applied(&conn).unwrap();
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].name, "0001_init");

    // Verify the second migration's table doesn't exist
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='bad_table'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!table_exists);
}

#[test]
fn test_partial_failure_preserves_successful_migrations() {
    let dir = tempfile::tempdir().unwrap();
    let mig_dir = dir.path().join("migrations");
    std::fs::create_dir(&mig_dir).unwrap();
    std::fs::write(
        mig_dir.join("0001_init.sql"),
        "CREATE TABLE t (id INTEGER);",
    )
    .unwrap();
    std::fs::write(mig_dir.join("0002_bad.sql"), "THIS IS NOT VALID SQL;").unwrap();

    let all = migrations::scan_migrations_dir(dir.path()).unwrap();
    assert_eq!(all.len(), 2);

    let mut conn = Connection::open_in_memory().unwrap();
    tracking::init_tracking_table(&conn).unwrap();

    // Apply migrations one by one
    let mut applied_count = 0;
    for m in &all {
        let tx = conn.transaction().unwrap();
        match tx.execute_batch(&m.sql) {
            Ok(()) => {
                tracking::record_applied(&tx, &m.name, &m.checksum).unwrap();
                tx.commit().unwrap();
                applied_count += 1;
            }
            Err(_) => break,
        }
    }

    assert_eq!(applied_count, 1);
    let applied = tracking::get_applied(&conn).unwrap();
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].name, "0001_init");
}

#[test]
fn test_checksum_drift_detected() {
    let dir = tempfile::tempdir().unwrap();
    let mig_dir = dir.path().join("migrations");
    std::fs::create_dir(&mig_dir).unwrap();
    std::fs::write(
        mig_dir.join("0001_init.sql"),
        "CREATE TABLE t (id INTEGER);",
    )
    .unwrap();

    let all = migrations::scan_migrations_dir(dir.path()).unwrap();

    let conn = Connection::open_in_memory().unwrap();
    tracking::init_tracking_table(&conn).unwrap();
    tracking::record_applied(&conn, &all[0].name, "old_checksum_that_differs").unwrap();

    // Now the file checksum doesn't match the recorded one
    let applied = tracking::get_applied(&conn).unwrap();
    let m = &all[0];
    let a = applied.iter().find(|a| a.name == m.name).unwrap();
    assert_ne!(a.checksum, m.checksum);
}
