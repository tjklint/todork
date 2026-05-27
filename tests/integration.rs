use assert_cmd::Command;
use predicates::str::contains;

fn todork() -> Command {
    Command::cargo_bin("todork").expect("binary should be present")
}

#[test]
fn help_exits_zero() {
    todork().arg("--help").assert().success();
}

#[test]
fn help_mentions_todork() {
    todork()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("todork"));
}

#[test]
fn version_exits_zero() {
    todork().arg("--version").assert().success();
}

#[test]
fn version_contains_package_version() {
    todork()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn no_args_does_not_crash() {
    // With no real scanner yet, scanning "." returns NotFound (1).
    // This test asserts the binary runs without a fatal error (exit 0 or 1).
    let output = todork().output().expect("binary should run");
    let code = output.status.code().unwrap_or(2);
    assert!(code < 2, "expected exit code 0 or 1, got {code}");
}
