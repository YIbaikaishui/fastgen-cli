"""File writing helpers with dry-run and overwrite protection."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from rich.console import Console

console = Console()


@dataclass
class GeneratedFile:
    path: Path
    content: str
    status: str = field(default="created")


def write_file(
    path: Path, content: str, *, force: bool = False, dry_run: bool = False
) -> GeneratedFile:
    if dry_run:
        return GeneratedFile(path=path, content=content, status="dry-run")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and not force:
        return GeneratedFile(path=path, content=content, status="skipped")
    path.write_text(content if content.endswith("\n") else content + "\n")
    return GeneratedFile(path=path, content=content, status="created")


def report(files: list[GeneratedFile], *, dry_run: bool = False) -> None:
    prefix = "[yellow][dry-run][/yellow] " if dry_run else ""
    for f in files:
        status_style = {
            "created": "green",
            "skipped": "yellow",
            "dry-run": "cyan",
        }[f.status]
        if f.status == "skipped":
            console.print(f"{prefix}[{status_style}][skipped][/] {f.path}")
        else:
            console.print(f"{prefix}[{status_style}][{f.status}][/] {f.path}")
