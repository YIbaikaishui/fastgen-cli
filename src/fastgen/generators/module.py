"""Feature-module generator: schemas.py + service.py + router.py + __init__.py.

The skeleton only sketches the module's shape (entity, business layer, API
boundary, shared session dependency) so AI agents can reason about it; real
business code is filled in by the developer.
"""

from __future__ import annotations

from pathlib import Path

from ..naming import to_kebab, to_pascal, to_plural, to_snake
from ..writers import GeneratedFile
from .base import render_tree

MODULE_TEMPLATE = "module"


def module_context(feature: str) -> dict[str, str]:
    snake = to_snake(feature)
    return {
        "pascal": to_pascal(feature),
        "snake": snake,
        "kebab": to_kebab(feature),
        "plural": to_plural(snake),
    }


def generate_module(
    feature: str,
    project_root: Path,
    *,
    force: bool = False,
    dry_run: bool = False,
) -> list[GeneratedFile]:
    snake = to_snake(feature)
    module_dir = project_root / "app" / "modules" / snake
    return render_tree(
        MODULE_TEMPLATE, module_context(feature), module_dir, force=force, dry_run=dry_run
    )
