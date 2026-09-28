//! Template files embedded into the binary at compile time.
//!
//! Generated from the `templates/` tree; each entry is `(dir, rel_path, content)`.

pub struct TemplateFile {
    pub dir: &'static str,
    pub rel: &'static str,
    pub content: &'static str,
}

pub const TEMPLATES: &[TemplateFile] = &[
    TemplateFile {
        dir: "alembic",
        rel: "alembic.ini.j2",
        content: include_str!("templates/alembic/alembic.ini.j2"),
    },
    TemplateFile {
        dir: "alembic",
        rel: "migrations/env.py.j2",
        content: include_str!("templates/alembic/migrations/env.py.j2"),
    },
    TemplateFile {
        dir: "alembic",
        rel: "migrations/script.py.mako.j2",
        content: include_str!("templates/alembic/migrations/script.py.mako.j2"),
    },
    TemplateFile {
        dir: "alembic",
        rel: "migrations/versions/0001_initial.py.j2",
        content: include_str!("templates/alembic/migrations/versions/0001_initial.py.j2"),
    },
    TemplateFile {
        dir: "core",
        rel: "config.py.j2",
        content: include_str!("templates/core/config.py.j2"),
    },
    TemplateFile {
        dir: "core",
        rel: "database.py.j2",
        content: include_str!("templates/core/database.py.j2"),
    },
    TemplateFile {
        dir: "module",
        rel: "__init__.py.j2",
        content: include_str!("templates/module/__init__.py.j2"),
    },
    TemplateFile {
        dir: "module",
        rel: "api/__init__.py.j2",
        content: include_str!("templates/module/api/__init__.py.j2"),
    },
    TemplateFile {
        dir: "module",
        rel: "api/router.py.j2",
        content: include_str!("templates/module/api/router.py.j2"),
    },
    TemplateFile {
        dir: "module",
        rel: "application/__init__.py.j2",
        content: include_str!("templates/module/application/__init__.py.j2"),
    },
    TemplateFile {
        dir: "module",
        rel: "application/schemas.py.j2",
        content: include_str!("templates/module/application/schemas.py.j2"),
    },
    TemplateFile {
        dir: "module",
        rel: "application/{{ snake }}_service.py.j2",
        content: include_str!("templates/module/application/{{ snake }}_service.py.j2"),
    },
    TemplateFile {
        dir: "module",
        rel: "domain/__init__.py.j2",
        content: include_str!("templates/module/domain/__init__.py.j2"),
    },
    TemplateFile {
        dir: "module",
        rel: "domain/model.py.j2",
        content: include_str!("templates/module/domain/model.py.j2"),
    },
    TemplateFile {
        dir: "module",
        rel: "domain/repository.py.j2",
        content: include_str!("templates/module/domain/repository.py.j2"),
    },
    TemplateFile {
        dir: "module",
        rel: "infrastructure/__init__.py.j2",
        content: include_str!("templates/module/infrastructure/__init__.py.j2"),
    },
    TemplateFile {
        dir: "module",
        rel: "infrastructure/{{ snake }}_repository.py.j2",
        content: include_str!("templates/module/infrastructure/{{ snake }}_repository.py.j2"),
    },
    TemplateFile {
        dir: "module",
        rel: "tests/conftest.py.j2",
        content: include_str!("templates/module/tests/conftest.py.j2"),
    },
    TemplateFile {
        dir: "module",
        rel: "tests/test_{{ snake }}.py.j2",
        content: include_str!("templates/module/tests/test_{{ snake }}.py.j2"),
    },
    TemplateFile {
        dir: "resource",
        rel: "__init__.py.j2",
        content: include_str!("templates/resource/__init__.py.j2"),
    },
    TemplateFile {
        dir: "resource",
        rel: "api/__init__.py.j2",
        content: include_str!("templates/resource/api/__init__.py.j2"),
    },
    TemplateFile {
        dir: "resource",
        rel: "api/router.py.j2",
        content: include_str!("templates/resource/api/router.py.j2"),
    },
    TemplateFile {
        dir: "resource",
        rel: "application/__init__.py.j2",
        content: include_str!("templates/resource/application/__init__.py.j2"),
    },
    TemplateFile {
        dir: "resource",
        rel: "application/schemas.py.j2",
        content: include_str!("templates/resource/application/schemas.py.j2"),
    },
    TemplateFile {
        dir: "resource",
        rel: "application/{{ snake }}_service.py.j2",
        content: include_str!("templates/resource/application/{{ snake }}_service.py.j2"),
    },
    TemplateFile {
        dir: "resource",
        rel: "domain/__init__.py.j2",
        content: include_str!("templates/resource/domain/__init__.py.j2"),
    },
    TemplateFile {
        dir: "resource",
        rel: "domain/model.py.j2",
        content: include_str!("templates/resource/domain/model.py.j2"),
    },
    TemplateFile {
        dir: "resource",
        rel: "domain/repository.py.j2",
        content: include_str!("templates/resource/domain/repository.py.j2"),
    },
    TemplateFile {
        dir: "resource",
        rel: "infrastructure/__init__.py.j2",
        content: include_str!("templates/resource/infrastructure/__init__.py.j2"),
    },
    TemplateFile {
        dir: "resource",
        rel: "infrastructure/{{ snake }}_repository.py.j2",
        content: include_str!("templates/resource/infrastructure/{{ snake }}_repository.py.j2"),
    },
    TemplateFile {
        dir: "resource",
        rel: "tests/conftest.py.j2",
        content: include_str!("templates/resource/tests/conftest.py.j2"),
    },
    TemplateFile {
        dir: "resource",
        rel: "tests/test_{{ snake }}.py.j2",
        content: include_str!("templates/resource/tests/test_{{ snake }}.py.j2"),
    },
    TemplateFile {
        dir: "project",
        rel: ".env.example.j2",
        content: include_str!("templates/project/.env.example.j2"),
    },
    TemplateFile {
        dir: "project",
        rel: ".env.j2",
        content: include_str!("templates/project/.env.j2"),
    },
    TemplateFile {
        dir: "project",
        rel: ".gitignore.j2",
        content: include_str!("templates/project/.gitignore.j2"),
    },
    TemplateFile {
        dir: "project",
        rel: ".python-version.j2",
        content: include_str!("templates/project/.python-version.j2"),
    },
    TemplateFile {
        dir: "project",
        rel: "README.md.j2",
        content: include_str!("templates/project/README.md.j2"),
    },
    TemplateFile {
        dir: "project",
        rel: "pyproject.toml.j2",
        content: include_str!("templates/project/pyproject.toml.j2"),
    },
    TemplateFile {
        dir: "project",
        rel: "src/__init__.py.j2",
        content: include_str!("templates/project/src/__init__.py.j2"),
    },
    TemplateFile {
        dir: "project",
        rel: "src/main.py.j2",
        content: include_str!("templates/project/src/main.py.j2"),
    },
    TemplateFile {
        dir: "project",
        rel: "tests/__init__.py.j2",
        content: include_str!("templates/project/tests/__init__.py.j2"),
    },
    TemplateFile {
        dir: "project",
        rel: "tests/conftest.py.j2",
        content: include_str!("templates/project/tests/conftest.py.j2"),
    },
    TemplateFile {
        dir: "project",
        rel: "tests/test_health.py.j2",
        content: include_str!("templates/project/tests/test_health.py.j2"),
    },
];

#[must_use]
pub fn templates_in(dir: &str) -> Vec<&'static TemplateFile> {
    TEMPLATES.iter().filter(|t| t.dir == dir).collect()
}
