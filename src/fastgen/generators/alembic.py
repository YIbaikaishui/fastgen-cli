"""Alembic migration scaffolding generator.

Writes ``alembic.ini`` + ``migrations/`` into a project so schema changes can be
applied with ``uv run alembic upgrade head`` instead of deleting the dev DB.
Files are only written when missing (idempotent, non-destructive), matching the
``core/`` behaviour.
"""

from __future__ import annotations

from datetime import UTC, datetime
from pathlib import Path

from ..writers import GeneratedFile
from .base import render_tree

ALEMBIC_TEMPLATE = "alembic"


def generate_alembic(
    project_root: Path,
    *,
    source: str | None = None,
    dry_run: bool = False,
) -> list[GeneratedFile]:
    from ..layout import source_dir_name

    source = source or source_dir_name(project_root)
    context = {
        "source": source,
        "create_date": datetime.now(UTC).strftime("%Y-%m-%d %H:%M:%S"),
    }
    return render_tree(ALEMBIC_TEMPLATE, context, project_root, force=False, dry_run=dry_run)
