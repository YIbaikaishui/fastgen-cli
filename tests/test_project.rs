//! Tests for the ``fastgen new`` project scaffold and src/app layout support.

use std::path::Path;

use fastgen_cli::generators::{
    core::generate_core,
    module::generate_module,
    project::generate_project,
    registry::{list_registered, read_registry, register_module},
};
use fastgen_cli::layout::{read_config, source_dir_name};

fn proj() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    generate_project("my_app", tmp.path(), None, "", "0.1.0", false, false).unwrap();
    tmp
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn test_new_creates_best_practice_layout() {
    let tmp = proj();
    let expected = [
        ".env",
        ".env.example",
        ".gitignore",
        ".python-version",
        ".fastgen.json",
        "pyproject.toml",
        "README.md",
        "src/__init__.py",
        "src/main.py",
        "src/core/__init__.py",
        "src/core/config.py",
        "src/core/database.py",
        "src/modules/__init__.py",
        "tests/__init__.py",
        "tests/conftest.py",
        "tests/test_health.py",
        "alembic.ini",
        "migrations/env.py",
        "migrations/script.py.mako",
        "migrations/versions/0001_initial.py",
    ];
    for rel in expected {
        assert!(tmp.path().join(rel).exists(), "missing {rel}");
    }
}

#[test]
fn test_new_src_layout_files() {
    let tmp = proj();
    assert!(read(&tmp.path().join(".gitignore")).contains(".env"));
    assert!(read(&tmp.path().join(".env")).starts_with("DATABASE_URL="));
    assert!(read(&tmp.path().join("src").join("main.py"))
        .contains("from src.core.database import engine"));
    assert!(
        read(&tmp.path().join("tests").join("conftest.py")).contains("from src.main import app")
    );
    assert!(read(&tmp.path().join("pyproject.toml")).contains("name = \"my_app\""));
}

#[test]
fn test_generated_async_generators_use_async_generator_annotation() {
    let tmp = proj();
    let main = read(&tmp.path().join("src").join("main.py"));
    assert!(!main.contains("AsyncIterator"));
    assert!(main.contains("async def lifespan(app: FastAPI) -> AsyncGenerator[None, None]"));
    let conftest = read(&tmp.path().join("tests").join("conftest.py"));
    assert!(!conftest.contains("AsyncIterator"));
    assert!(conftest.contains("async def client() -> AsyncGenerator[AsyncClient, None]"));
}

#[test]
fn test_new_writes_layout_config() {
    let tmp = proj();
    let config = read_config(tmp.path());
    assert_eq!(config.get("source_dir").unwrap(), "src");
    assert_eq!(source_dir_name(tmp.path()), "src");
}

#[test]
fn test_new_core_uses_src_imports() {
    let tmp = proj();
    assert!(
        read(&tmp.path().join("src").join("core").join("database.py"))
            .contains("from src.core.config import settings")
    );
}

#[test]
fn test_make_module_in_src_layout() {
    let tmp = proj();
    generate_module("user", tmp.path(), None, false, false).unwrap();
    register_module(tmp.path(), "user", None, false).unwrap();

    let module_dir = tmp.path().join("src").join("modules").join("user");
    assert!(module_dir.join("application").join("schemas.py").exists());
    assert!(module_dir
        .join("application")
        .join("user_service.py")
        .exists());
    assert!(module_dir.join("api").join("router.py").exists());
    assert!(module_dir.join("domain").join("model.py").exists());
    assert!(module_dir.join("domain").join("repository.py").exists());
    assert!(module_dir
        .join("infrastructure")
        .join("user_repository.py")
        .exists());
    assert!(read(&module_dir.join("api").join("router.py"))
        .contains("from src.core.database import get_session"));

    assert_eq!(
        read_registry(tmp.path(), None),
        vec![("user".to_string(), "src.modules.user".to_string())]
    );
    let names: Vec<String> = list_registered(tmp.path(), None)
        .into_iter()
        .map(|(name, _, _)| name)
        .collect();
    assert_eq!(names, vec!["user".to_string()]);
}

#[test]
fn test_make_module_in_app_layout_backwards_compatible() {
    let tmp = tempfile::tempdir().unwrap();
    generate_module("user", tmp.path(), None, false, false).unwrap();
    register_module(tmp.path(), "user", None, false).unwrap();

    assert!(tmp
        .path()
        .join("app")
        .join("modules")
        .join("user")
        .join("api")
        .join("router.py")
        .exists());
    assert_eq!(
        read_registry(tmp.path(), None),
        vec![("user".to_string(), "app.modules.user".to_string())]
    );
    assert_eq!(source_dir_name(tmp.path()), "app");
}

#[test]
fn test_generate_core_skips_existing_code() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("src").join("core")).unwrap();
    std::fs::write(
        tmp.path().join("src").join("core").join("config.py"),
        "# custom config\n",
    )
    .unwrap();
    let files = generate_core(tmp.path(), Some("src"), false).unwrap();
    assert_eq!(
        read(&tmp.path().join("src").join("core").join("config.py")),
        "# custom config\n"
    );
    assert!(!files.iter().any(|f| {
        f.path.file_name() == Some(std::ffi::OsStr::new("config.py"))
            && f.status == fastgen_cli::writers::Status::Created
    }));
    assert!(tmp
        .path()
        .join("src")
        .join("core")
        .join("database.py")
        .exists());
}
