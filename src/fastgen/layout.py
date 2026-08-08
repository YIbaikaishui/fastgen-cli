"""Project layout resolution: locate the source directory for a target project.

fastgen supports two layouts:
  - ``src`` layout (recommended, created by ``fastgen new``): modules live in
    ``src/modules/``, the entrypoint is ``src/main.py``.
  - ``app`` layout (legacy): modules live in ``app/modules/``, entrypoint is
    ``app/main.py``.

The layout is resolved from ``.fastgen.json`` (written by ``fastgen new``) and
falls back to auto-detection, then to ``app`` for backwards compatibility.
"""

from __future__ import annotations

import json
from pathlib import Path

from .writers import GeneratedFile, write_file

CONFIG_FILE = ".fastgen.json"
DEFAULT_SOURCE = "app"


def read_config(project_root: Path) -> dict:
    """Read the fastgen config for a project, or ``{}`` when absent/invalid."""
    path = project_root / CONFIG_FILE
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}


def source_dir_name(project_root: Path) -> str:
    """Return the source directory name (``src`` or ``app``) for a project."""
    configured = read_config(project_root).get("source_dir")
    if configured:
        return configured
    if (project_root / "src" / "main.py").exists():
        return "src"
    if (project_root / "app" / "main.py").exists():
        return "app"
    return DEFAULT_SOURCE


def write_config(
    project_root: Path,
    config: dict,
    *,
    dry_run: bool = False,
) -> GeneratedFile:
    """Persist the project config to ``.fastgen.json``."""
    content = json.dumps(config, indent=2, sort_keys=True) + "\n"
    return write_file(project_root / CONFIG_FILE, content, force=True, dry_run=dry_run)
