//! Tests for ``fastgen make module`` generated output.

use std::path::Path;

use fastgen_cli::cli::run_with;

fn scaffold(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    run_with([
        "fastgen",
        "new",
        "demo",
        "--dir",
        tmp.path().to_str().unwrap(),
    ])
    .unwrap();
    tmp.path().join("demo")
}

fn make_module(root: &Path, feature: &str) {
    run_with([
        "fastgen",
        "make",
        "module",
        feature,
        "--dir",
        root.to_str().unwrap(),
    ])
    .unwrap();
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn test_module_generates_full_skeleton() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(&tmp);
    make_module(&root, "post");
    let module_dir = root.join("src").join("modules").join("post");
    for name in [
        "__init__.py",
        "domain/__init__.py",
        "domain/model.py",
        "domain/repository.py",
        "application/__init__.py",
        "application/schemas.py",
        "application/post_service.py",
        "infrastructure/__init__.py",
        "infrastructure/post_repository.py",
        "api/__init__.py",
        "api/router.py",
    ] {
        assert!(module_dir.join(name).exists(), "missing {name}");
    }
    assert!(module_dir.join("tests").join("conftest.py").exists());
    assert!(module_dir.join("tests").join("test_post.py").exists());
}

#[test]
fn test_module_schemas_follow_orm_read_convention() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(&tmp);
    make_module(&root, "post");
    let schemas = read(
        &root
            .join("src")
            .join("modules")
            .join("post")
            .join("application")
            .join("schemas.py"),
    );
    assert!(schemas.contains("from_attributes=True"));
    assert!(schemas.contains("class PostCreate"));
    assert!(schemas.contains("class PostRead"));
    assert!(schemas.contains("class PostUpdate"));
}

#[test]
fn test_module_model_generates_entity() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(&tmp);
    make_module(&root, "post");
    let model = read(
        &root
            .join("src")
            .join("modules")
            .join("post")
            .join("domain")
            .join("model.py"),
    );
    assert!(model.contains("class Post(Base)"));
    assert!(model.contains("__tablename__ = \"posts\""));
    assert!(model.contains("mapped_column(primary_key=True)"));
}

#[test]
fn test_module_tests_include_test_db_fixture() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(&tmp);
    make_module(&root, "post");
    let conftest = read(
        &root
            .join("src")
            .join("modules")
            .join("post")
            .join("tests")
            .join("conftest.py"),
    );
    assert!(conftest.contains("dependency_overrides[get_session]"));
    assert!(conftest.contains("StaticPool"));
    assert!(!conftest.contains("AsyncIterator"));
    assert!(conftest.contains("AsyncGenerator[AsyncClient, None]"));
}

#[test]
fn test_module_service_defines_error_hierarchy() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(&tmp);
    make_module(&root, "post");
    let service = read(
        &root
            .join("src")
            .join("modules")
            .join("post")
            .join("application")
            .join("post_service.py"),
    );
    assert!(service.contains("class PostError(Exception)"));
    assert!(service.contains("class PostNotFound"));
}

#[test]
fn test_module_router_maps_not_found_to_404() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(&tmp);
    make_module(&root, "post");
    let router = read(
        &root
            .join("src")
            .join("modules")
            .join("post")
            .join("api")
            .join("router.py"),
    );
    assert!(router.contains("router = APIRouter(prefix=\"/posts\""));
    assert!(router.contains("raise HTTPException(status_code=status.HTTP_404_NOT_FOUND"));
    assert!(router.contains("from src.modules.post.application.post_service import"));
}

#[test]
fn test_module_exports_router_from_api_layer() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(&tmp);
    make_module(&root, "post");
    let init = read(
        &root
            .join("src")
            .join("modules")
            .join("post")
            .join("__init__.py"),
    );
    assert!(init.contains("from .api.router import router"));
    assert!(init.contains("__all__ = [\"router\"]"));
}

#[test]
fn test_make_module_mounts_router_in_main() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(&tmp);
    make_module(&root, "post");
    let main_py = read(&root.join("src").join("main.py"));
    assert!(main_py.contains("fastgen: auto-mount"));
    assert!(main_py.contains("app.include_router(_module.router)"));
}

#[test]
fn test_make_module_syncs_legacy_app_layout_main() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("legacy");
    std::fs::create_dir_all(root.join("app").join("modules")).unwrap();
    std::fs::write(
        root.join("app").join("main.py"),
        "from fastapi import FastAPI\n\n\napp = FastAPI(title=\"Legacy\")\n",
    )
    .unwrap();
    make_module(&root, "user");
    let main_py = read(&root.join("app").join("main.py"));
    assert!(main_py.contains("fastgen: auto-mount"));
    assert!(main_py.contains("from app.modules import modules"));
    assert!(main_py.contains("import importlib"));
    assert_eq!(
        main_py
            .matches("app.include_router(_module.router)")
            .count(),
        1
    );

    make_module(&root, "user");
    assert_eq!(
        read(&root.join("app").join("main.py"))
            .matches("app.include_router(_module.router)")
            .count(),
        1
    );
}

#[test]
fn test_legacy_sync_inserts_imports_in_ruff_order() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("legacy");
    std::fs::create_dir_all(root.join("app").join("modules")).unwrap();
    std::fs::write(
        root.join("app").join("main.py"),
        "from fastapi import FastAPI\n\n\napp = FastAPI(title=\"Legacy\")\n",
    )
    .unwrap();
    make_module(&root, "user");
    let lines = read(&root.join("app").join("main.py"));
    let import_block: Vec<&str> = lines
        .lines()
        .filter(|line| line.starts_with("import ") || line.starts_with("from "))
        .collect();
    assert_eq!(
        import_block,
        vec![
            "import importlib",
            "from fastapi import FastAPI",
            "from app.modules import modules",
        ]
    );
}
