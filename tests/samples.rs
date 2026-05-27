/// Tests that verify the sample fixture files exist and contain exactly the
/// expected number of each annotation tag.  These counts are the contract
/// that later scanner tests depend on — if a sample file is edited, update
/// the counts here too.
use std::fs;
use std::path::Path;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Count occurrences of `needle` as a whole word in `haystack` (case-sensitive).
///
/// Uses `\bNEEDLE\b` so that e.g. "DEBUG" does not count as a "BUG" match.
/// This mirrors how the real todork scanner treats annotation tags.
fn count_in(haystack: &str, needle: &str) -> usize {
    // Build the pattern once per call — acceptable in tests.
    let pat = format!(r"\b{}\b", regex::escape(needle));
    regex::Regex::new(&pat)
        .expect("pattern should be valid")
        .find_iter(haystack)
        .count()
}

fn read_sample(rel_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel_path);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()))
}

// ── file existence ────────────────────────────────────────────────────────────

#[test]
fn sample_python_app_exists() {
    assert!(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/python/app.py")
        .exists());
}

#[test]
fn sample_python_utils_exists() {
    assert!(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/python/utils.py")
        .exists());
}

#[test]
fn sample_python_config_exists() {
    assert!(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/python/config.py")
        .exists());
}

#[test]
fn sample_node_index_exists() {
    assert!(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/node/index.js")
        .exists());
}

#[test]
fn sample_node_server_exists() {
    assert!(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/node/server.js")
        .exists());
}

#[test]
fn sample_node_helpers_exists() {
    assert!(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/node/helpers.ts")
        .exists());
}

// ── annotation counts: samples/python/app.py ─────────────────────────────────

#[test]
fn app_py_todo_count() {
    assert_eq!(count_in(&read_sample("samples/python/app.py"), "TODO"), 1);
}

#[test]
fn app_py_fixme_count() {
    assert_eq!(count_in(&read_sample("samples/python/app.py"), "FIXME"), 1);
}

#[test]
fn app_py_hack_count() {
    assert_eq!(count_in(&read_sample("samples/python/app.py"), "HACK"), 1);
}

#[test]
fn app_py_note_count() {
    assert_eq!(count_in(&read_sample("samples/python/app.py"), "NOTE"), 1);
}

// ── annotation counts: samples/python/utils.py ───────────────────────────────

#[test]
fn utils_py_todo_count() {
    // Includes "TODO(alice):" — one occurrence
    assert_eq!(count_in(&read_sample("samples/python/utils.py"), "TODO"), 1);
}

#[test]
fn utils_py_optimize_count() {
    assert_eq!(
        count_in(&read_sample("samples/python/utils.py"), "OPTIMIZE"),
        1
    );
}

#[test]
fn utils_py_deprecated_count() {
    assert_eq!(
        count_in(&read_sample("samples/python/utils.py"), "DEPRECATED"),
        1
    );
}

// ── annotation counts: samples/python/config.py ──────────────────────────────

#[test]
fn config_py_xxx_count() {
    assert_eq!(count_in(&read_sample("samples/python/config.py"), "XXX"), 1);
}

#[test]
fn config_py_bug_count() {
    assert_eq!(count_in(&read_sample("samples/python/config.py"), "BUG"), 1);
}

// ── annotation counts: samples/node/index.js ─────────────────────────────────

#[test]
fn index_js_todo_count() {
    assert_eq!(count_in(&read_sample("samples/node/index.js"), "TODO"), 1);
}

#[test]
fn index_js_fixme_count() {
    assert_eq!(count_in(&read_sample("samples/node/index.js"), "FIXME"), 1);
}

#[test]
fn index_js_hack_count() {
    // Includes "HACK(bob):" — one occurrence
    assert_eq!(count_in(&read_sample("samples/node/index.js"), "HACK"), 1);
}

// ── annotation counts: samples/node/server.js ────────────────────────────────

#[test]
fn server_js_note_count() {
    assert_eq!(count_in(&read_sample("samples/node/server.js"), "NOTE"), 1);
}

#[test]
fn server_js_todo_count() {
    assert_eq!(count_in(&read_sample("samples/node/server.js"), "TODO"), 1);
}

// ── annotation counts: samples/node/helpers.ts ───────────────────────────────

#[test]
fn helpers_ts_optimize_count() {
    assert_eq!(
        count_in(&read_sample("samples/node/helpers.ts"), "OPTIMIZE"),
        1
    );
}

#[test]
fn helpers_ts_xxx_count() {
    assert_eq!(count_in(&read_sample("samples/node/helpers.ts"), "XXX"), 1);
}

#[test]
fn helpers_ts_todo_count() {
    // Block comment TODO — still one occurrence
    assert_eq!(count_in(&read_sample("samples/node/helpers.ts"), "TODO"), 1);
}

// ── file existence: samples/zig ───────────────────────────────────────────────

#[test]
fn sample_zig_main_exists() {
    assert!(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/zig/main.zig")
        .exists());
}

#[test]
fn sample_zig_http_exists() {
    assert!(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/zig/http.zig")
        .exists());
}

// ── file existence: samples/qsharp ────────────────────────────────────────────

#[test]
fn sample_qsharp_grover_exists() {
    assert!(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/qsharp/Grover.qs")
        .exists());
}

#[test]
fn sample_qsharp_teleport_exists() {
    assert!(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/qsharp/Teleport.qs")
        .exists());
}

// ── file existence: samples/lua ───────────────────────────────────────────────

#[test]
fn sample_lua_router_exists() {
    assert!(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/lua/router.lua")
        .exists());
}

#[test]
fn sample_lua_cache_exists() {
    assert!(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("samples/lua/cache.lua")
        .exists());
}

// ── annotation counts: samples/zig/main.zig ──────────────────────────────────

#[test]
fn main_zig_note_count() {
    assert_eq!(count_in(&read_sample("samples/zig/main.zig"), "NOTE"), 1);
}

#[test]
fn main_zig_todo_count() {
    assert_eq!(count_in(&read_sample("samples/zig/main.zig"), "TODO"), 1);
}

#[test]
fn main_zig_hack_count() {
    assert_eq!(count_in(&read_sample("samples/zig/main.zig"), "HACK"), 1);
}

#[test]
fn main_zig_fixme_count() {
    assert_eq!(count_in(&read_sample("samples/zig/main.zig"), "FIXME"), 1);
}

// ── annotation counts: samples/zig/http.zig ──────────────────────────────────

#[test]
fn http_zig_todo_count() {
    assert_eq!(count_in(&read_sample("samples/zig/http.zig"), "TODO"), 1);
}

#[test]
fn http_zig_xxx_count() {
    assert_eq!(count_in(&read_sample("samples/zig/http.zig"), "XXX"), 1);
}

#[test]
fn http_zig_optimize_count() {
    assert_eq!(
        count_in(&read_sample("samples/zig/http.zig"), "OPTIMIZE"),
        1
    );
}

#[test]
fn http_zig_hack_count() {
    assert_eq!(count_in(&read_sample("samples/zig/http.zig"), "HACK"), 1);
}

// ── annotation counts: samples/qsharp/Grover.qs ──────────────────────────────

#[test]
fn grover_qs_note_count() {
    assert_eq!(
        count_in(&read_sample("samples/qsharp/Grover.qs"), "NOTE"),
        1
    );
}

#[test]
fn grover_qs_todo_count() {
    assert_eq!(
        count_in(&read_sample("samples/qsharp/Grover.qs"), "TODO"),
        1
    );
}

#[test]
fn grover_qs_fixme_count() {
    assert_eq!(
        count_in(&read_sample("samples/qsharp/Grover.qs"), "FIXME"),
        1
    );
}

#[test]
fn grover_qs_hack_count() {
    assert_eq!(
        count_in(&read_sample("samples/qsharp/Grover.qs"), "HACK"),
        1
    );
}

#[test]
fn grover_qs_xxx_count() {
    assert_eq!(count_in(&read_sample("samples/qsharp/Grover.qs"), "XXX"), 1);
}

// ── annotation counts: samples/qsharp/Teleport.qs ────────────────────────────

#[test]
fn teleport_qs_note_count() {
    assert_eq!(
        count_in(&read_sample("samples/qsharp/Teleport.qs"), "NOTE"),
        1
    );
}

#[test]
fn teleport_qs_todo_count() {
    assert_eq!(
        count_in(&read_sample("samples/qsharp/Teleport.qs"), "TODO"),
        1
    );
}

#[test]
fn teleport_qs_optimize_count() {
    assert_eq!(
        count_in(&read_sample("samples/qsharp/Teleport.qs"), "OPTIMIZE"),
        1
    );
}

#[test]
fn teleport_qs_fixme_count() {
    assert_eq!(
        count_in(&read_sample("samples/qsharp/Teleport.qs"), "FIXME"),
        1
    );
}

#[test]
fn teleport_qs_deprecated_count() {
    assert_eq!(
        count_in(&read_sample("samples/qsharp/Teleport.qs"), "DEPRECATED"),
        1
    );
}

// ── annotation counts: samples/lua/router.lua ────────────────────────────────

#[test]
fn router_lua_note_count() {
    assert_eq!(count_in(&read_sample("samples/lua/router.lua"), "NOTE"), 1);
}

#[test]
fn router_lua_todo_count() {
    assert_eq!(count_in(&read_sample("samples/lua/router.lua"), "TODO"), 1);
}

#[test]
fn router_lua_fixme_count() {
    assert_eq!(count_in(&read_sample("samples/lua/router.lua"), "FIXME"), 1);
}

// ── annotation counts: samples/lua/cache.lua ─────────────────────────────────

#[test]
fn cache_lua_hack_count() {
    assert_eq!(count_in(&read_sample("samples/lua/cache.lua"), "HACK"), 1);
}

#[test]
fn cache_lua_bug_count() {
    assert_eq!(count_in(&read_sample("samples/lua/cache.lua"), "BUG"), 1);
}

#[test]
fn cache_lua_optimize_count() {
    assert_eq!(
        count_in(&read_sample("samples/lua/cache.lua"), "OPTIMIZE"),
        1
    );
}

// ── cross-file totals ─────────────────────────────────────────────────────────

#[test]
fn total_zig_annotations() {
    let files = ["samples/zig/main.zig", "samples/zig/http.zig"];
    let combined: String = files.iter().map(|f| read_sample(f)).collect();
    // main: NOTE+TODO+HACK+FIXME=4, http: TODO+XXX+OPTIMIZE+HACK=4  ->  8
    let tags = ["TODO", "FIXME", "HACK", "NOTE", "OPTIMIZE", "XXX"];
    let total: usize = tags.iter().map(|t| count_in(&combined, t)).sum();
    assert_eq!(total, 8);
}

#[test]
fn total_qsharp_annotations() {
    let files = ["samples/qsharp/Grover.qs", "samples/qsharp/Teleport.qs"];
    let combined: String = files.iter().map(|f| read_sample(f)).collect();
    // Grover: NOTE+TODO+FIXME+HACK+XXX=5, Teleport: NOTE+TODO+OPTIMIZE+FIXME+DEPRECATED=5  ->  10
    let tags = [
        "TODO",
        "FIXME",
        "HACK",
        "NOTE",
        "OPTIMIZE",
        "DEPRECATED",
        "XXX",
    ];
    let total: usize = tags.iter().map(|t| count_in(&combined, t)).sum();
    assert_eq!(total, 10);
}

#[test]
fn total_lua_annotations() {
    let files = ["samples/lua/router.lua", "samples/lua/cache.lua"];
    let combined: String = files.iter().map(|f| read_sample(f)).collect();
    // router: NOTE+TODO+FIXME=3, cache: HACK+BUG+OPTIMIZE=3  ->  6
    let tags = ["TODO", "FIXME", "HACK", "NOTE", "OPTIMIZE", "BUG"];
    let total: usize = tags.iter().map(|t| count_in(&combined, t)).sum();
    assert_eq!(total, 6);
}

#[test]
fn total_python_annotations() {
    let files = [
        "samples/python/app.py",
        "samples/python/utils.py",
        "samples/python/config.py",
    ];
    let combined: String = files.iter().map(|f| read_sample(f)).collect();
    // app: TODO+FIXME+HACK+NOTE=4, utils: TODO+OPTIMIZE+DEPRECATED=3, config: XXX+BUG=2  →  9
    let tags = [
        "TODO",
        "FIXME",
        "HACK",
        "NOTE",
        "OPTIMIZE",
        "DEPRECATED",
        "XXX",
        "BUG",
    ];
    let total: usize = tags.iter().map(|t| count_in(&combined, t)).sum();
    assert_eq!(total, 9);
}

#[test]
fn total_node_annotations() {
    let files = [
        "samples/node/index.js",
        "samples/node/server.js",
        "samples/node/helpers.ts",
    ];
    let combined: String = files.iter().map(|f| read_sample(f)).collect();
    // index: TODO+FIXME+HACK=3, server: NOTE+TODO=2, helpers: OPTIMIZE+XXX+TODO=3  →  8
    let tags = ["TODO", "FIXME", "HACK", "NOTE", "OPTIMIZE", "XXX"];
    let total: usize = tags.iter().map(|t| count_in(&combined, t)).sum();
    assert_eq!(total, 8);
}

#[test]
fn grand_total_annotations() {
    let files = [
        "samples/python/app.py",
        "samples/python/utils.py",
        "samples/python/config.py",
        "samples/node/index.js",
        "samples/node/server.js",
        "samples/node/helpers.ts",
        "samples/zig/main.zig",
        "samples/zig/http.zig",
        "samples/qsharp/Grover.qs",
        "samples/qsharp/Teleport.qs",
        "samples/lua/router.lua",
        "samples/lua/cache.lua",
    ];
    let combined: String = files.iter().map(|f| read_sample(f)).collect();
    let tags = [
        "TODO",
        "FIXME",
        "HACK",
        "NOTE",
        "OPTIMIZE",
        "DEPRECATED",
        "XXX",
        "BUG",
    ];
    let total: usize = tags.iter().map(|t| count_in(&combined, t)).sum();
    // 9 python + 8 node + 8 zig + 10 qsharp + 6 lua = 41 total
    assert_eq!(total, 41);
}
