use assert_cmd::Command;
use predicates::prelude::predicate;

/// A `trslat` command locked to English output and no external locale override.
fn cmd() -> Command {
    let mut c = Command::cargo_bin("trslat").unwrap();
    c.env("LANG", "en_US.UTF-8").env_remove("LC_ALL");
    c
}

#[test]
fn shows_help_with_usage() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: trslat"));
}

#[test]
fn shows_version() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("trslat "));
}

#[test]
fn empty_argument_is_rejected() {
    cmd()
        .arg("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty"));
}

#[test]
fn empty_piped_stdin_is_rejected() {
    cmd()
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty"));
}

#[test]
fn unknown_flag_fails() {
    cmd().arg("--bogus").assert().failure();
}