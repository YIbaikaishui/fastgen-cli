"""Name transformation helpers (snake_case / PascalCase / kebab-case + pluralization)."""

from __future__ import annotations

import re

_WORD_SPLIT_RE = re.compile(r"[_\-\s]+")


def _split(name: str) -> list[str]:
    parts: list[str] = []
    for chunk in _WORD_SPLIT_RE.split(name):
        if not chunk:
            continue
        parts.extend(re.findall(r"[A-Z]+(?=[A-Z][a-z])|[A-Z]?[a-z]+|[A-Z]+|\d+", chunk))
    return parts


def to_snake(name: str) -> str:
    return "_".join(p.lower() for p in _split(name))


def to_pascal(name: str) -> str:
    return "".join(p.capitalize() for p in _split(name))


def to_kebab(name: str) -> str:
    return "-".join(p.lower() for p in _split(name))


def to_plural(word: str) -> str:
    """Simple English pluralization."""
    if word.endswith(("s", "x", "z", "ch", "sh")):
        return f"{word}es"
    if word.endswith("y") and len(word) > 1 and word[-2] not in "aeiou":
        return f"{word[:-1]}ies"
    if word.endswith("f"):
        return f"{word[:-1]}ves"
    if word.endswith("fe"):
        return f"{word[:-2]}ves"
    return f"{word}s"
