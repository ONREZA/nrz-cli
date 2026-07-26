use super::*;

fn asset(name: &str) -> Asset {
    Asset {
        name: name.to_string(),
    }
}

fn complete_assets(platform: &str) -> Vec<Asset> {
    vec![
        asset(&format!("nrz-{platform}.tar.gz")),
        asset(CHECKSUMS_ASSET_NAME),
    ]
}

fn release(tag_name: &str, prerelease: bool, draft: bool, assets: Vec<Asset>) -> ReleaseInfo {
    ReleaseInfo {
        tag_name: tag_name.to_string(),
        prerelease,
        draft,
        assets,
    }
}

#[test]
fn stable_channel_skips_incomplete_latest_release() {
    let releases = vec![
        release("v0.33.0", false, false, vec![]),
        release("v0.32.4", false, false, complete_assets("linux-x64")),
    ];

    let selected =
        select_release_for_channel(&releases, UpgradeChannel::Stable, "linux-x64").unwrap();

    assert_eq!(selected.tag_name, "v0.32.4");
}

#[test]
fn prerelease_channel_requires_matching_channel_and_asset() {
    let releases = vec![
        release("v0.34.0-alpha.0", true, false, complete_assets("linux-x64")),
        release("v0.33.0-beta.1", true, false, complete_assets("linux-x64")),
    ];

    let selected =
        select_release_for_channel(&releases, UpgradeChannel::Beta, "linux-x64").unwrap();

    assert_eq!(selected.tag_name, "v0.33.0-beta.1");
}

#[test]
fn release_selection_ignores_drafts() {
    let releases = vec![
        release("v0.33.0-beta.1", true, true, complete_assets("linux-x64")),
        release("v0.33.0-beta.0", true, false, complete_assets("linux-x64")),
    ];

    let selected =
        select_release_for_channel(&releases, UpgradeChannel::Beta, "linux-x64").unwrap();

    assert_eq!(selected.tag_name, "v0.33.0-beta.0");
}

#[test]
fn release_pages_use_explicit_page_and_max_page_size() {
    assert_eq!(
        releases_url(3),
        "https://api.github.com/repos/onreza/nrz-cli/releases?per_page=100&page=3"
    );
}

#[test]
fn full_release_page_is_not_terminal() {
    let full_page = vec![release("v0.33.0-beta.0", true, false, vec![]); GITHUB_RELEASES_PER_PAGE];
    let partial_page =
        vec![release("v0.33.0-beta.0", true, false, vec![]); GITHUB_RELEASES_PER_PAGE - 1];

    assert!(release_page_may_have_next(&full_page));
    assert!(!release_page_may_have_next(&partial_page));
}

#[test]
fn parse_releases_reads_prerelease_and_assets() {
    let json = serde_json::json!([
        {
            "tag_name": "v0.33.0-beta.0",
            "prerelease": true,
            "draft": false,
            "assets": [
                {
                    "name": "nrz-linux-x64.tar.gz",
                    "browser_download_url": "https://example.com/nrz-linux-x64.tar.gz"
                }
            ]
        }
    ]);

    let releases = parse_releases(&json).unwrap();

    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].tag_name, "v0.33.0-beta.0");
    assert!(releases[0].prerelease);
    assert_eq!(releases[0].assets[0].name, "nrz-linux-x64.tar.gz");
}

#[test]
fn normalize_tag_accepts_plain_or_prefixed_versions() {
    assert_eq!(normalize_tag("0.33.0-beta.0"), "v0.33.0-beta.0");
    assert_eq!(normalize_tag("v0.33.0-beta.0"), "v0.33.0-beta.0");
}

#[test]
fn upgrade_result_uses_machine_readable_field_names() {
    let result = UpgradeResult {
        current_version: "0.36.2",
        target_version: "0.37.0",
        release: "v0.37.0",
        platform: "linux-x64",
        channel: Some("stable"),
        updated: true,
        asset: Some("nrz-linux-x64.tar.gz"),
    };

    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::json!({
            "currentVersion": "0.36.2",
            "targetVersion": "0.37.0",
            "release": "v0.37.0",
            "platform": "linux-x64",
            "channel": "stable",
            "updated": true,
            "asset": "nrz-linux-x64.tar.gz"
        })
    );
}

#[test]
fn release_selection_requires_checksum_manifest() {
    let releases = vec![release(
        "v0.33.0",
        false,
        false,
        vec![asset("nrz-linux-x64.tar.gz")],
    )];

    assert!(select_release_for_channel(&releases, UpgradeChannel::Stable, "linux-x64").is_none());
}

#[test]
fn release_asset_urls_are_bound_to_the_repository_and_tag() {
    assert_eq!(
        release_asset_url("v0.36.2", "nrz-linux-x64.tar.gz"),
        "https://github.com/onreza/nrz-cli/releases/download/v0%2E36%2E2/nrz%2Dlinux%2Dx64%2Etar%2Egz"
    );
}

#[test]
fn release_checksum_must_match_downloaded_archive() {
    let archive = b"trusted release bytes";
    let digest = sha256_hex(archive);
    let checksums = format!("{digest}  nrz-linux-x64.tar.gz\n");

    verify_release_checksum("nrz-linux-x64.tar.gz", archive, checksums.as_bytes()).unwrap();

    let err = verify_release_checksum(
        "nrz-linux-x64.tar.gz",
        b"tampered release bytes",
        checksums.as_bytes(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("Checksum verification failed"));
}

#[cfg(unix)]
#[test]
fn binary_replacement_does_not_follow_legacy_temp_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let current = dir.path().join("nrz");
    let outside = dir.path().join("outside");
    std::fs::write(&current, b"old").unwrap();
    std::fs::write(&outside, b"outside").unwrap();
    std::os::unix::fs::symlink(&outside, current.with_extension("tmp")).unwrap();

    replace_binary_at(&current, b"new").unwrap();

    assert_eq!(std::fs::read(&current).unwrap(), b"new");
    assert_eq!(std::fs::read(&outside).unwrap(), b"outside");
    assert!(current.with_extension("tmp").is_symlink());
    assert!(
        std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .all(|entry| !entry.file_name().to_string_lossy().contains(".update-"))
    );
}

#[test]
fn cleanup_removes_only_stale_owned_uuid_update_candidates() {
    let dir = tempfile::tempdir().unwrap();
    let current = dir.path().join("nrz");
    let candidate = dir
        .path()
        .join(".nrz.update-019fa73f4cc87c70bcd49d050b4b0380.tmp");
    let fresh_candidate = dir
        .path()
        .join(".nrz.update-019fa73f4cc87c70bcd49d050b4b0382.tmp");
    let unrelated = dir.path().join(".nrz.update-not-a-uuid.tmp");
    let candidate_directory = dir
        .path()
        .join(".nrz.update-019fa73f4cc87c70bcd49d050b4b0381.tmp");
    std::fs::write(&current, b"current").unwrap();
    std::fs::write(&candidate, b"partial").unwrap();
    std::fs::write(&fresh_candidate, b"active").unwrap();
    std::fs::write(&unrelated, b"keep").unwrap();
    std::fs::create_dir(&candidate_directory).unwrap();
    let candidate_file = std::fs::File::options()
        .write(true)
        .open(&candidate)
        .unwrap();
    candidate_file
        .set_times(std::fs::FileTimes::new().set_modified(
            std::time::SystemTime::now()
                - UPDATE_DEBRIS_STALE_AFTER
                - std::time::Duration::from_secs(1),
        ))
        .unwrap();

    cleanup_old_files_at(&current);

    assert!(!candidate.exists());
    assert_eq!(std::fs::read(&fresh_candidate).unwrap(), b"active");
    assert_eq!(std::fs::read(&unrelated).unwrap(), b"keep");
    assert!(candidate_directory.is_dir());
}
