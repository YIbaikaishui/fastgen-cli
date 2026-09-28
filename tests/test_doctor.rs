//! CLI-level tests for `fastgen doctor`, running the real binary so exit codes
//! are covered.

use std::process::Command;

use fastgen_cli::cli::run_with;
use fastgen_cli::generators::project::generate_project;

fn fastgen() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fastgen"))
}

fn scaffold(dir: &std::path::Path) {
    generate_project("demo", dir, None, "", "0.1.0", false, false).unwrap();
}

#[test]
fn clean_project_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    scaffold(tmp.path());
    let out = fastgen()
        .args(["doctor", "--dir", tmp.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("clean"));
}

#[test]
fn stale_registry_entry_exits_one() {
    let tmp = tempfile::tempdir().unwrap();
    scaffold(tmp.path());
    // Register a module via the library, then delete its directory.
    run_with([
        "fastgen",
        "make",
        "module",
        "user",
        "--dir",
        tmp.path().to_str().unwrap(),
    ])
    .unwrap();
    std::fs::remove_dir_all(tmp.path().join("src").join("modules").join("user")).unwrap();

    let out = fastgen()
        .args(["doctor", "--dir", tmp.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "errors must fail the command");
    assert!(String::from_utf8_lossy(&out.stdout).contains("stale-registry-entry"));
}

#[test]
fn fix_repairs_and_second_run_is_clean() {
    let tmp = tempfile::tempdir().unwrap();
    scaffold(tmp.path());
    run_with([
        "fastgen",
        "make",
        "module",
        "user",
        "--dir",
        tmp.path().to_str().unwrap(),
    ])
    .unwrap();
    std::fs::remove_dir_all(tmp.path().join("src").join("modules").join("user")).unwrap();

    let fixed = fastgen()
        .args(["doctor", "--dir", tmp.path().to_str().unwrap(), "--fix"])
        .output()
        .unwrap();
    assert!(fixed.status.success());
    assert!(String::from_utf8_lossy(&fixed.stdout).contains("[fixed]"));

    let after = fastgen()
        .args(["doctor", "--dir", tmp.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(after.status.success());
    assert!(String::from_utf8_lossy(&after.stdout).contains("clean"));
}

#[test]
fn json_output_is_machine_readable() {
    let tmp = tempfile::tempdir().unwrap();
    scaffold(tmp.path());
    let out = fastgen()
        .args(["doctor", "--dir", tmp.path().to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(parsed["errors"], 0);
    assert_eq!(parsed["warnings"], 0);
    assert!(parsed["issues"].as_array().unwrap().is_empty());
}
