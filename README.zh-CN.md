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

- 🗂️ **一个模块 = 一个文件夹**（`app/modules/<feature>/`），每次都是统一的结构
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
# 生成 user 模块（创建 app/modules/user/ + app/core/ + 注册表）
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

```
app/
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

## 🛠️ CLI 参考

| 命令 | 说明 |
| --- | --- |
| `fastgen make module <feature>` | 生成模块骨架、创建 `app/core/` 并登记注册表 |
| `fastgen list` | 列出已注册模块、import 路径和用途 |
| `fastgen --version` / `-V` | 显示版本号 |

### 选项

| 参数 | 适用命令 | 说明 |
| --- | --- | --- |
| `--dir <path>` / `-d` | `make module`、`list` | 目标项目根目录（默认当前目录） |
| `--dry-run` | `make module` | 预览将要生成的文件，不写入任何内容 |
| `--force` / `-f` | `make module` | 覆盖已存在的骨架文件 |

---

## 📐 固定约定

- **模块**位于 `app/modules/<feature>/`——一个文件夹对应一个业务单元。
- **路由**暴露 `prefix="/<复数形式>"`（REST 风格），复用 `app.core.database` 里的 `SessionDep`。
- **注册表**——`app/modules/__init__.py` 保存"模块名 → import 路径"映射，由 fastgen 自动保持同步，请勿手改。
- **核心**——`app/core/config.py` 和 `database.py` 只在**缺失或为空**时生成。已有代码即使加 `--force` 也绝不触碰。

---

## 🔭 Roadmap

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
