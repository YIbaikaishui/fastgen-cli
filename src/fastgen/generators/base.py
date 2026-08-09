"""Jinja2 rendering helpers shared by all generators."""

from __future__ import annotations

from pathlib import Path

from jinja2 import Environment, FileSystemLoader, StrictUndefined

from ..writers import GeneratedFile, write_file

TEMPLATES_DIR = Path(__file__).resolve().parent.parent / "templates"


def _env(template_dir: str) -> Environment:
    return Environment(
        loader=FileSystemLoader(TEMPLATES_DIR / template_dir),
        trim_blocks=True,
        lstrip_blocks=True,
        undefined=StrictUndefined,
    )


def render_template(template_dir: str, name: str, context: dict) -> str:
    """Render a single ``*.j2`` file without writing anything."""
    return _env(template_dir).get_template(name).render(**context)


def render_tree(
    template_dir: str,
    context: dict,
    dest_dir: Path,
    *,
    force: bool = False,
    dry_run: bool = False,
) -> list[GeneratedFile]:
    """Render every ``*.j2`` file in ``template_dir`` into ``dest_dir``."""
    src = TEMPLATES_DIR / template_dir
    env = _env(template_dir)
    files: list[GeneratedFile] = []
    for tmpl in sorted(src.rglob("*.j2")):
        rel_template = tmpl.relative_to(src)
        rel = Path(env.from_string(str(rel_template)).render(**context))
        rendered = env.get_template(str(rel_template)).render(**context)
        target = dest_dir / rel.with_suffix("")
        files.append(write_file(target, rendered, force=force, dry_run=dry_run))
    return files
