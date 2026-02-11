//! Unit tests for KV store

use std::path::Path;
use std::time::Duration;

use super::kv::{KvFile, KvFileEntry, KvStore, load_kv_file, save_kv_file};

// --- KvStore in-memory tests ---

#[test]
fn store_get_set() {
    let kv = KvStore::new();
    assert_eq!(kv.get("key"), None);

    kv.set("key".into(), "value".into(), 0);
    assert_eq!(kv.get("key"), Some("value".into()));
}

#[test]
fn store_overwrite() {
    let kv = KvStore::new();
    kv.set("k".into(), "v1".into(), 0);
    kv.set("k".into(), "v2".into(), 0);
    assert_eq!(kv.get("k"), Some("v2".into()));
}

#[test]
fn store_delete() {
    let kv = KvStore::new();
    assert!(!kv.delete("missing"));

    kv.set("k".into(), "v".into(), 0);
    assert!(kv.delete("k"));
    assert_eq!(kv.get("k"), None);
    assert!(!kv.delete("k"));
}

#[test]
fn store_has() {
    let kv = KvStore::new();
    assert!(!kv.has("k"));

    kv.set("k".into(), "v".into(), 0);
    assert!(kv.has("k"));

    kv.delete("k");
    assert!(!kv.has("k"));
}

#[test]
fn store_list_all() {
    let kv = KvStore::new();
    kv.set("a".into(), "1".into(), 0);
    kv.set("b".into(), "2".into(), 0);
    kv.set("c".into(), "3".into(), 0);

    let keys = kv.list(None, 100);
    assert_eq!(keys, vec!["a", "b", "c"]);
}

#[test]
fn store_list_prefix() {
    let kv = KvStore::new();
    kv.set("user:1".into(), "a".into(), 0);
    kv.set("user:2".into(), "b".into(), 0);
    kv.set("post:1".into(), "c".into(), 0);

    let keys = kv.list(Some("user:"), 100);
    assert_eq!(keys, vec!["user:1", "user:2"]);
}

#[test]
fn store_list_limit() {
    let kv = KvStore::new();
    kv.set("a".into(), "1".into(), 0);
    kv.set("b".into(), "2".into(), 0);
    kv.set("c".into(), "3".into(), 0);

    let keys = kv.list(None, 2);
    assert_eq!(keys.len(), 2);
}

#[test]
fn store_clear() {
    let kv = KvStore::new();
    kv.set("a".into(), "1".into(), 0);
    kv.set("b".into(), "2".into(), 0);
    kv.clear();
    assert_eq!(kv.list(None, 100), Vec::<String>::new());
}

#[test]
fn store_ttl_expiration() {
    let kv = KvStore::new();
    kv.set("k".into(), "v".into(), 1);
    assert_eq!(kv.get("k"), Some("v".into()));

    std::thread::sleep(Duration::from_millis(1100));
    assert_eq!(kv.get("k"), None);
}

#[test]
fn store_has_ttl_expiration() {
    let kv = KvStore::new();
    kv.set("k".into(), "v".into(), 1);
    assert!(kv.has("k"));

    std::thread::sleep(Duration::from_millis(1100));
    assert!(!kv.has("k"));
}

#[test]
fn store_list_excludes_expired() {
    let kv = KvStore::new();
    kv.set("keep".into(), "v".into(), 0);
    kv.set("expire".into(), "v".into(), 1);

    std::thread::sleep(Duration::from_millis(1100));
    let keys = kv.list(None, 100);
    assert_eq!(keys, vec!["keep"]);
}

#[test]
fn store_clone_shares_state() {
    let kv = KvStore::new();
    let kv2 = kv.clone();
    kv.set("k".into(), "v".into(), 0);
    assert_eq!(kv2.get("k"), Some("v".into()));
}

// --- kv.json persistence tests ---

#[test]
fn kvfile_load_missing_returns_default() {
    let kv = load_kv_file(Path::new("/nonexistent/kv.json"));
    assert!(kv.entries.is_empty());
}

#[test]
fn kvfile_save_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("kv.json");

    let mut kv = KvFile::default();
    kv.entries.insert(
        "key1".into(),
        KvFileEntry {
            value: "val1".into(),
            expires_at: None,
        },
    );
    kv.entries.insert(
        "key2".into(),
        KvFileEntry {
            value: "val2".into(),
            expires_at: Some(9999999999),
        },
    );
    save_kv_file(&path, &kv).unwrap();

    let loaded = load_kv_file(&path);
    assert_eq!(loaded.entries.len(), 2);
    assert_eq!(loaded.entries["key1"].value, "val1");
    assert_eq!(loaded.entries["key1"].expires_at, None);
    assert_eq!(loaded.entries["key2"].value, "val2");
    assert_eq!(loaded.entries["key2"].expires_at, Some(9999999999));
}

#[test]
fn kvfile_is_expired_no_ttl() {
    let entry = KvFileEntry {
        value: "v".into(),
        expires_at: None,
    };
    assert!(!super::kv::is_expired(&entry));
}

#[test]
fn kvfile_is_expired_future() {
    let entry = KvFileEntry {
        value: "v".into(),
        expires_at: Some(u64::MAX),
    };
    assert!(!super::kv::is_expired(&entry));
}

#[test]
fn kvfile_is_expired_past() {
    let entry = KvFileEntry {
        value: "v".into(),
        expires_at: Some(0),
    };
    assert!(super::kv::is_expired(&entry));
}

#[test]
fn kvfile_load_corrupt_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("kv.json");
    std::fs::write(&path, "not json").unwrap();

    let kv = load_kv_file(&path);
    assert!(kv.entries.is_empty());
}
