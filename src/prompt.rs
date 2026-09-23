//! Prompt construction for AI agents — the structure contract fastgen hands to
//! codex/opencode.
//!
//! The prompt is fastgen's real value in the AI flow: it encodes the module
//! conventions, the file tree to fill in, and the rest of the project's module
//! map, so the agent writes code *inside* the structure instead of beside it.

use std::path::Path;

use anyhow::Context;

use crate::generators::{module::module_context, registry::read_registry};
use crate::naming::to_pascal;
use crate::render::rendered_rel_paths;

/// The conventions fastgen scaffolds follow. Kept terse — this ships to a model.
const CONVENTIONS: &str = "\
- Services are async and free of HTTP; routers map domain exceptions (e.g. XNotFound) to HTTPException.
- Sessions come from `{source}.core.database.get_session`; the router defines `SessionDep = Annotated[AsyncSession, Depends(get_session)]`.
- Models use SQLAlchemy 2.0 style (`Mapped[...]` / `mapped_column(...)`), inherit `Base` from `{source}.core.database`, and set `__tablename__` to the plural name.
- Schemas are `XBase` / `XCreate` / `XUpdate` / `XRead`; `XRead` sets `model_config = ConfigDict(from_attributes=True)`.
- The repository is a `Protocol` port in `domain/repository.py`, implemented as `SqlXRepository` in `infrastructure/`.
- The service defines an error hierarchy: `XError(Exception)` plus specific subclasses like `XNotFound`.
- The router exposes `prefix=\"/{plural}\"`, tags `[\"{plural}\"]`, and full CRUD endpoints (create/list/get/update/delete).
- The database schema is managed by Alembic migrations — never call `create_all` at runtime.
- Keep everything async. Do not add dependencies unless the task explicitly requires it.";

/// One-line purpose per scaffolded template path (relative, with `{{ snake }}` intact).
const FILE_PURPOSES: &[(&str, &str)] = &[
    ("__init__.py", "re-exports the router"),
    ("domain/__init__.py", "domain package marker"),
    ("domain/model.py", "SQLAlchemy entity: fields + table"),
    (
        "domain/repository.py",
        "repository Protocol port (add/get/list/delete)",
    ),
    ("application/__init__.py", "application package marker"),
    (
        "application/schemas.py",
        "Pydantic XBase/XCreate/XUpdate/XRead",
    ),
    (
        "application/{{ snake }}_service.py",
        "async service: use cases + XError/XNotFound hierarchy",
    ),
    (
        "infrastructure/__init__.py",
        "infrastructure package marker",
    ),
    (
        "infrastructure/{{ snake }}_repository.py",
        "SQLAlchemy implementation of the repository port",
    ),
    ("api/__init__.py", "api package marker"),
    (
        "api/router.py",
        "FastAPI router: CRUD endpoints, SessionDep, exception mapping",
    ),
    (
        "tests/conftest.py",
        "in-memory SQLite fixtures + dependency override",
    ),
    ("tests/test_{{ snake }}.py", "module smoke tests"),
];

fn purpose(rel: &str) -> &str {
    FILE_PURPOSES
        .iter()
        .find(|(path, _)| *path == rel)
        .map_or("module file", |(_, purpose)| *purpose)
}

fn conventions(source: &str, plural: &str) -> String {
    CONVENTIONS
        .replace("{source}", source)
        .replace("{plural}", plural)
}

fn module_map(project_root: &Path, source: &str) -> String {
    let entries = read_registry(project_root, Some(source));
    if entries.is_empty() {
        return "- (none yet — this will be the first module)".to_string();
    }
    entries
        .iter()
        .map(|(name, path)| format!("- {name} → {path}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn default_module_task(pascal: &str, snake: &str) -> String {
    format!(
        "Implement a complete, idiomatic CRUD for the {pascal} module: sensible model fields for a {snake}, schemas, service logic (including the XNotFound paths), repository queries, and working router endpoints. Add a short docstring to every public class and function. Do not write business logic that needs external services."
    )
}

/// Build the prompt for `fastgen make module <feature>` (or `new --ai`).
///
/// # Errors
///
/// Returns an error when the module template paths cannot be rendered.
pub fn build_module_prompt(
    feature: &str,
    source: &str,
    project_root: &Path,
    task: Option<&str>,
) -> anyhow::Result<String> {
    let ctx = module_context(feature, source);
    let snake = ctx
        .get_attr("snake")
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| crate::naming::to_snake(feature));
    let plural = ctx
        .get_attr("plural")
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| crate::naming::to_plural(&snake));
    let pascal = to_pascal(feature);
    let module_dir = format!("{source}/modules/{snake}");

    let files = rendered_rel_paths("module", &ctx)
        .with_context(|| "failed to build the module file list")?
        .iter()
        .map(|rel| format!("- {module_dir}/{rel} — {}", purpose(rel)))
        .collect::<Vec<_>>()
        .join("\n");

    let task = task.map_or_else(|| default_module_task(&pascal, &snake), String::from);

    Ok(format!(
        "You are completing a FastAPI feature module scaffolded by fastgen (structure-first development).\n\
         The scaffold is deterministic: fill it in with real code. Do NOT move, rename, or delete the existing files.\n\
\n\
         ## Module\n\
         - Feature: {feature} ({pascal})\n\
         - Directory: {module_dir}/ (relative to the project root)\n\
         - Import path: {source}.modules.{snake}\n\
         - Router prefix: /{plural}\n\
\n\
         ## Files to fill in (already created)\n\
         {files}\n\
\n\
         ## Other modules registered in this project\n\
         {map}\n\
\n\
         ## Conventions (fastgen's structure contract)\n\
         {conventions}\n\
\n\
         ## Task\n\
         {task}\n\
\n\
         Rules:\n\
         - Only create/edit files inside {module_dir}/.\n\
         - Do not run servers, tests, git, or migrations.\n\
         - After editing, verify each touched .py file compiles with `python -m py_compile <file>`.\n\
         - Prefer editing existing skeletons over adding new files.",
        map = module_map(project_root, source),
        conventions = conventions(source, &plural),
    ))
}

/// Build the prompt for `fastgen new <name>`: complete the project and, if a spec
/// is given, create the modules it describes (registering each in the registry).
///
/// # Errors
///
/// Returns an error when the registry cannot be read.
pub fn build_project_prompt(
    name: &str,
    source: &str,
    project_root: &Path,
    task: Option<&str>,
) -> anyhow::Result<String> {
    let task = task.map_or_else(
        || {
            format!(
                "Verify the scaffolded `{name}` project is complete and consistent — config, database, entrypoint, tests and the module registry should all be wired. Fix anything obviously broken; do not add features."
            )
        },
        String::from,
    );
    let registry_hint = format!("{source}/modules/__init__.py");
    let map = module_map(project_root, source);
    Ok(format!(
        "You are setting up a FastAPI project scaffolded by fastgen (structure-first development).\n\
\n\
         ## Project layout\n\
         - Source root: `{source}/` (src layout)\n\
         - Shared core: `{source}/core/` (config.py + database.py — never overwrite existing content)\n\
         - Module registry: `{registry_hint}` — a plain `modules: dict[str, str]` mapping module name → import path\n\
         - Entrypoint: `{source}/main.py` — routers auto-mount from the registry via a `# --- fastgen: auto-mount ---` block (do not edit that block by hand)\n\
         - Migrations: `migrations/` (Alembic; schema is never created at runtime)\n\
\n\
         ## Modules currently registered\n\
         {map}\n\
\n\
         ## Module structure (follow exactly when adding modules)\n\
         Each `{source}/modules/<feature>/` is a vertical slice: `domain/` (model.py + repository.py Protocol), `application/` (schemas.py + <feature>_service.py), `infrastructure/` (<feature>_repository.py), `api/` (router.py), `tests/`.\n\
         When you create a module, ALSO add it to the registry dict (`\"<feature>\": \"{source}.modules.<feature>\"`) — routers then mount automatically.\n\
\n\
         ## Conventions (fastgen's structure contract)\n\
         {conventions}\n\
\n\
         ## Task\n\
         {task}\n\
\n\
         Rules:\n\
         - Stay inside the project directory.\n\
         - Do not run servers, tests, git, or migrations.\n\
         - After editing, verify each touched .py file compiles with `python -m py_compile <file>`.",
        conventions = conventions(source, "items"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        crate::generators::project::generate_project(
            "demo",
            tmp.path(),
            None,
            "",
            "0.1.0",
            false,
            false,
        )
        .unwrap();
        tmp
    }

    #[test]
    fn module_prompt_contains_structure_and_task() {
        let tmp = project();
        let prompt =
            build_module_prompt("order", "src", tmp.path(), Some("add soft delete")).unwrap();
        assert!(prompt.contains("src/modules/order/"));
        assert!(prompt.contains("api/router.py"));
        assert!(prompt.contains("Router prefix: /orders"));
        assert!(prompt.contains("add soft delete"));
        assert!(prompt.contains("XNotFound"));
        assert!(prompt.contains("from_attributes=True"));
        assert!(prompt.contains("SessionDep"));
        assert!(prompt.contains("src.modules.order"));
    }

    #[test]
    fn module_prompt_default_task_mentions_feature() {
        let tmp = project();
        let prompt = build_module_prompt("invoice", "src", tmp.path(), None).unwrap();
        assert!(prompt.contains("Invoice"));
        assert!(prompt.contains("CRUD"));
    }

    #[test]
    fn module_prompt_lists_other_registered_modules() {
        let tmp = project();
        crate::generators::module::generate_module("user", tmp.path(), Some("src"), false, false)
            .unwrap();
        crate::generators::registry::register_module(tmp.path(), "user", Some("src"), false)
            .unwrap();
        let prompt = build_module_prompt("order", "src", tmp.path(), None).unwrap();
        assert!(prompt.contains("user → src.modules.user"));
    }

    #[test]
    fn project_prompt_contains_layout_and_registry_rule() {
        let tmp = project();
        let prompt = build_project_prompt("demo", "src", tmp.path(), None).unwrap();
        assert!(prompt.contains("src/main.py"));
        assert!(prompt.contains("auto-mount"));
        assert!(prompt.contains("registry dict"));
        assert!(
            prompt.contains("Never call create_all".to_lowercase().as_str())
                || prompt.contains("create_all")
        );
    }

    #[test]
    fn project_prompt_uses_custom_task() {
        let tmp = project();
        let prompt = build_project_prompt(
            "demo",
            "src",
            tmp.path(),
            Some("task manager with projects"),
        )
        .unwrap();
        assert!(prompt.contains("task manager with projects"));
    }
}
