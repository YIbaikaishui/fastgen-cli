//! Resource generator: a **working** CRUD vertical slice.
//!
//! Where [`crate::generators::module`] sketches a module's shape for a human (or
//! an AI agent) to fill in, a resource is generated *complete*: real columns
//! from `--fields`, a service that actually maps payloads onto the entity,
//! offset pagination, and a test suite that exercises the whole CRUD loop.
//!
//! Resources live in the same place modules do (`<src>/modules/<name>/`), so the
//! registry, the `main.py` auto-mount, `fastgen list` and `fastgen doctor` all
//! work on them unchanged.

use std::path::Path;

use minijinja::{context, Value};

use crate::fields::Fields;
use crate::layout::source_dir_name;
use crate::naming::{to_kebab, to_pascal, to_plural, to_snake};
use crate::render::render_tree;
use crate::writers::GeneratedFile;

const RESOURCE_TEMPLATE: &str = "resource";

/// Join import groups into a flat list of lines, blank line between groups.
/// Empty groups are dropped so no stray blank line is emitted.
fn import_block(groups: Vec<Vec<String>>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for group in groups.into_iter().filter(|g| !g.is_empty()) {
        if !out.is_empty() {
            out.push(String::new());
        }
        out.extend(group);
    }
    out
}

/// The template context for a resource with the given fields.
///
/// # Errors
///
/// Returns an error when the context cannot be built.
pub fn resource_context(name: &str, source: &str, fields: &Fields) -> anyhow::Result<Value> {
    let snake = to_snake(name);
    let model_imports = import_block(vec![
        fields.imports(),
        vec![
            format!(
                "from sqlalchemy import {}",
                fields.column_types().join(", ")
            ),
            "from sqlalchemy.orm import Mapped, mapped_column".to_string(),
        ],
        vec![format!("from {source}.core.database import Base")],
    ]);
    let schema_imports = import_block(vec![
        fields.schema_imports(),
        vec!["from pydantic import BaseModel, ConfigDict".to_string()],
    ]);
    // isort puts `application.schemas` before `application.<snake>_service` only
    // for names sorting after "schemas", so the order is computed, not assumed.
    let pascal = to_pascal(name);
    let mut app_imports = vec![
        format!(
            "from {source}.modules.{snake}.application.schemas import (\n    {pascal}Create,\n    {pascal}Page,\n    {pascal}Read,\n    {pascal}Update,\n)"
        ),
        format!(
            "from {source}.modules.{snake}.application.{snake}_service import (\n    {pascal}NotFound,\n    {pascal}Service,\n)"
        ),
    ];
    app_imports.sort();

    let schema_items: Vec<Value> = fields
        .items
        .iter()
        .map(|f| {
            context! {
                name => f.name,
                annotation => f.annotation(),
                update_annotation => f.update_annotation(),
                default => if f.optional { " = None" } else { "" },
                column => f.column_expr(),
                sample => f.ty.sample(),
                optional => f.optional,
            }
        })
        .collect();
    Ok(context! {
        pascal => to_pascal(name),
        snake => snake,
        kebab => to_kebab(name),
        plural => to_plural(&snake),
        source => source,
        import_lines => model_imports,
        app_imports => app_imports,
        schema_import_lines => schema_imports,
        has_required => fields.first_required().is_some(),
        fields => schema_items,
    })
}

/// Generate a complete CRUD resource for `name` into
/// `<project_root>/<source>/modules/<name>/`.
///
/// `fields_spec` is the raw `--fields` value; when `None` the resource gets a
/// single `name: str` field.
///
/// # Errors
///
/// Returns an error when the spec is invalid, a template fails to render, or a
/// file cannot be written.
pub fn generate_resource(
    name: &str,
    project_root: &Path,
    source: Option<&str>,
    fields_spec: Option<&str>,
    force: bool,
    dry_run: bool,
) -> anyhow::Result<Vec<GeneratedFile>> {
    let source = source.map_or_else(|| source_dir_name(project_root), ToString::to_string);
    let fields = match fields_spec {
        Some(spec) => crate::fields::parse_fields(spec)?,
        None => Fields::fallback(),
    };
    let ctx = resource_context(name, &source, &fields)?;
    let module_dir = project_root
        .join(&source)
        .join("modules")
        .join(to_snake(name));
    render_tree(RESOURCE_TEMPLATE, &ctx, &module_dir, force, dry_run)
}
