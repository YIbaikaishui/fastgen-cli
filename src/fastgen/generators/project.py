"""Project generator: a best-practice FastAPI project scaffold.

``fastgen new <name>`` creates a ``src``-layout project that is immediately
manageable by the rest of fastgen-cli: ``src/core/`` (never overwritten),
``src/modules/__init__.py`` (module registry), ``.fastgen.json`` (layout
config), plus ``.env``, ``src/main.py``, a ``tests/`` suite and the usual
tooling (``pyproject.toml``, ``.gitignore``, ``.python-version``).
"""

from __future__ import annotations

from pathlib import Path

from ..layout import write_config
from ..naming import to_snake, to_title
from ..writers import GeneratedFile, write_file
from .base import render_tree
from .core import generate_core
from .registry import registry_content

PROJECT_TEMPLATE = "project"


def generate_project(
    name: str,
    project_root: Path,
    *,
    title: str | None = None,
    description: str = "",
    version: str = "0.1.0",
    force: bool = False,
    dry_run: bool = False,
) -> list[GeneratedFile]:
    """Scaffold a ``src``-layout FastAPI project into ``project_root``."""
    project_name = to_snake(name)
    context = {
        "source": "src",
        "project_name": project_name,
        "title": title or to_title(name),
        "description": description,
        "version": version,
    }

    files = render_tree(PROJECT_TEMPLATE, context, project_root, force=force, dry_run=dry_run)
    files += generate_core(project_root, source="src", dry_run=dry_run)

    registry = project_root / "src" / "modules" / "__init__.py"
    if not registry.exists() or force:
        files.append(write_file(registry, registry_content({}), force=True, dry_run=dry_run))

    files.append(write_config(project_root, {"source_dir": "src"}, dry_run=dry_run))
    return files
