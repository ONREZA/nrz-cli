use super::{SourceArchivePathIndex, SourcePathGraphError};

#[test]
fn required_paths_preserve_intermediate_aliases_and_directory_visits() {
    let index = SourceArchivePathIndex::from_entries([
        ("assets/pkg", Some("../package.json")),
        ("package.json", Some("payload.json")),
        ("payload.json", None),
        ("assets/repeated", Some("../alias/../alias/file")),
        ("alias", Some("dir")),
        ("dir/file", None),
        ("unrelated/file", None),
    ])
    .unwrap();

    assert_eq!(
        index.required_paths("assets/pkg").unwrap(),
        ["assets/pkg", "package.json", "payload.json"]
    );
    assert_eq!(
        index.required_paths("assets/repeated").unwrap(),
        ["assets/repeated", "alias", "dir", "dir/file"]
    );
    index.validate_symlink("assets/repeated").unwrap();
    assert_eq!(index.resolved_path("assets/repeated").unwrap(), "dir/file");
}

#[test]
fn required_paths_retain_directory_witness_before_parent_traversal() {
    let index = SourceArchivePathIndex::from_entries([
        ("link", Some("unused/../real")),
        ("unused/witness", None),
        ("real", None),
    ])
    .unwrap();
    assert_eq!(
        index.required_paths("link").unwrap(),
        ["link", "unused", "real"]
    );

    let pruned =
        SourceArchivePathIndex::from_entries([("link", Some("unused/../real")), ("real", None)])
            .unwrap();
    assert!(matches!(
        pruned.required_paths("link"),
        Err(SourcePathGraphError::MissingTarget(_))
    ));
    assert!(matches!(
        index.required_paths("real"),
        Err(SourcePathGraphError::MissingTarget(_))
    ));
}

#[test]
fn filesystem_index_preserves_unix_names_and_empty_directories() {
    let entries = [
        ("tree/back\\slash", None),
        ("tree/link", Some("empty/../back\\slash")),
    ];
    let index = SourceArchivePathIndex::from_filesystem_entries(entries, ["tree/empty"]).unwrap();
    assert_eq!(
        index.resolved_path("tree/link").unwrap(),
        "tree/back\\slash"
    );
    assert_eq!(
        index.required_paths("tree/link").unwrap(),
        ["tree/link", "tree/empty", "tree", "tree/back\\slash"]
    );
    assert!(matches!(
        SourceArchivePathIndex::from_entries(entries),
        Err(SourcePathGraphError::InvalidPath(_))
    ));
    let archive = SourceArchivePathIndex::from_entries([
        ("tree/file", None),
        ("tree/link", Some("back\\slash")),
    ])
    .unwrap();
    assert!(matches!(
        archive.validate_symlink("tree/link"),
        Err(SourcePathGraphError::UnsafeTarget(_))
    ));
}

#[test]
fn filesystem_index_rejects_noncanonical_entry_and_directory_paths() {
    for path in [
        "tree/bad\0name",
        "tree//file",
        "tree/./file",
        "tree/../file",
    ] {
        assert!(
            matches!(
                SourceArchivePathIndex::from_filesystem_entries([(path, None)], []),
                Err(SourcePathGraphError::InvalidPath(_))
            ),
            "entry {path:?}"
        );
        assert!(
            matches!(
                SourceArchivePathIndex::from_filesystem_entries([], [path]),
                Err(SourcePathGraphError::InvalidPath(_))
            ),
            "directory {path:?}"
        );
    }
}

#[test]
fn physical_namespace_boundary_prevents_reentry_without_rejecting_directory_aliases() {
    let index = SourceArchivePathIndex::from_filesystem_entries(
        [
            ("tree/real", None),
            ("tree/alias", Some("directory")),
            ("tree/link", Some("alias/../alias/../real")),
            ("tree/reenter", Some("../tree/real")),
        ],
        ["tree/directory"],
    )
    .unwrap();
    assert_eq!(
        index.resolve_symlink_within("tree/link", "tree").unwrap().1,
        "tree/real"
    );
    assert_eq!(index.resolved_path("tree/reenter").unwrap(), "tree/real");
    assert!(matches!(
        index.resolve_symlink_within("tree/reenter", "tree"),
        Err(SourcePathGraphError::UnsafeTarget(_))
    ));
    for boundary in ["", "tr", "other", "tree/directory"] {
        assert!(matches!(
            index.resolve_symlink_within("tree/link", boundary),
            Err(SourcePathGraphError::UnsafeTarget(_))
        ));
    }
}
