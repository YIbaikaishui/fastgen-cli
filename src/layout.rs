//! Project layout resolution: locate the source directory for a target project.
//!
//! fastgen supports two layouts:
//!   - ``src`` layout (recommended, created by ``fastgen new``): modules live in
//!     ``src/modules/``, the entrypoint is ``src/main.py``.
//!   - ``app`` layout (legacy): modules live in ``app/modules/``, entrypoint is
//!     ``app/main.py``.
//!
//! The layout is resolved from ``.fastgen.json`` (written by ``fastgen new``) and
//! falls back to auto-detection, then to ``app`` for backwards compatibility.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::writers::{write_file, GeneratedFile};

pub const CONFIG_FILE: &str = ".fastgen.json";
pub const DEFAULT_SOURCE: &str = "app";

/// Read the fastgen config for a project, or an empty map when absent/invalid.
#[must_use]
pub fn read_config(project_root: &Path) -> Map<String, Value> {
    let path = project_root.join(CONFIG_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| match v {
                Value::Object(map) => Some(map),
                _ => None,
            })
            .unwrap_or_default(),
        Err(_) => Map::new(),
    }
}

/// Return the source directory name (`src` or `app`) for a project.
#[must_use]
pub fn source_dir_name(project_root: &Path) -> String {
    let config = read_config(project_root);
    if let Some(Value::String(s)) = config.get("source_dir") {
        if !s.is_empty() {
            return s.clone();
        }
    }
    if project_root.join("src").join("main.py").exists() {
        return "src".to_string();
    }
    if project_root.join("app").join("main.py").exists() {
        return "app".to_string();
    }
    DEFAULT_SOURCE.to_string()
}

/// Persist the project config to `.fastgen.json`.
///
/// # Errors
///
/// Returns an [`io::Error`] when the file cannot be written.
pub fn write_config(
    project_root: &Path,
    config: &Value,
    dry_run: bool,
) -> io::Result<GeneratedFile> {
    let content = format!("{}\n", serde_json::to_string_pretty(config)?);
    write_file(&project_root.join(CONFIG_FILE), &content, true, dry_run)
}

/// Registry location: `<src>/modules/__init__.py`.
#[must_use]
pub fn registry_path(project_root: &Path, source: Option<&str>) -> PathBuf {
    let source = source.map_or_else(|| source_dir_name(project_root), ToString::to_string);
    project_root
        .join(source)
        .join("modules")
        .join("__init__.py")
}
