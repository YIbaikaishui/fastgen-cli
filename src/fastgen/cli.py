"""fastgen CLI entrypoint."""

# ruff: noqa: B008  (Typer's idiomatic Option/Argument-in-default pattern)

from __future__ import annotations

from pathlib import Path

import typer
from rich.table import Table

from . import __version__
from .generators.alembic import generate_alembic
from .generators.core import generate_core
from .generators.main import sync_main
from .generators.module import generate_module
from .generators.project import generate_project
from .generators.registry import list_registered, register_module
from .writers import console, report

app = typer.Typer(
    name="fastgen",
    help="FastAPI feature-based module manager (nest-cli style).",
    no_args_is_help=True,
)
make_app = typer.Typer(help="Scaffold modules and core infrastructure.", no_args_is_help=True)
app.add_typer(make_app, name="make")
init_app = typer.Typer(help="Add optional infrastructure to an existing project.", no_args_is_help=True)
app.add_typer(init_app, name="init")


@app.callback(invoke_without_command=True)
def main(
    version: bool = typer.Option(False, "--version", "-V", help="Show version and exit."),
) -> None:
    if version:
        typer.echo(f"fastgen {__version__}")
        raise typer.Exit()


@app.command("new")
def new_project(
    name: str = typer.Argument(..., help="Project name, or '.' to scaffold into --dir."),
    directory: Path = typer.Option(
        Path.cwd(), "--dir", "-d", help="Parent directory for the new project."
    ),
    title: str | None = typer.Option(None, "--title", help="Human-readable app title."),
    description: str = typer.Option("", "--description", help="Short project description."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Preview files without writing."),
    force: bool = typer.Option(False, "--force", "-f", help="Overwrite an existing project."),
) -> None:
    """Scaffold a new best-practice FastAPI project.

    Generates a ``src``-layout project: ``.env``, ``src/main.py``, ``src/core/``
    (config + async database), the ``src/modules/`` registry, a ``tests/`` suite,
    plus ``pyproject.toml`` / ``.gitignore`` / ``.python-version``. The project is
    immediately manageable with ``fastgen make module`` / ``fastgen list``.
    """
    target = directory if name == "." else directory / name
    if target.exists() and any(target.iterdir()) and not force:
        typer.secho(
            f"Directory {target} already exists and is not empty. Use --force to overwrite.",
            fg=typer.colors.RED,
        )
        raise typer.Exit(code=1)
    files = generate_project(
        name,
        target,
        title=title,
        description=description,
        force=force,
        dry_run=dry_run,
    )
    report(files, dry_run=dry_run)
    if not dry_run:
        typer.secho(f"Project created at {target}", fg=typer.colors.GREEN)
        typer.secho(
            f"cd {target} && uv sync && uv run uvicorn src.main:app --reload",
            fg=typer.colors.CYAN,
        )


@make_app.command("module")
def make_module(
    feature: str = typer.Argument(..., help="Feature name, e.g. user"),
    directory: Path = typer.Option(Path.cwd(), "--dir", "-d", help="Target project root."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Preview files without writing."),
    force: bool = typer.Option(False, "--force", "-f", help="Overwrite existing files."),
) -> None:
    """Scaffold a feature module and register it.

    Generates a minimal module skeleton (schemas / service / router /
    __init__) that outlines the module's shape; fill in the entity fields and
    business logic yourself. The shared core/ database scaffolding and the
    module registry are created automatically.
    """
    files = generate_module(feature, directory, force=force, dry_run=dry_run)
    files += generate_core(directory, dry_run=dry_run)
    files.append(register_module(directory, feature, dry_run=dry_run))
    files.append(sync_main(directory, dry_run=dry_run))
    report(files, dry_run=dry_run)
    skipped = [f for f in files if f.status == "skipped"]
    if skipped and not force:
        typer.secho(
            f"{len(skipped)} file(s) already exist. Re-run with --force to overwrite.",
            fg=typer.colors.YELLOW,
        )


@init_app.command("alembic")
def init_alembic(
    directory: Path = typer.Option(Path.cwd(), "--dir", "-d", help="Target project root."),
    dry_run: bool = typer.Option(False, "--dry-run", help="Preview files without writing."),
) -> None:
    """Add Alembic migration scaffolding to an existing project.

    Writes ``alembic.ini`` and ``migrations/`` (async env + an empty baseline
    revision), wiring ``DATABASE_URL`` from the project's settings and
    ``Base.metadata`` from the module registry. Existing files are never
    overwritten.
    """
    files = generate_alembic(directory, dry_run=dry_run)
    report(files, dry_run=dry_run)
    typer.secho(
        "Next steps:\n"
        "  1. Add the dependency:  uv add alembic\n"
        "  2. Apply the baseline:   uv run alembic upgrade head\n"
        "  3. After model changes:  uv run alembic revision --autogenerate -m '<message>'"
        " && uv run alembic upgrade head\n"
        "  If the DB was already created without Alembic (create_all), adopt it with"
        " `uv run alembic stamp head`,\n"
        "  or delete the dev DB and re-create it via steps 2-3.",
        fg=typer.colors.CYAN,
    )


@app.command("list")
def list_modules(
    directory: Path = typer.Option(Path.cwd(), "--dir", "-d", help="Target project root."),
) -> None:
    """List registered modules and their boundaries."""
    rows = list_registered(directory)
    table = Table(title="Registered modules")
    table.add_column("module", style="cyan")
    table.add_column("path", style="magenta")
    table.add_column("description")
    for name, import_path, doc in rows:
        table.add_row(name, import_path, doc)
    console.print(table)
    if not rows:
        typer.secho("No modules registered yet. Run `fastgen make module <name>`.", fg="yellow")


if __name__ == "__main__":
    app()
