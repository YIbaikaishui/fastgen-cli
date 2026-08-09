"""Tests for Alembic migration scaffolding."""

from pathlib import Path

from typer.testing import CliRunner

from fastgen.cli import app

runner = CliRunner()


def _scaffold(tmp_path: Path) -> Path:
    result = runner.invoke(app, ["new", "demo", "--dir", str(tmp_path)])
    assert result.exit_code == 0, result.output
    return tmp_path / "demo"


def test_new_project_includes_alembic_scaffold(tmp_path: Path) -> None:
    root = _scaffold(tmp_path)
    assert (root / "alembic.ini").exists()
    assert (root / "migrations" / "env.py").exists()
    assert (root / "migrations" / "script.py.mako").exists()
    assert (root / "migrations" / "versions" / "0001_initial.py").exists()


def test_new_project_declares_alembic_dependency(tmp_path: Path) -> None:
    root = _scaffold(tmp_path)
    pyproject = (root / "pyproject.toml").read_text(encoding="utf-8")
    assert "alembic" in pyproject


def test_env_py_wires_settings_and_registry(tmp_path: Path) -> None:
    root = _scaffold(tmp_path)
    env = (root / "migrations" / "env.py").read_text(encoding="utf-8")
    assert "from src.core.config import settings" in env
    assert "config.set_main_option(\"sqlalchemy.url\", settings.database_url)" in env
    assert "importlib.import_module" in env
    assert "async_engine_from_config" in env


def test_generated_main_defers_schema_to_alembic(tmp_path: Path) -> None:
    root = _scaffold(tmp_path)
    main = (root / "src" / "main.py").read_text(encoding="utf-8")
    assert "create_all" not in main
    assert "alembic upgrade head" in main


def test_init_alembic_adds_to_legacy_app_layout(tmp_path: Path) -> None:
    root = tmp_path / "legacy"
    (root / "app" / "modules").mkdir(parents=True)
    (root / "app" / "main.py").write_text("from fastapi import FastAPI\n\napp = FastAPI()\n")
    result = runner.invoke(app, ["init", "alembic", "--dir", str(root)])
    assert result.exit_code == 0, result.output
    assert (root / "alembic.ini").exists()
    assert (root / "migrations" / "env.py").exists()
    env = (root / "migrations" / "env.py").read_text(encoding="utf-8")
    assert "from app.core.config import settings" in env
    assert "from app.modules import modules" in env


def test_init_alembic_is_idempotent(tmp_path: Path) -> None:
    root = _scaffold(tmp_path)
    first = runner.invoke(app, ["init", "alembic", "--dir", str(root)])
    assert first.exit_code == 0
    alembic_ini = (root / "alembic.ini").read_text(encoding="utf-8")
    second = runner.invoke(app, ["init", "alembic", "--dir", str(root)])
    assert second.exit_code == 0
    assert (root / "alembic.ini").read_text(encoding="utf-8") == alembic_ini
