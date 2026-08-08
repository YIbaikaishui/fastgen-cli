<div align="center">

# ⚡ fastgen-cli

**A nest-cli style module manager for FastAPI** — scaffold modules, keep the project tidy, and let AI agents see the whole structure at a glance.

Zero-config. One command. Fill in the business logic yourself.

[![PyPI version](https://img.shields.io/pypi/v/fastgen-cli.svg)](https://pypi.org/project/fastgen-cli/)
[![Python](https://img.shields.io/pypi/pyversions/fastgen-cli.svg)](https://pypi.org/project/fastgen-cli/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![PyPI downloads](https://img.shields.io/pypi/dm/fastgen-cli)](https://pypi.org/project/fastgen-cli/)

</div>

---

## ✨ Why fastgen-cli?

FastAPI is famously unopinionated — which is great for freedom, but bad for *structure*. Projects drift into chaos: routers scattered, entities everywhere, no one knows what modules exist.

**fastgen-cli** fixes exactly that. It manages **module structure**, not your business code:

- 🗂️ **One module = one folder** (`app/modules/<feature>/`), with a consistent shape every time
- 🧩 **Minimal skeleton** — entity class, service boundary, router + shared session dependency. Just enough to *see* the module, never enough to get in the way
- 📇 **Auto-maintained registry** — `app/modules/__init__.py` tracks every module; AI agents and devs read it to understand the project instantly
- 🔌 **Shared DB core** generated once — `app/core/` with pydantic-settings config + async SQLAlchemy `get_session` (best-practice, `expire_on_commit=False`, `AsyncAttrs`)
- 🛡️ **Never overwrites your code** — only generates what's missing or empty

---

## 📦 Installation

```bash
pip install fastgen-cli
# or
uv add fastgen-cli
```

> Requires **Python 3.11+**.

---

## 🚀 Quick start

```bash
# Scaffold a user module (creates app/modules/user/ + app/core/ + registry)
fastgen make module user

# See all registered modules and their boundaries
fastgen list
```

That's it. No config file, no YAML, no spec — run a command, get the skeleton.

```
$ fastgen list
             Registered modules
┏━━━━━━━━┳━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━┓
┃ module ┃ path             ┃ description    ┃
┡━━━━━━━━╇━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━┩
│ user   │ app.modules.user │ User module.   │
└────────┴──────────────────┴────────────────┘
```

---

## 🧱 What it generates

```
app/
├── core/                        # auto-created on first use (never overwritten)
│   ├── __init__.py
│   ├── config.py                # pydantic-settings Settings, DATABASE_URL from .env
│   └── database.py              # Base (AsyncAttrs), async engine, get_session
└── modules/
    ├── __init__.py              # 📇 module registry (auto-maintained)
    └── user/
        ├── __init__.py          # re-exports the router
        ├── schemas.py           # entity skeleton:  class User(BaseModel): pass
        ├── service.py           # business layer:  class UserService
        └── router.py            # APIRouter + SessionDep (async DI wired)
```

**`router.py`** already wires the shared session dependency, so you just add endpoints:

```python
from fastapi import APIRouter, Depends, HTTPException

from app.core.database import get_session
from app.modules.user.schemas import User

SessionDep = Annotated[AsyncSession, Depends(get_session)]

router = APIRouter(prefix="/users", tags=["users"])

@router.get("", response_model=list[User])
async def list_users(session: SessionDep) -> list[User]:
    ...
```

---

## 🛠️ CLI reference

| Command | Description |
| --- | --- |
| `fastgen make module <feature>` | Scaffold a feature module, generate `app/core/`, register it |
| `fastgen list` | List registered modules, import paths, and purposes |
| `fastgen --version` / `-V` | Show version |

### Options

| Flag | Applies to | Description |
| --- | --- | --- |
| `--dir <path>` / `-d` | `make module`, `list` | Target project root (default: current dir) |
| `--dry-run` | `make module` | Preview files without writing anything |
| `--force` / `-f` | `make module` | Overwrite existing skeleton files |

---

## 📐 Conventions (fixed)

- **Modules** live in `app/modules/<feature>/` — one business unit per folder.
- **Router** exposes `prefix="/<plural>"` (REST-style) and reuses `SessionDep` from `app.core.database`.
- **Registry** — `app/modules/__init__.py` maps module name → import path. Always kept in sync by fastgen; don't hand-edit.
- **Core** — `app/core/config.py` and `database.py` are generated only when **missing or empty**. Existing code is never touched, even with `--force`.

---

## 🔭 Roadmap

- [x] `make module` — schemas + service skeleton
- [x] Module registry + `fastgen list`
- [ ] `make resource` — full CRUD router generation
- [ ] Alembic migration hints

---

## 🧑‍💻 Development

```bash
git clone https://github.com/YIbaikaishui/fastgen-cli.git
cd fastgen-cli
uv sync
uv run fastgen --help
```

Lint with `uv run ruff check src`.

---

## 📄 License

MIT © [YIbaikaishui](LICENSE)
