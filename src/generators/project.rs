//! Project generator: a best-practice `FastAPI` project scaffold.
//!
//! ``fastgen new <name>`` creates a ``src``-layout project that is immediately
//! manageable by the rest of fastgen-cli: `src/core/` (never overwritten),
//! `src/modules/__init__.py` (module registry), `.fastgen.json` (layout
//! config), plus ``.env``, ``src/main.py``, a ``tests/`` suite and the usual
//! tooling (``pyproject.toml``, ``.gitignore``, ``.python-version``).

use std::path::Path;

use minijinja::{context, Value};
use serde_json::json;

use crate::generators::{alembic, core, registry};
use crate::layout::write_config;
use crate::naming::{to_snake, to_title};
use crate::render::render_tree;
use crate::writers::{write_file, GeneratedFile};

const PROJECT_TEMPLATE: &str = "project";

/// # Errors
///
/// Returns an error when a template fails to render or a file cannot be written.
pub fn generate_project(
    name: &str,
    project_root: &Path,
    title: Option<&str>,
    description: &str,
    version: &str,
    force: bool,
    dry_run: bool,
) -> anyhow::Result<Vec<GeneratedFile>> {
    let project_name = to_snake(name);
    let ctx: Value = context! {
        source => "src",
        project_name => project_name,
        title => title.map_or_else(|| to_title(name), ToString::to_string),
        description => description,
        version => version,
    };

    let mut files = render_tree(PROJECT_TEMPLATE, &ctx, project_root, force, dry_run)?;
    files.extend(core::generate_core(project_root, Some("src"), dry_run)?);
    files.extend(alembic::generate_alembic(
        project_root,
        Some("src"),
        dry_run,
    )?);

    let registry = project_root.join("src").join("modules").join("__init__.py");
    if !registry.exists() || force {
        files.push(write_file(
            &registry,
            &registry::registry_content(&[]),
            true,
            dry_run,
        )?);
    }

    files.push(write_config(
        project_root,
        &json!({ "source_dir": "src" }),
        dry_run,
    )?);
    Ok(files)
}
