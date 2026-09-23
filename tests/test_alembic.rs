//! Tests for Alembic migration scaffolding.

use std::path::Path;

use fastgen_cli::agent::AgentSelection;
use fastgen_cli::cli::run_with_mode;

fn scaffold(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    run_with_mode(
        [
            "fastgen",
            "new",
            "demo",
            "--dir",
            tmp.path().to_str().unwrap(),
        ],
        AgentSelection::Skip,
    )
    .unwrap();
    tmp.path().join("demo")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn test_new_project_includes_alembic_scaffold() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(&tmp);
    assert!(root.join("alembic.ini").exists());
    assert!(root.join("migrations").join("env.py").exists());
    assert!(root.join("migrations").join("script.py.mako").exists());
    assert!(root
        .join("migrations")
        .join("versions")
        .join("0001_initial.py")
        .exists());
}

#[test]
fn test_new_project_declares_alembic_dependency() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(&tmp);
    assert!(read(&root.join("pyproject.toml")).contains("alembic"));
}

#[test]
fn test_env_py_wires_settings_and_registry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(&tmp);
    let env = read(&root.join("migrations").join("env.py"));
    assert!(env.contains("from src.core.config import settings"));
    assert!(env.contains("config.set_main_option(\"sqlalchemy.url\", settings.database_url)"));
    assert!(env.contains("importlib.import_module"));
    assert!(env.contains("async_engine_from_config"));
}

#[test]
fn test_generated_main_defers_schema_to_alembic() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(&tmp);
    let main = read(&root.join("src").join("main.py"));
    assert!(!main.contains("create_all"));
    assert!(main.contains("alembic upgrade head"));
}

#[test]
fn test_init_alembic_adds_to_legacy_app_layout() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("legacy");
    std::fs::create_dir_all(root.join("app").join("modules")).unwrap();
    std::fs::write(
        root.join("app").join("main.py"),
        "from fastapi import FastAPI\n\napp = FastAPI()\n",
    )
    .unwrap();
    run_with_mode(
        [
            "fastgen",
            "init",
            "alembic",
            "--dir",
            root.to_str().unwrap(),
        ],
        AgentSelection::Skip,
    )
    .unwrap();
    assert!(root.join("alembic.ini").exists());
    assert!(root.join("migrations").join("env.py").exists());
    let env = read(&root.join("migrations").join("env.py"));
    assert!(env.contains("from app.core.config import settings"));
    assert!(env.contains("from app.modules import modules"));
}

#[test]
fn test_init_alembic_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(&tmp);
    run_with_mode(
        [
            "fastgen",
            "init",
            "alembic",
            "--dir",
            root.to_str().unwrap(),
        ],
        AgentSelection::Skip,
    )
    .unwrap();
    let alembic_ini = read(&root.join("alembic.ini"));
    run_with_mode(
        [
            "fastgen",
            "init",
            "alembic",
            "--dir",
            root.to_str().unwrap(),
        ],
        AgentSelection::Skip,
    )
    .unwrap();
    assert_eq!(read(&root.join("alembic.ini")), alembic_ini);
}
