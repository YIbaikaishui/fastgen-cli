"""fastgen CLI entrypoint."""

# ruff: noqa: B008  (Typer's idiomatic Option/Argument-in-default pattern)

from __future__ import annotations

from pathlib import Path

import typer
from rich.table import Table

from . import __version__
from .generators.core import generate_core
from .generators.module import generate_module
from .generators.registry import list_registered, register_module
from .writers import console, report

app = typer.Typer(
    name="fastgen",
    help="FastAPI feature-based module manager (nest-cli style).",
    no_args_is_help=True,
)
make_app = typer.Typer(help="Scaffold modules and core infrastructure.", no_args_is_help=True)
app.add_typer(make_app, name="make")


@app.callback(invoke_without_command=True)
def main(
    version: bool = typer.Option(False, "--version", "-V", help="Show version and exit."),
) -> None:
    if version:
        typer.echo(f"fastgen {__version__}")
        raise typer.Exit()


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
    business logic yourself. The shared app/core/ database scaffolding and the
    module registry are created automatically.
    """
    files = generate_module(feature, directory, force=force, dry_run=dry_run)
    files += generate_core(directory, dry_run=dry_run)
    files.append(register_module(directory, feature, dry_run=dry_run))
    report(files)
    skipped = [f for f in files if f.status == "skipped"]
    if skipped and not force:
        typer.secho(
            f"{len(skipped)} file(s) already exist. Re-run with --force to overwrite.",
            fg=typer.colors.YELLOW,
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
