//! End-to-end CLI tests.

use fastgen_cli::cli::run_with;
use fastgen_cli::generators::registry::list_registered;

fn path_str(path: &std::path::Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn test_new_command() {
    let tmp = tempfile::tempdir().unwrap();
    run_with(["fastgen", "new", "demo", "--dir", path_str(tmp.path())]).unwrap();
    let root = tmp.path().join("demo");
    assert!(root.join(".env").exists());
    assert!(root.join("src").join("main.py").exists());
    assert!(root.join("tests").join("test_health.py").exists());
    assert!(root.join("src").join("core").join("config.py").exists());
    assert!(root.join(".fastgen.json").exists());
}

#[test]
fn test_new_refuses_non_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("demo");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("keep.txt"), "x").unwrap();
    let err = run_with(["fastgen", "new", "demo", "--dir", path_str(tmp.path())]).unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[test]
fn test_new_then_make_module() {
    let tmp = tempfile::tempdir().unwrap();
    run_with(["fastgen", "new", "demo", "--dir", path_str(tmp.path())]).unwrap();
    let root = tmp.path().join("demo");
    run_with([
        "fastgen",
        "make",
        "module",
        "user",
        "--dir",
        path_str(&root),
    ])
    .unwrap();
    assert!(root
        .join("src")
        .join("modules")
        .join("user")
        .join("api")
        .join("router.py")
        .exists());

    let listed = list_registered(&root, None);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].1, "src.modules.user");
}
