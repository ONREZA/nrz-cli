use rusqlite::Connection;

use super::tracking::*;

fn mem_db() -> Connection {
    Connection::open_in_memory().unwrap()
}

#[test]
fn test_init_tracking_table() {
    let conn = mem_db();
    init_tracking_table(&conn).unwrap();
    // idempotent
    init_tracking_table(&conn).unwrap();
}

#[test]
fn test_get_applied_empty() {
    let conn = mem_db();
    init_tracking_table(&conn).unwrap();
    let applied = get_applied(&conn).unwrap();
    assert!(applied.is_empty());
}

#[test]
fn test_record_and_get_applied() {
    let conn = mem_db();
    init_tracking_table(&conn).unwrap();

    record_applied(&conn, "0001_init", "abc123").unwrap();
    record_applied(&conn, "0002_users", "def456").unwrap();

    let applied = get_applied(&conn).unwrap();
    assert_eq!(applied.len(), 2);
    assert_eq!(applied[0].name, "0001_init");
    assert_eq!(applied[0].checksum, "abc123");
    assert_eq!(applied[1].name, "0002_users");
    assert_eq!(applied[1].checksum, "def456");
    assert!(!applied[0].applied_at.is_empty());
}

#[test]
fn test_record_duplicate_fails() {
    let conn = mem_db();
    init_tracking_table(&conn).unwrap();

    record_applied(&conn, "0001_init", "abc").unwrap();
    let result = record_applied(&conn, "0001_init", "abc");
    assert!(result.is_err());
}

#[test]
fn test_applied_sorted_by_name() {
    let conn = mem_db();
    init_tracking_table(&conn).unwrap();

    record_applied(&conn, "0003_c", "c").unwrap();
    record_applied(&conn, "0001_a", "a").unwrap();
    record_applied(&conn, "0002_b", "b").unwrap();

    let applied = get_applied(&conn).unwrap();
    assert_eq!(applied[0].name, "0001_a");
    assert_eq!(applied[1].name, "0002_b");
    assert_eq!(applied[2].name, "0003_c");
}
