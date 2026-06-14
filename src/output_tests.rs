use super::output::{FRAME_SENTINEL, Phase, cap_message, render_frame};

#[test]
fn phase_as_str_all_variants() {
    assert_eq!(Phase::Auth.as_str(), "auth");
    assert_eq!(Phase::Build.as_str(), "build");
    assert_eq!(Phase::Db.as_str(), "db");
    assert_eq!(Phase::Deploy.as_str(), "deploy");
    assert_eq!(Phase::Detect.as_str(), "detect");
    assert_eq!(Phase::Domains.as_str(), "domains");
    assert_eq!(Phase::Env.as_str(), "env");
    assert_eq!(Phase::Init.as_str(), "init");
    assert_eq!(Phase::Install.as_str(), "install");
    assert_eq!(Phase::Link.as_str(), "link");
    assert_eq!(Phase::Projects.as_str(), "projects");
    assert_eq!(Phase::Rollback.as_str(), "rollback");
    assert_eq!(Phase::Workspace.as_str(), "workspace");
}

#[test]
fn phase_display_matches_as_str() {
    let phases = [
        Phase::Auth,
        Phase::Build,
        Phase::Db,
        Phase::Deploy,
        Phase::Detect,
        Phase::Domains,
        Phase::Env,
        Phase::Init,
        Phase::Install,
        Phase::Link,
        Phase::Projects,
        Phase::Rollback,
        Phase::Workspace,
    ];
    for p in phases {
        assert_eq!(format!("{p}"), p.as_str());
    }
}

#[test]
fn render_frame_is_sentinel_prefixed_single_line() {
    let frame =
        render_frame(&serde_json::json!({"s": "user", "p": "build", "l": "info", "m": "hi"}));

    // Exactly one line, sentinel-prefixed, newline-terminated.
    assert!(frame.starts_with(FRAME_SENTINEL));
    assert!(frame.ends_with('\n'));
    assert_eq!(frame.matches('\n').count(), 1);

    // Body (sentinel and newline stripped) is the intact JSON object.
    let body = frame
        .strip_prefix(FRAME_SENTINEL)
        .unwrap()
        .strip_suffix('\n')
        .unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(body).expect("frame body must be valid JSON");
    assert_eq!(parsed["s"], "user");
    assert_eq!(parsed["m"], "hi");
}

#[test]
fn cap_message_leaves_short_messages_untouched() {
    let s = "a short build log line";
    assert!(matches!(cap_message(s), std::borrow::Cow::Borrowed(_)));
    assert_eq!(cap_message(s), s);
}

// Frame must stay under the container-runtime log-line split (~16 KiB), proven
// on the *serialized* frame for the worst-case escaping inputs below.
const FRAME_SPLIT_LIMIT: usize = 16 * 1024;

fn frame_len_for(msg: &str) -> usize {
    let capped = cap_message(msg);
    let frame =
        render_frame(&serde_json::json!({"s": "user", "p": "build", "l": "info", "m": capped}));
    assert_eq!(
        frame.matches('\n').count(),
        1,
        "frame must be a single line"
    );
    frame.len()
}

#[test]
fn cap_message_keeps_plain_oversized_lines_under_the_split() {
    let huge = "x".repeat(64 * 1024);
    assert!(cap_message(&huge).ends_with("…[truncated]"));
    assert!(frame_len_for(&huge) < FRAME_SPLIT_LIMIT);
}

#[test]
fn cap_message_bounds_serialized_size_for_escape_heavy_input() {
    // Control bytes JSON-escape to a 6-byte \u00XX sequence — a raw-byte
    // cap would let the serialized frame exceed 16 KiB; the escaped-size cap must not.
    let control_heavy = "\u{1f}".repeat(64 * 1024);
    assert!(frame_len_for(&control_heavy) < FRAME_SPLIT_LIMIT);

    // Quotes and backslashes (2x escape) — common in JSON-in-logs.
    let quote_heavy = "\"\\".repeat(32 * 1024);
    assert!(frame_len_for(&quote_heavy) < FRAME_SPLIT_LIMIT);

    // A typical ANSI-colored build line repeated.
    let ansi = "\u{1b}[31merror\u{1b}[0m something failed ".repeat(2048);
    assert!(frame_len_for(&ansi) < FRAME_SPLIT_LIMIT);
}

#[test]
fn cap_message_truncates_on_a_char_boundary() {
    // Multi-byte chars right at the cut must not panic or produce invalid UTF-8.
    let huge = "é".repeat(16 * 1024); // 2 bytes each, well over the cap
    let capped = cap_message(&huge);
    assert!(capped.ends_with("…[truncated]"));
    assert!(frame_len_for(&huge) < FRAME_SPLIT_LIMIT);
}

#[test]
fn cap_message_leaves_borderline_messages_intact() {
    // A message whose escaped form is within budget is returned untouched.
    let ok = "a normal log line".repeat(100); // ~1.7 KiB, no escaping
    assert!(matches!(cap_message(&ok), std::borrow::Cow::Borrowed(_)));
}

#[test]
fn render_frame_embeds_newlines_in_json_without_breaking_framing() {
    // A message containing newlines must stay a single wire line — serde escapes
    // them inside the JSON string, so the frame still has exactly one '\n'.
    let frame = render_frame(
        &serde_json::json!({"s": "user", "p": "build", "l": "info", "m": "line1\nline2"}),
    );
    assert_eq!(frame.matches('\n').count(), 1);
    assert!(frame.ends_with('\n'));
}
