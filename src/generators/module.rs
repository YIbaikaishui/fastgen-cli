//! Feature-module generator: vertical-slice module skeleton.
//!
//! Each scaffolded module is layered under ``modules/<feature>/``:
//! `domain/` (model + repository port), `application/` (schemas + service),
//! `infrastructure/` (`SQLAlchemy` repository), `api/` (router), plus `tests/`.
//! The skeleton only sketches the module's shape so AI agents can reason about it;
//! real business code is filled in by the developer.

use std::path::Path;

use minijinja::{context, Value};

use crate::layout::source_dir_name;
use crate::naming::{to_kebab, to_pascal, to_plural, to_snake};
use crate::render::render_tree;
use crate::writers::GeneratedFile;

const MODULE_TEMPLATE: &str = "module";

#[must_use]
pub fn module_context(feature: &str, source: &str) -> Value {
    let snake = to_snake(feature);
    context! {
        pascal => to_pascal(feature),
        snake => snake,
        kebab => to_kebab(feature),
        plural => to_plural(&to_snake(feature)),
        source => source,
    }
}

/// # Errors
///
/// Returns an error when a template fails to render or a file cannot be written.
pub fn generate_module(
    feature: &str,
    project_root: &Path,
    source: Option<&str>,
    force: bool,
    dry_run: bool,
) -> anyhow::Result<Vec<GeneratedFile>> {
    let source = source.map_or_else(|| source_dir_name(project_root), ToString::to_string);
    let snake = to_snake(feature);
    let module_dir = project_root.join(&source).join("modules").join(&snake);
    render_tree(
        MODULE_TEMPLATE,
        &module_context(feature, &source),
        &module_dir,
        force,
        dry_run,
    )
}
