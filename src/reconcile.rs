//! Post-agent reconciliation: make the registry and auto-mount block agree with
//! what is actually on disk after an agent ran.
//!
//! Agents sometimes create or rename modules; `reconcile_registry` re-scans
//! `<src>/modules/` and registers anything missing, so the registry stays the
//! single source of truth.

use std::fs;
use std::path::Path;

use crate::generators::main::sync_main;
use crate::generators::registry::{read_registry, register_module};
use crate::layout::source_dir_name;
use crate::writers::GeneratedFile;

/// Register any module directory that is missing from the registry, then
/// re-sync the `main.py` auto-mount block (idempotent).
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) when the registry or `main.py`
/// cannot be written.
pub fn reconcile_registry(
    project_root: &Path,
    source: Option<&str>,
) -> std::io::Result<Vec<GeneratedFile>> {
    let source = source.map_or_else(|| source_dir_name(project_root), ToString::to_string);
    let modules_dir = project_root.join(&source).join("modules");
    let Ok(entries) = fs::read_dir(&modules_dir) else {
        return Ok(Vec::new());
    };

    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with("__") {
            continue;
        }
        let registered = read_registry(project_root, Some(&source))
            .iter()
            .any(|(existing, _)| existing == name);
        if !registered {
            files.push(register_module(project_root, name, Some(&source), false)?);
        }
    }
    files.push(sync_main(project_root, Some(&source), false)?);
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::project::generate_project;

    #[test]
    fn reconcile_registers_agent_created_modules() {
        let tmp = tempfile::tempdir().unwrap();
        generate_project("demo", tmp.path(), None, "", "0.1.0", false, false).unwrap();
        // Simulate an agent that created a module directory but did not register it.
        let module_dir = tmp.path().join("src").join("modules").join("invoice");
        fs::create_dir_all(module_dir.join("api")).unwrap();
        fs::write(module_dir.join("api").join("router.py"), "router = None\n").unwrap();

        reconcile_registry(tmp.path(), None).unwrap();

        let registry = read_registry(tmp.path(), None);
        assert!(
            registry
                .iter()
                .any(|(name, path)| name == "invoice" && path == "src.modules.invoice"),
            "agent-created module should be registered: {registry:?}"
        );
    }

    #[test]
    fn reconcile_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        generate_project("demo", tmp.path(), None, "", "0.1.0", false, false).unwrap();
        let module_dir = tmp.path().join("src").join("modules").join("invoice");
        fs::create_dir_all(module_dir).unwrap();

        reconcile_registry(tmp.path(), None).unwrap();
        let before =
            fs::read_to_string(tmp.path().join("src").join("modules").join("__init__.py")).unwrap();
        reconcile_registry(tmp.path(), None).unwrap();
        let after =
            fs::read_to_string(tmp.path().join("src").join("modules").join("__init__.py")).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn reconcile_keeps_existing_registry() {
        let tmp = tempfile::tempdir().unwrap();
        generate_project("demo", tmp.path(), None, "", "0.1.0", false, false).unwrap();
        crate::generators::module::generate_module("user", tmp.path(), Some("src"), false, false)
            .unwrap();
        crate::generators::registry::register_module(tmp.path(), "user", Some("src"), false)
            .unwrap();
        reconcile_registry(tmp.path(), None).unwrap();
        let registry = read_registry(tmp.path(), None);
        assert_eq!(registry.len(), 1);
        assert_eq!(registry[0].0, "user");
    }
}
