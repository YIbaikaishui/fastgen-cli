//! Core scaffolding generator: ``<src>/core/`` (config.py + database.py).
//!
//! Files are only generated when they are missing or empty; existing code is
//! never overwritten (``force`` does not apply here).

use std::fs;
use std::path::Path;

use minijinja::{context, Value};

use crate::layout::source_dir_name;
use crate::render::render_template;
use crate::writers::{write_file, GeneratedFile};

const CORE_TEMPLATE: &str = "core";

fn has_code(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|text| !text.trim().is_empty())
}

/// Generate `<src>/core/` when files are missing or empty; existing code is
/// never overwritten (`force` does not apply here).
///
/// # Errors
///
/// Returns an error when a template fails to render or a file cannot be written.
pub fn generate_core(
    project_root: &Path,
    source: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Vec<GeneratedFile>> {
    let source = source.map_or_else(|| source_dir_name(project_root), ToString::to_string);
    let core_dir = project_root.join(&source).join("core");
    let mut files: Vec<GeneratedFile> = Vec::new();
    let ctx: Value = context! { source => source };

    let init_py = core_dir.join("__init__.py");
    if !init_py.exists() {
        files.push(write_file(&init_py, "", false, dry_run)?);
    }

    let config_py = core_dir.join("config.py");
    if !has_code(&config_py) {
        let content = render_template(CORE_TEMPLATE, "config.py.j2", &ctx)?;
        files.push(write_file(&config_py, &content, true, dry_run)?);
    }

    let database_py = core_dir.join("database.py");
    if !has_code(&database_py) {
        let content = render_template(CORE_TEMPLATE, "database.py.j2", &ctx)?;
        files.push(write_file(&database_py, &content, true, dry_run)?);
    }

    Ok(files)
}
