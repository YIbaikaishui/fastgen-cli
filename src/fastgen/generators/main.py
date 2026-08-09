"""Entrypoint sync: ensure ``<src>/main.py`` auto-mounts registry routers.

``fastgen make module`` calls :func:`sync_main` so a freshly scaffolded router
is served without hand-editing ``main.py``. Injection is marker-based and
idempotent; existing hand-written code is left untouched.
"""

from __future__ import annotations

from pathlib import Path

from ..writers import GeneratedFile, write_file

MOUNT_MARKER = "# --- fastgen: auto-mount (do not remove) ---"
MOUNT_END_MARKER = "# --- fastgen: end auto-mount ---"

_MOUNT_BLOCK = [
    MOUNT_MARKER,
    "for _import_path in modules.values():",
    "    _module = importlib.import_module(_import_path)",
    "    app.include_router(_module.router)",
    MOUNT_END_MARKER,
]


def _already_mounted(content: str) -> bool:
    return MOUNT_MARKER in content or "app.include_router(_module.router)" in content


def _import_groups(lines: list[str], idx: int) -> tuple[list[list[str]], int]:
    """Split the import region into blank-line-separated groups.

    Returns ``(groups, end)`` where ``end`` is the index just past the region
    (the next non-import, non-blank line).
    """
    groups: list[list[str]] = []
    current: list[str] = []
    end = idx
    while end < len(lines):
        line = lines[end]
        if line.startswith(("import ", "from ")):
            current.append(line)
        elif line == "":
            if current:
                groups.append(current)
                current = []
        else:
            break
        end += 1
    if current:
        groups.append(current)
    return groups, end


def _insert_plain_import(group: list[str], imp: str) -> None:
    """Insert a plain ``import x`` after ``from __future__`` and existing plain imports."""
    pos = 0
    while pos < len(group) and group[pos].startswith("from __future__"):
        pos += 1
    while pos < len(group) and group[pos].startswith("import ") and group[pos] < imp:
        pos += 1
    group.insert(pos, imp)


def _insert_module_import(groups: list[list[str]], imp: str, source: str) -> None:
    """Place ``from <source>.modules import modules`` in the first-party import group.

    Ruff's isort keeps ``<source>.*`` imports together, so the new import is inserted
    alphabetically inside the contiguous ``from <source>.`` run (a fresh group is
    appended when no such run exists).
    """
    for group in groups:
        run_start = next(
            (i for i, line in enumerate(group) if line.startswith(f"from {source}.")),
            None,
        )
        if run_start is None:
            continue
        run_end = run_start
        while run_end + 1 < len(group) and group[run_end + 1].startswith(f"from {source}."):
            run_end += 1
        pos = run_start
        for line in group[run_start : run_end + 1]:
            if line < imp:
                pos += 1
            else:
                break
        group.insert(pos, imp)
        return
    groups.append([imp])


def _insert_imports(lines: list[str], source: str) -> list[str]:
    idx = 0
    if lines and lines[0].startswith('"""'):
        if len(lines[0]) >= 6 and lines[0].rstrip().endswith('"""'):
            idx = 1
        else:
            for i, line in enumerate(lines):
                if i > 0 and line.rstrip().endswith('"""'):
                    idx = i + 1
                    break
            else:
                idx = len(lines)
    groups, end = _import_groups(lines, idx)
    trailing = 0
    cursor = end
    while cursor > idx and lines[cursor - 1] == "":
        trailing += 1
        cursor -= 1
    if not any(line.startswith("import importlib") for line in lines):
        if groups:
            _insert_plain_import(groups[0], "import importlib")
        else:
            groups = [["import importlib"]]
    module_imp = f"from {source}.modules import modules"
    if not any(line.startswith(module_imp) for line in lines):
        _insert_module_import(groups, module_imp, source)
    region: list[str] = []
    for i, group in enumerate(groups):
        if i:
            region.append("")
        region.extend(group)
    region.extend([""] * trailing)
    lines[idx:end] = region
    return lines


def sync_main(
    project_root: Path,
    *,
    source: str | None = None,
    dry_run: bool = False,
) -> GeneratedFile:
    """Inject the registry auto-mount loop into ``<src>/main.py`` (idempotent)."""
    from ..layout import source_dir_name

    source = source or source_dir_name(project_root)
    main_path = project_root / source / "main.py"
    if not main_path.exists():
        return GeneratedFile(path=main_path, content="", status="skipped")
    content = main_path.read_text(encoding="utf-8")
    if _already_mounted(content):
        return GeneratedFile(path=main_path, content=content, status="skipped")

    lines = _insert_imports(content.splitlines(), source)
    start = next(
        (i for i, line in enumerate(lines) if line.startswith("app = FastAPI(")),
        None,
    )
    if start is None:
        new_content = content.rstrip("\n") + "\n" + "\n".join(_MOUNT_BLOCK) + "\n"
    else:
        depth = 0
        end = start
        for i in range(start, len(lines)):
            depth += lines[i].count("(") - lines[i].count(")")
            if depth <= 0:
                end = i
                break
        lines[end + 1 : end + 1] = _MOUNT_BLOCK
        new_content = "\n".join(lines) + "\n"
    return write_file(main_path, new_content, force=True, dry_run=dry_run)
