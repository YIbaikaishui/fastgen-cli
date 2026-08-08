"""End-to-end CLI tests using typer's test runner."""

from pathlib import Path

from typer.testing import CliRunner

from fastgen.cli import app

runner = CliRunner()


def test_new_command(tmp_path: Path) -> None:
    result = runner.invoke(app, ["new", "demo", "--dir", str(tmp_path)])
    assert result.exit_code == 0, result.output
    root = tmp_path / "demo"
    assert (root / ".env").exists()
    assert (root / "src" / "main.py").exists()
    assert (root / "tests" / "test_health.py").exists()
    assert (root / "src" / "core" / "config.py").exists()
    assert (root / ".fastgen.json").exists()


def test_new_refuses_non_empty_dir(tmp_path: Path) -> None:
    target = tmp_path / "demo"
    target.mkdir()
    (target / "keep.txt").write_text("x", encoding="utf-8")
    result = runner.invoke(app, ["new", "demo", "--dir", str(tmp_path)])
    assert result.exit_code == 1
    assert "already exists" in result.output


def test_new_then_make_module(tmp_path: Path) -> None:
    runner.invoke(app, ["new", "demo", "--dir", str(tmp_path)])
    result = runner.invoke(app, ["make", "module", "user", "--dir", str(tmp_path / "demo")])
    assert result.exit_code == 0, result.output
    module_dir = tmp_path / "demo" / "src" / "modules" / "user"
    assert (module_dir / "router.py").exists()

    listed = runner.invoke(app, ["list", "--dir", str(tmp_path / "demo")])
    assert listed.exit_code == 0
    assert "src.modules.user" in listed.output
