"""Tests for ``fastgen make module`` generated output."""

from pathlib import Path

from typer.testing import CliRunner

from fastgen.cli import app
from fastgen.generators.main import _insert_imports

runner = CliRunner()


def _scaffold(tmp_path: Path) -> Path:
    result = runner.invoke(app, ["new", "demo", "--dir", str(tmp_path)])
    assert result.exit_code == 0, result.output
    return tmp_path / "demo"


def test_module_generates_full_skeleton(tmp_path: Path) -> None:
    root = _scaffold(tmp_path)
    result = runner.invoke(app, ["make", "module", "post", "--dir", str(root)])
    assert result.exit_code == 0, result.output
    module_dir = root / "src" / "modules" / "post"
    for name in ("model.py", "schemas.py", "service.py", "router.py", "__init__.py"):
        assert (module_dir / name).exists(), name
    assert (module_dir / "tests" / "conftest.py").exists()
    assert (module_dir / "tests" / "test_post.py").exists()


def test_module_schemas_follow_orm_read_convention(tmp_path: Path) -> None:
    root = _scaffold(tmp_path)
    runner.invoke(app, ["make", "module", "post", "--dir", str(root)])
    schemas = (root / "src" / "modules" / "post" / "schemas.py").read_text(encoding="utf-8")
    assert "from_attributes=True" in schemas
    assert "class PostCreate" in schemas
    assert "class PostRead" in schemas
    assert "class PostUpdate" in schemas


def test_module_model_generates_entity(tmp_path: Path) -> None:
    root = _scaffold(tmp_path)
    runner.invoke(app, ["make", "module", "post", "--dir", str(root)])
    model = (root / "src" / "modules" / "post" / "model.py").read_text(encoding="utf-8")
    assert "class Post(Base)" in model
    assert '__tablename__ = "posts"' in model
    assert "mapped_column(primary_key=True)" in model


def test_module_tests_include_test_db_fixture(tmp_path: Path) -> None:
    root = _scaffold(tmp_path)
    runner.invoke(app, ["make", "module", "post", "--dir", str(root)])
    conftest = (root / "src" / "modules" / "post" / "tests" / "conftest.py").read_text(
        encoding="utf-8"
    )
    assert "dependency_overrides[get_session]" in conftest
    assert "StaticPool" in conftest
    assert "AsyncIterator" not in conftest
    assert "AsyncGenerator[AsyncClient, None]" in conftest


def test_module_service_defines_error_hierarchy(tmp_path: Path) -> None:
    root = _scaffold(tmp_path)
    runner.invoke(app, ["make", "module", "post", "--dir", str(root)])
    service = (root / "src" / "modules" / "post" / "service.py").read_text(encoding="utf-8")
    assert "class PostError(Exception)" in service
    assert "class PostNotFound" in service


def test_make_module_mounts_router_in_main(tmp_path: Path) -> None:
    root = _scaffold(tmp_path)
    runner.invoke(app, ["make", "module", "post", "--dir", str(root)])
    main_py = (root / "src" / "main.py").read_text(encoding="utf-8")
    assert "fastgen: auto-mount" in main_py
    assert "app.include_router(_module.router)" in main_py


def test_make_module_syncs_legacy_app_layout_main(tmp_path: Path) -> None:
    root = tmp_path / "legacy"
    (root / "app" / "modules").mkdir(parents=True)
    (root / "app" / "main.py").write_text(
        'from fastapi import FastAPI\n\n\napp = FastAPI(title="Legacy")\n',
        encoding="utf-8",
    )
    result = runner.invoke(app, ["make", "module", "user", "--dir", str(root)])
    assert result.exit_code == 0, result.output
    main_py = (root / "app" / "main.py").read_text(encoding="utf-8")
    assert "fastgen: auto-mount" in main_py
    assert "from app.modules import modules" in main_py
    assert "import importlib" in main_py
    assert main_py.count("app.include_router(_module.router)") == 1

    result2 = runner.invoke(app, ["make", "module", "user", "--dir", str(root)])
    assert result2.exit_code == 0
    assert (
        (root / "app" / "main.py").read_text(encoding="utf-8").count(
            "app.include_router(_module.router)"
        )
        == 1
    )


def test_legacy_sync_inserts_imports_in_ruff_order(tmp_path: Path) -> None:
    root = tmp_path / "legacy"
    (root / "app" / "modules").mkdir(parents=True)
    (root / "app" / "main.py").write_text(
        'from fastapi import FastAPI\n\n\napp = FastAPI(title="Legacy")\n',
        encoding="utf-8",
    )
    result = runner.invoke(app, ["make", "module", "user", "--dir", str(root)])
    assert result.exit_code == 0, result.output
    lines = (root / "app" / "main.py").read_text(encoding="utf-8").splitlines()
    import_block = [
        line
        for line in lines
        if line.startswith(("import ", "from "))
    ]
    assert import_block == [
        "import importlib",
        "from fastapi import FastAPI",
        "from app.modules import modules",
    ]


def test_insert_imports_respects_docstring_and_future(tmp_path: Path) -> None:
    lines = [
        '"""Entrypoint."""',
        "from __future__ import annotations",
        "",
        "from collections.abc import AsyncGenerator",
        "from fastapi import FastAPI",
        "",
        "from app.core.database import Base, engine",
        "from app.modules.user.router import router as user_router",
        "",
        "app = FastAPI()",
    ]
    out = _insert_imports(lines, "app")
    block = [line for line in out if line.startswith(("import ", "from "))]
    assert block.index("from __future__ import annotations") == 0
    assert block.index("import importlib") < block.index("from fastapi import FastAPI")
    assert block.index("from app.modules import modules") < block.index(
        "from app.modules.user.router import router as user_router"
    )
    assert block.index("from fastapi import FastAPI") < block.index(
        "from app.modules import modules"
    )
