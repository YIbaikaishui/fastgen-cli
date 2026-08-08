<div align="center">

# ⚡ fastgen-cli

**面向 FastAPI 的 nest-cli 风格模块管理器** —— 一键生成模块骨架，让项目结构保持整洁，让 AI 助手一眼看清整个项目。

零配置。一条命令。业务逻辑交给你。

[![PyPI version](https://img.shields.io/pypi/v/fastgen-cli.svg)](https://pypi.org/project/fastgen-cli/)
[![Python](https://img.shields.io/pypi/pyversions/fastgen-cli.svg)](https://pypi.org/project/fastgen-cli/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![PyPI downloads](https://img.shields.io/pypi/dm/fastgen-cli)](https://pypi.org/project/fastgen-cli/)

</div>

---

## ✨ 为什么选择 fastgen-cli？

FastAPI 以"不强制结构"著称——自由是好事，但项目也容易失控：路由散落各处、实体到处乱放、没人知道项目里到底有哪些模块。

**fastgen-cli** 正是来解决这个问题的。它管理**模块结构**，不碰你的业务代码：

- 🏗️ **一键脚手架整个项目**——`fastgen new my-app` 生成一个最佳实践的 `src/` 布局 FastAPI 项目（`.env`、`src/main.py`、`src/core/`、模块注册表、`tests/`），开箱即跑
- 🗂️ **一个模块 = 一个文件夹**（`<src>/modules/<feature>/`），每次都是统一的结构
- 🧩 **最小骨架**——实体类、业务层、路由 + 共享 session 依赖。刚好够"看懂"模块，绝不多生成代码挡住你
- 📇 **自动维护注册表**——`app/modules/__init__.py` 记录每个模块；AI 和开发者读它即可瞬间了解项目
- 🔌 **共享 DB 核心**只生成一次——`app/core/` 内含 pydantic-settings 配置 + 异步 SQLAlchemy `get_session`（最佳实践：`expire_on_commit=False`、`AsyncAttrs`）
- 🛡️ **绝不覆盖你的代码**——只生成缺失或为空的内容

---

## 📦 安装

```bash
pip install fastgen-cli
# 或
uv add fastgen-cli
```

> 需要 **Python 3.11+**。

---

## 🚀 快速开始

```bash
# 脚手架整个项目（src/ 布局：.env、src/main.py、src/core/、tests/）
fastgen new my-app
cd my-app && uv sync && uv run uvicorn src.main:app --reload

# 生成 user 模块（创建 src/modules/user/ + src/core/ + 注册表）
fastgen make module user

# 查看所有已注册模块及其边界
fastgen list
```

就是这样。没有配置文件、没有 YAML、没有 spec——一条命令，拿到骨架。

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

## 🧱 生成的内容

### `fastgen new <name>`

一个完整、可直接运行的最佳实践 FastAPI 项目：

```
my-app/
├── .env / .env.example         # DATABASE_URL 等
├── .gitignore                  # 忽略 .env、venv、__pycache__、*.db
├── .python-version             # 3.11
├── pyproject.toml              # 依赖 + ruff / pytest 配置
├── README.md
├── .fastgen.json               # {"source_dir": "src"} —— fastgen 依据它识别布局
├── src/
│   ├── __init__.py
│   ├── main.py                 # FastAPI 应用；模块路由从注册表自动加载
│   ├── core/                   # 共享基础设施（绝不覆盖）
│   │   ├── __init__.py
│   │   ├── config.py           # pydantic-settings 配置，读 .env
│   │   └── database.py         # Base（AsyncAttrs）、异步 engine、get_session
│   └── modules/
│       └── __init__.py         # 📇 模块注册表（自动维护）
└── tests/
    ├── __init__.py
    ├── conftest.py             # httpx ASGI client fixture
    └── test_health.py          # /health 冒烟测试
```

### `fastgen make module <feature>`

```
app/  （由 `fastgen new` 生成的项目则为 src/）
├── core/                        # 首次使用时自动创建（绝不覆盖）
│   ├── __init__.py
│   ├── config.py                # pydantic-settings 配置，DATABASE_URL 读自 .env
│   └── database.py              # Base（AsyncAttrs）、异步 engine、get_session
└── modules/
    ├── __init__.py              # 📇 模块注册表（自动维护）
    └── user/
        ├── __init__.py          # 对外暴露 router
        ├── schemas.py           # 实体骨架：  class User(BaseModel): pass
        ├── service.py           # 业务层边界：  class UserService
        └── router.py            # APIRouter + SessionDep（已接好异步依赖注入）
```

**`router.py`** 已经接好了共享 session 依赖，你只需添加端点：

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

## ⚖️ 它和别的方案比怎么样？

### 与其他代码生成工具对比

| 工具 | 理念 | 项目结构 | 后续模块管理 | FastAPI 专用 |
| --- | --- | --- | --- | --- |
| **fastgen-cli** | 有主见、最小骨架、零配置 | 最佳实践 `src/` 布局 | ✅ 自动注册表 + `make module` / `list` | ✅ |
| **`nest new`**（NestJS） | 原版 nest-cli，同一思路 | 有主见 | ✅ | ❌ Node.js/TS |
| **cookiecutter** | 面向任意内容的可复用模板 | 模板说了算 | ❌ 静态快照 | 取决于模板 |
| **Full-stack FastAPI 模板** | 全家桶（Docker、认证等） | 庞大的、强约定的单体 | ❌ 手动 | ✅ |
| **`uv init`** | 最简 Python 项目 | `pyproject.toml` + hello world | ❌ | ❌ |
| **让 LLM 手写脚手架** | 每次聊天临时拼 | 每次运行都不一致 | ❌ | 偶尔 |

**fastgen-cli** 位于"极简但全要自己干"（`uv init`）和"全家桶且约定过多"（full-stack 模板）之间。它给你一个完整、可运行的 FastAPI 底座，**并且**长期维护模块结构——这是任何静态模板生成器都做不到的。

### 与直接用 `uv init` 起步对比

`uv init my-app` 是最自然的基线——极简、通用、无锁定。权衡如下：

| | `uv init` | `fastgen new` |
| --- | --- | --- |
| 得到什么 | `pyproject.toml` + `main.py` hello world | 完整 FastAPI 应用：`.env`、`src/main.py`（lifespan + `/health`）、`src/core/`（pydantic-settings + 异步 SQLAlchemy）、模块注册表、`tests/`、ruff/pytest 配置 |
| 接下来你要 | 手写依赖、`src/` 布局、lifespan/配置/数据库/测试 | 只管写业务逻辑 |
| 最终结构 | 每个开发者都不同 | 所有项目完全一致 |
| 后续模块管理 | 无 | `fastgen make module` 维护注册表，可随时 `fastgen list` |
| 锁定风险 | 无 | 布局就是普通文件；任何时候都能删掉 fastgen，生成物不强制依赖它 |

**`uv init` 的优点**：通用、极简、零意见，而且你本来就有 `uv`。
**缺点**：所有 FastAPI 相关的决定（布局、DB session 接线、配置、测试）都留给你自己，于是每个项目结构都不一样。

**`fastgen new` 的优点**：一条命令得到完整的最佳实践底座；整个团队保持一致；模块通过注册表随时可发现；绝不覆盖你的代码；方便 AI 助手理解项目。
**缺点**：布局有主见（`src/` + `core/` + 注册表）——需要非标准结构时得自己改；只面向 FastAPI。

**二者互补，而非竞争**：`fastgen new` 生成的项目仍由 `uv` 管理（`uv sync`、`uv run`）。如果你确实是从 `uv init` 起步的，之后也能无缝接入 fastgen——在项目里跑 `fastgen make module <feature>` 即可，它会自动创建 `core/`、`modules/` 和注册表（并能自动识别布局）。

---

## 🛠️ CLI 参考

| 命令 | 说明 |
| --- | --- |
| `fastgen new <name>` | 脚手架一个新的最佳实践 `src/` 布局 FastAPI 项目 |
| `fastgen make module <feature>` | 生成模块骨架、创建 `src/core/` 并登记注册表 |
| `fastgen list` | 列出已注册模块、import 路径和用途 |
| `fastgen --version` / `-V` | 显示版本号 |

### 选项

| 参数 | 适用命令 | 说明 |
| --- | --- | --- |
| `--dir <path>` / `-d` | `new`、`make module`、`list` | 目标项目根目录（默认当前目录） |
| `--title <name>` | `new` | 人类可读的应用标题（默认取项目名） |
| `--description <text>` | `new` | 简短的项目描述 |
| `--dry-run` | `new`、`make module` | 预览将要生成的文件，不写入任何内容 |
| `--force` / `-f` | `new`、`make module` | 覆盖已存在的文件 |

---

## 📐 固定约定

- **布局**——`fastgen new` 生成 `src/` 布局并记录到 `.fastgen.json`。fastgen 依次按 `.fastgen.json`、自动探测、最后回退到 `app/`（兼容旧项目）的顺序解析布局。
- **模块**位于 `<src>/modules/<feature>/`——一个文件夹对应一个业务单元。
- **路由**暴露 `prefix="/<复数形式>"`（REST 风格），复用 `<src>.core.database` 里的 `SessionDep`。
- **注册表**——`<src>/modules/__init__.py` 保存"模块名 → import 路径"映射，由 fastgen 自动保持同步，请勿手改。
- **核心**——`<src>/core/config.py` 和 `database.py` 只在**缺失或为空**时生成。已有代码即使加 `--force` 也绝不触碰。

---

## 🔭 Roadmap

- [x] `new` —— 脚手架整个最佳实践 `src/` 布局项目
- [x] `make module` —— schemas + service 骨架
- [x] 模块注册表 + `fastgen list`
- [ ] `make resource` —— 完整 CRUD 路由生成
- [ ] Alembic 迁移提示

---

## 🧑‍💻 开发

```bash
git clone https://github.com/YIbaikaishui/fastgen-cli.git
cd fastgen-cli
uv sync
uv run fastgen --help
```

代码检查：`uv run ruff check src`。

---

## 📄 协议

MIT © 一白开水
