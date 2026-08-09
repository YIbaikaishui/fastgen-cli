"""Tests for the ``fastgen new`` project scaffold and src/app layout support."""

from pathlib import Path

import pytest

from fastgen.generators.core import generate_core
from fastgen.generators.module import generate_module
from fastgen.generators.project import generate_project
from fastgen.generators.registry import list_registered, read_registry, register_module
from fastgen.layout import read_config, source_dir_name


@pytest.fixture
def proj(tmp_path: Path) -> Path:
    root = tmp_path / "my_app"
    generate_project("my_app", root)
    return root


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def test_new_creates_best_practice_layout(proj: Path) -> None:
    expected = [
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
    ]
    for rel in expected:
        assert (proj / rel).exists(), f"missing {rel}"


def test_new_src_layout_files(proj: Path) -> None:
    assert ".env" in _read(proj / ".gitignore")
    assert _read(proj / ".env").startswith("DATABASE_URL=")
    assert "from src.core.database import engine" in _read(proj / "src/main.py")
    assert "from src.main import app" in _read(proj / "tests/conftest.py")
    assert 'name = "my_app"' in _read(proj / "pyproject.toml")


def test_generated_async_generators_use_async_generator_annotation(proj: Path) -> None:
    main = _read(proj / "src/main.py")
    assert "AsyncIterator" not in main
    assert "async def lifespan(app: FastAPI) -> AsyncGenerator[None, None]" in main
    conftest = _read(proj / "tests/conftest.py")
    assert "AsyncIterator" not in conftest
    assert "async def client() -> AsyncGenerator[AsyncClient, None]" in conftest


def test_new_writes_layout_config(proj: Path) -> None:
    assert read_config(proj) == {"source_dir": "src"}
    assert source_dir_name(proj) == "src"


def test_new_core_uses_src_imports(proj: Path) -> None:
    assert "from src.core.config import settings" in _read(proj / "src/core/database.py")


def test_make_module_in_src_layout(proj: Path) -> None:
    generate_module("user", proj)
    register_module(proj, "user")

    module_dir = proj / "src" / "modules" / "user"
    assert (module_dir / "schemas.py").exists()
    assert (module_dir / "service.py").exists()
    assert (module_dir / "router.py").exists()
    assert "from src.core.database import get_session" in _read(module_dir / "router.py")

    assert read_registry(proj) == {"user": "src.modules.user"}
    names = [name for name, *_ in list_registered(proj)]
    assert names == ["user"]


def test_make_module_in_app_layout_backwards_compatible(tmp_path: Path) -> None:
    generate_module("user", tmp_path)
    register_module(tmp_path, "user")

    assert (tmp_path / "app" / "modules" / "user" / "router.py").exists()
    assert read_registry(tmp_path) == {"user": "app.modules.user"}
    assert source_dir_name(tmp_path) == "app"


def test_generate_core_skips_existing_code(tmp_path: Path) -> None:
    (tmp_path / "src" / "core").mkdir(parents=True)
    (tmp_path / "src" / "core" / "config.py").write_text("# custom config\n", encoding="utf-8")
    files = generate_core(tmp_path, source="src")
    assert _read(tmp_path / "src" / "core" / "config.py") == "# custom config\n"
    assert not any(f.path.name == "config.py" and f.status == "created" for f in files)
    assert (tmp_path / "src" / "core" / "database.py").exists()
