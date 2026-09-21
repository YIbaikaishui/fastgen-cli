//! Minijinja rendering helpers shared by all generators.

use std::path::Path;

use anyhow::{anyhow, Context};
use minijinja::{Environment, UndefinedBehavior, Value};

use crate::templates::templates_in;
use crate::writers::{write_file, GeneratedFile};

fn environment() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    env
}

/// Render a single template file without writing anything.
///
/// # Errors
///
/// Returns an error when the template is missing or fails to render.
pub fn render_template(dir: &str, name: &str, ctx: &Value) -> anyhow::Result<String> {
    let env = environment();
    let entry = templates_in(dir)
        .into_iter()
        .find(|t| t.rel == name)
        .ok_or_else(|| anyhow!("template not found: {dir}/{name}"))?;
    env.render_str(entry.content, ctx)
        .with_context(|| format!("failed to render template {dir}/{name}"))
}

/// Render every template in `dir` into `dest_dir` (sorted by path, like the Python version).
///
/// # Errors
///
/// Returns an error when a template (or a template *path*) fails to render, or
/// when a target file cannot be written.
pub fn render_tree(
    dir: &str,
    ctx: &Value,
    dest_dir: &Path,
    force: bool,
    dry_run: bool,
) -> anyhow::Result<Vec<GeneratedFile>> {
    let env = environment();
    let mut entries: Vec<_> = templates_in(dir);
    entries.sort_by(|a, b| a.rel.cmp(b.rel));
    let mut files = Vec::with_capacity(entries.len());
    for entry in entries {
        // The relative template path may itself contain Jinja expressions
        // (e.g. "{{ snake }}_service.py.j2"); render it, then drop the ".j2" suffix.
        let rel_name = env
            .render_str(entry.rel, ctx)
            .with_context(|| format!("failed to render template path {}", entry.rel))?;
        let target_rel = rel_name
            .strip_suffix(".j2")
            .unwrap_or(rel_name.as_str())
            .to_string();
        let rendered = env
            .render_str(entry.content, ctx)
            .with_context(|| format!("failed to render template {dir}/{rel_name}"))?;
        files.push(write_file(
            &dest_dir.join(target_rel),
            &rendered,
            force,
            dry_run,
        )?);
    }
    Ok(files)
}
