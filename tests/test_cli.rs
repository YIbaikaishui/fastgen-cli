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

#[test]
fn test_new_clones_custom_layout_repo() {
    // A fake layout repo on disk; git clone works with plain paths.
    let layout = tempfile::tempdir().unwrap();
    std::fs::write(
        layout.path().join("pyproject.toml"),
        "[project]\nname = \"starter\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(layout.path().join("src").join("modules")).unwrap();
    std::fs::write(layout.path().join("src").join("main.py"), "app = None\n").unwrap();
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["add", "-A"],
        vec![
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "-m",
            "l",
        ],
    ] {
        assert!(std::process::Command::new("git")
            .args(&args)
            .current_dir(layout.path())
            .status()
            .unwrap()
            .success());
    }

    let tmp = tempfile::tempdir().unwrap();
    run_with([
        "fastgen",
        "new",
        "my-app",
        "--dir",
        path_str(tmp.path()),
        "--layout",
        layout.path().to_str().unwrap(),
    ])
    .unwrap();

    let root = tmp.path().join("my-app");
    assert!(root.join("src").join("main.py").exists());
    assert!(
        !root.join(".git").exists(),
        "layout history must not be kept"
    );
    assert!(root.join(".fastgen.json").exists());
    let pyproject = std::fs::read_to_string(root.join("pyproject.toml")).unwrap();
    assert!(pyproject.contains("name = \"my_app\""));
}
