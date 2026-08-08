"""Core scaffolding generator: <src>/core/ (config.py + database.py).

Files are only generated when they are missing or empty; existing code is
never overwritten (``force`` does not apply here).
"""

from __future__ import annotations

from pathlib import Path

from ..layout import source_dir_name
from ..writers import GeneratedFile, write_file
from .base import render_template

CORE_TEMPLATE = "core"


def _has_code(path: Path) -> bool:
    return path.exists() and bool(path.read_text(encoding="utf-8").strip())


def generate_core(
    project_root: Path,
    *,
    source: str | None = None,
    dry_run: bool = False,
) -> list[GeneratedFile]:
    source = source or source_dir_name(project_root)
    core_dir = project_root / source / "core"
    files: list[GeneratedFile] = []
    context = {"source": source}

    init_py = core_dir / "__init__.py"
    if not init_py.exists():
        files.append(write_file(init_py, "", dry_run=dry_run))

    config_py = core_dir / "config.py"
    if not _has_code(config_py):
        content = render_template(CORE_TEMPLATE, "config.py.j2", context)
        files.append(write_file(config_py, content, force=True, dry_run=dry_run))

    database_py = core_dir / "database.py"
    if not _has_code(database_py):
        content = render_template(CORE_TEMPLATE, "database.py.j2", context)
        files.append(write_file(database_py, content, force=True, dry_run=dry_run))

    return files
