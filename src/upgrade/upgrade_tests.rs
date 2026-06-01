use super::*;

fn asset(name: &str) -> Asset {
    Asset {
        name: name.to_string(),
        browser_download_url: format!("https://example.com/{name}"),
    }
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
        release("v0.32.4", false, false, vec![asset("nrz-linux-x64.tar.gz")]),
    ];

    let selected =
        select_release_for_channel(&releases, UpgradeChannel::Stable, "linux-x64").unwrap();

    assert_eq!(selected.tag_name, "v0.32.4");
}

#[test]
fn prerelease_channel_requires_matching_channel_and_asset() {
    let releases = vec![
        release(
            "v0.34.0-rc.0",
            true,
            false,
            vec![asset("nrz-linux-x64.tar.gz")],
        ),
        release(
            "v0.33.0-beta.1",
            true,
            false,
            vec![asset("nrz-linux-x64.tar.gz")],
        ),
    ];

    let selected =
        select_release_for_channel(&releases, UpgradeChannel::Beta, "linux-x64").unwrap();

    assert_eq!(selected.tag_name, "v0.33.0-beta.1");
}

#[test]
fn release_selection_ignores_drafts() {
    let releases = vec![
        release(
            "v0.33.0-beta.1",
            true,
            true,
            vec![asset("nrz-linux-x64.tar.gz")],
        ),
        release(
            "v0.33.0-beta.0",
            true,
            false,
            vec![asset("nrz-linux-x64.tar.gz")],
        ),
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
