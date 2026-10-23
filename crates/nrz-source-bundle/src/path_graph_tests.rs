use super::{PathWorkBudget, PathWorkError, SourceArchivePathIndex, SourcePathGraphError};

#[test]
fn v1_memory_admission_covers_deep_component_index() {
    let path = format!("{}file", "d/".repeat(1000));
    let index = SourceArchivePathIndex::from_entries([(path.as_str(), None)]).unwrap();
    let body = serde_json::to_vec(&serde_json::json!({
        "files": [{"path": path, "size": 0}]
    }))
    .unwrap();
    let forecast = crate::source_bundle_verification_manifest_memory_bytes(&body).unwrap();
    let index_bytes = index.nodes.capacity() * std::mem::size_of::<super::PathNode<'_>>()
        + index.children.capacity() * std::mem::size_of::<((usize, &str), usize)>();

    assert!(forecast >= index_bytes as u64, "{forecast} < {index_bytes}");
}

#[test]
fn shared_index_working_allowance_covers_actual_growth_and_checks_overflow() {
    for count in [1, 7, 64, 257, 1025] {
        let paths: Vec<_> = (0..count).map(|index| format!("d{index}/file")).collect();
        let index =
            SourceArchivePathIndex::from_entries(paths.iter().map(|path| (path.as_str(), None)))
                .unwrap();
        let forecast =
            SourceArchivePathIndex::working_memory_bytes(index.nodes.len() as u64, 4096).unwrap();
        let node_bytes = index.nodes.capacity() * super::PATH_INDEX_NODE_BYTES;
        let child_bytes = index.children.capacity() * (super::PATH_INDEX_CHILD_BYTES + 1);
        // Retained tables plus an old table during growth and bounded lookup frames.
        assert!(
            forecast
                >= (2 * (node_bytes + child_bytes) + super::SYMLINK_VALIDATION_SCRATCH_BYTES)
                    as u64
        );
    }
    assert!(SourceArchivePathIndex::working_memory_bytes(u64::MAX, 1).is_none());
    assert!(SourceArchivePathIndex::working_memory_bytes(1, u64::MAX).is_none());
}

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
fn endpoint_preparation_reports_revisits_without_custody_paths() {
    let index = SourceArchivePathIndex::from_entries([
        ("d/file", None),
        ("d/self", Some(".")),
        ("d/bounce", Some("self/../d/self")),
    ])
    .unwrap();
    let mut work = PathWorkBudget::new(10_000);
    assert_eq!(
        index
            .resolved_endpoint_with_budget("d/self", &mut work)
            .unwrap(),
        ("d", 1)
    );
    assert_eq!(
        index
            .resolved_endpoint_with_budget("d/bounce", &mut work)
            .unwrap(),
        ("d", 3)
    );
    assert_eq!(index.resolve_symlink("d/bounce").unwrap().1, "d");
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

#[test]
fn deep_paths_preserve_directory_witnesses_without_recursive_traversal() {
    let depth = 65_536;
    let path = format!("{}file", "a/".repeat(depth));
    let index = SourceArchivePathIndex::from_entries([
        (path.as_str(), None),
        ("link", Some(path.as_str())),
    ])
    .unwrap();
    let (required, resolved) = index.resolve_symlink("link").unwrap();
    assert_eq!(resolved, path);
    assert_eq!(required.len(), depth + 2);
    assert_eq!(required[0], "link");
    assert_eq!(required[1], "a");
    assert_eq!(required.last().copied(), Some(path.as_str()));
}

#[test]
fn shared_work_budget_bounds_fan_in_to_a_long_target_with_only_two_expansions() {
    let target = format!("{}file", "dir/".repeat(4096));
    let aliases: Vec<_> = (0..16).map(|number| format!("alias-{number}")).collect();
    let mut entries = vec![(target.as_str(), None), ("shared", Some(target.as_str()))];
    entries.extend(aliases.iter().map(|path| (path.as_str(), Some("shared"))));
    let index = SourceArchivePathIndex::from_entries(entries.iter().copied()).unwrap();
    for alias in &aliases {
        index.validate_symlink(alias).unwrap();
    }

    let mut budget = PathWorkBudget::new(100_000);
    index
        .validate_symlink_with_budget(&aliases[0], &mut budget)
        .unwrap();
    let first_remaining = budget.remaining.unwrap();
    assert!(first_remaining < 100_000 - target.len() as u64);
    assert!(aliases[1..].iter().any(|alias| matches!(
        index.validate_symlink_with_budget(alias, &mut budget),
        Err(PathWorkError::Exhausted)
    )));

    let mut sufficient = PathWorkBudget::new(10_000_000);
    let index = SourceArchivePathIndex::from_entries_with_budget(entries, &mut sufficient).unwrap();
    for alias in &aliases {
        index
            .validate_symlink_with_budget(alias, &mut sufficient)
            .unwrap();
        assert_eq!(index.resolved_path(alias).unwrap(), target);
    }
}

#[test]
fn target_scan_and_path_lookup_are_charged_before_invalid_input_is_examined() {
    let target = format!("{}\\", "./".repeat(8192));
    let index = SourceArchivePathIndex::from_entries([("link", Some(target.as_str()))]).unwrap();
    assert!(matches!(
        index.validate_symlink("link"),
        Err(SourcePathGraphError::UnsafeTarget(_))
    ));
    assert!(matches!(
        index.validate_symlink_with_budget("link", &mut PathWorkBudget::new(100)),
        Err(PathWorkError::Exhausted)
    ));
    assert!(matches!(
        index.validate_symlink_with_budget("link", &mut PathWorkBudget::new(1_000_000)),
        Err(PathWorkError::Invalid(SourcePathGraphError::UnsafeTarget(
            _
        )))
    ));
    let missing = "x/".repeat(8192);
    assert!(matches!(
        index.validate_symlink_with_budget(&missing, &mut PathWorkBudget::new(100)),
        Err(PathWorkError::Exhausted)
    ));
}

#[test]
fn index_construction_uses_the_same_allowance_and_preserves_path_errors() {
    assert!(matches!(
        SourceArchivePathIndex::from_entries_with_budget(
            [("invalid/../file", None)],
            &mut PathWorkBudget::new(1)
        ),
        Err(PathWorkError::Exhausted)
    ));
    assert!(matches!(
        SourceArchivePathIndex::from_entries_with_budget(
            [("invalid/../file", None)],
            &mut PathWorkBudget::new(1000)
        ),
        Err(PathWorkError::Invalid(SourcePathGraphError::InvalidPath(_)))
    ));
    let mut budget = PathWorkBudget::new(1000);
    let index = SourceArchivePathIndex::from_entries_with_budget(
        [("file", None), ("link", Some("file"))],
        &mut budget,
    )
    .unwrap();
    let after_construction = budget.remaining.unwrap();
    assert!(after_construction < 1000);
    index
        .validate_symlink_with_budget("link", &mut budget)
        .unwrap();
    assert!(budget.remaining.unwrap() < after_construction);
}

#[test]
fn budgeted_validation_preserves_native_order_cycles_revisits_and_expansion_limit() {
    let index = SourceArchivePathIndex::from_entries([
        ("dir/file", None),
        ("alias", Some("dir")),
        ("valid", Some("alias/../alias/file")),
        ("file-parent", Some("dir/file/../file")),
        ("escape", Some("../dir/file")),
        ("missing", Some("absent/../dir/file")),
        ("cycle-a", Some("cycle-b")),
        ("cycle-b", Some("cycle-a")),
    ])
    .unwrap();
    for path in ["valid", "file-parent", "escape", "missing", "cycle-a"] {
        let unbudgeted = index.validate_symlink(path);
        let budgeted = index.validate_symlink_with_budget(path, &mut PathWorkBudget::new(100_000));
        match (unbudgeted, budgeted) {
            (Ok(()), Ok(())) => {}
            (Err(expected), Err(PathWorkError::Invalid(actual))) => {
                assert_eq!(actual.to_string(), expected.to_string(), "{path}");
            }
            pair => panic!("different validation result for {path}: {pair:?}"),
        }
    }

    let paths: Vec<_> = (0..41).map(|number| format!("link-{number}")).collect();
    let mut entries = vec![("file", None)];
    for (number, path) in paths.iter().enumerate() {
        entries.push((
            path.as_str(),
            Some(paths.get(number + 1).map_or("file", String::as_str)),
        ));
    }
    let index = SourceArchivePathIndex::from_entries(entries).unwrap();
    index
        .validate_symlink_with_budget(&paths[1], &mut PathWorkBudget::new(100_000))
        .unwrap();
    assert!(matches!(
        index.validate_symlink_with_budget(&paths[0], &mut PathWorkBudget::new(100_000)),
        Err(PathWorkError::Invalid(SourcePathGraphError::UnsafeTarget(
            _
        )))
    ));
}
