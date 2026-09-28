//! `fastgen doctor`: structure-drift detection for fastgen-managed projects.
//!
//! The registry and the auto-mount block are fastgen's contract with the app:
//! when they drift from what's on disk, the failure modes are silent (a module
//! nobody mounted) or loud (a stale entry crashing `main.py` at import time).
//! `doctor` makes both visible — and `--fix` repairs what it can.

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::generators::main::sync_main;
use crate::generators::registry::{read_registry, registry_content};
use crate::layout::source_dir_name;
use crate::validate::compile_check;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The app will crash or silently serve routes it shouldn't.
    Error,
    /// Drift that loses functionality without crashing.
    Warning,
}

impl Severity {
    fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    /// Repairable by `doctor --fix`.
    pub fixable: bool,
}

impl Issue {
    fn error(code: &'static str, message: impl Into<String>, fixable: bool) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            fixable,
        }
    }

    fn warning(code: &'static str, message: impl Into<String>, fixable: bool) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message: message.into(),
            fixable,
        }
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "severity": self.severity.as_str(),
            "code": self.code,
            "message": self.message,
            "fixable": self.fixable,
        })
    }
}

fn module_dirs(source_root: &Path) -> Vec<String> {
    let modules_dir = source_root.join("modules");
    let Ok(entries) = fs::read_dir(&modules_dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .filter(|name| !name.starts_with("__"))
        .collect();
    names.sort();
    names
}

/// Check a project for registry/mount/module drift.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) when files cannot be read.
pub fn check_project(project_root: &Path, source: Option<&str>) -> std::io::Result<Vec<Issue>> {
    let source = source.map_or_else(|| source_dir_name(project_root), String::from);
    let source_root = project_root.join(&source);
    let mut issues = Vec::new();

    if !source_root.join("main.py").exists() {
        issues.push(Issue::error(
            "no-entrypoint",
            format!("{source}/main.py not found — is this a fastgen project?"),
            false,
        ));
        return Ok(issues);
    }

    // Shared core must exist: every scaffolded module imports it.
    for file in ["core/config.py", "core/database.py"] {
        if !source_root.join(file).exists() {
            issues.push(Issue::error(
                "missing-core",
                format!("{source}/{file} is missing — modules import from core"),
                false,
            ));
        }
    }

    let registry_path = source_root.join("modules").join("__init__.py");
    let registry = read_registry(project_root, Some(&source));
    let registered: Vec<&String> = registry.iter().map(|(name, _)| name).collect();
    let on_disk = module_dirs(&source_root);

    if !registry_path.exists() && !on_disk.is_empty() {
        issues.push(Issue::error(
            "no-registry",
            format!(
                "{source}/modules/__init__.py is missing but {} module dir(s) exist — \
                 routers cannot auto-mount",
                on_disk.len()
            ),
            true,
        ));
    }

    // Stale entries: registered but gone from disk — crashes main.py on boot.
    for (name, path) in &registry {
        if !source_root.join("modules").join(name).is_dir() {
            issues.push(Issue::error(
                "stale-registry-entry",
                format!(
                    "registry entry '{name}' → '{path}' has no directory — \
                     main.py will crash at import time"
                ),
                true,
            ));
        }
    }

    // Unregistered modules: exist but nobody mounts them.
    for name in &on_disk {
        if !registered.contains(&name) {
            issues.push(Issue::warning(
                "unregistered-module",
                format!("{source}/modules/{name}/ exists but is not in the registry — its router is never mounted"),
                true,
            ));
        }
    }

    // Auto-mount block must exist and cover the registry.
    let main_py = fs::read_to_string(source_root.join("main.py"))?;
    if !main_py.contains("fastgen: auto-mount") {
        issues.push(Issue::error(
            "no-auto-mount",
            format!("{source}/main.py has no fastgen auto-mount block — routers are not served"),
            true,
        ));
    } else if registry.is_empty() && !on_disk.is_empty() {
        issues.push(Issue::error(
            "empty-registry",
            format!(
                "the registry is empty but {} module dir(s) exist on disk ({}) — no routes are served",
                on_disk.len(),
                on_disk.join(", ")
            ),
            true,
        ));
    }

    // Every registered module must expose `router` (the mount does `module.router`).
    for (name, _) in &registry {
        let module_dir = source_root.join("modules").join(name);
        if !module_dir.is_dir() {
            continue; // already reported as a stale entry
        }
        let init = module_dir.join("__init__.py");
        let exposes_router = fs::read_to_string(&init).is_ok_and(|text| text.contains("router"));
        if !exposes_router {
            issues.push(Issue::error(
                "module-without-router",
                format!("module '{name}' does not expose `router` from its __init__.py — the auto-mount will fail"),
                false,
            ));
        }
    }

    // Syntax: broken Python anywhere is a broken app.
    for (path, err) in compile_check(&source_root) {
        let shown = path.strip_prefix(project_root).unwrap_or(&path);
        issues.push(Issue::error(
            "syntax-error",
            format!("{} does not compile: {}", shown.display(), first_line(&err)),
            false,
        ));
    }

    Ok(issues)
}

/// The actual error line: Python tracebacks end with `SyntaxError: ...`.
fn first_line(text: &str) -> String {
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Repair what `check_project` flagged as fixable: register missing modules,
/// drop stale entries, (re)create the auto-mount block.
///
/// # Panics
///
/// Must not panic: the registry path's parent is always created first.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) when the registry or `main.py`
/// cannot be written.
pub fn fix_project(project_root: &Path, source: Option<&str>) -> std::io::Result<Vec<String>> {
    let source = source.map_or_else(|| source_dir_name(project_root), String::from);
    let source_root = project_root.join(&source);
    let mut actions = Vec::new();
    if !source_root.join("main.py").exists() {
        return Ok(actions);
    }

    let registry_path = source_root.join("modules").join("__init__.py");
    let on_disk = module_dirs(&source_root);

    // Rebuild the registry from disk: drop stale entries, add missing modules.
    let current = read_registry(project_root, Some(&source));
    let mut desired: Vec<(String, String)> = Vec::new();
    for name in &on_disk {
        let import_path = current
            .iter()
            .find(|(existing, _)| existing == name)
            .map_or_else(
                || format!("{source}.modules.{name}"),
                |(_, path)| path.clone(),
            );
        desired.push((name.clone(), import_path));
    }
    let content = registry_content(&desired);
    let existing = fs::read_to_string(&registry_path).unwrap_or_default();
    if existing.trim_end_matches('\n') != content.trim_end_matches('\n') {
        if let Some(parent) = registry_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&registry_path, &content)?;
        let removed: Vec<String> = current
            .iter()
            .filter(|(name, _)| !on_disk.contains(name))
            .map(|(name, _)| name.clone())
            .collect();
        let added: Vec<String> = on_disk
            .iter()
            .filter(|name| !current.iter().any(|(existing, _)| existing == *name))
            .cloned()
            .collect();
        if !removed.is_empty() {
            actions.push(format!(
                "registry: removed stale entries ({})",
                removed.join(", ")
            ));
        }
        if !added.is_empty() {
            actions.push(format!("registry: registered ({})", added.join(", ")));
        }
    }

    // (Re)create the auto-mount block if missing.
    let main_py = fs::read_to_string(source_root.join("main.py"))?;
    if !main_py.contains("fastgen: auto-mount") {
        sync_main(project_root, Some(&source), false)?;
        actions.push("main.py: injected the auto-mount block".to_string());
    }

    Ok(actions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::project::generate_project;

    fn project() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        generate_project("demo", tmp.path(), None, "", "0.1.0", false, false).unwrap();
        tmp
    }

    #[test]
    fn clean_project_has_no_issues() {
        let tmp = project();
        let issues = check_project(tmp.path(), None).unwrap();
        assert!(issues.is_empty(), "unexpected issues: {issues:?}");
    }

    #[test]
    fn detects_stale_registry_entry() {
        let tmp = project();
        // Register a module, then delete its directory.
        crate::generators::module::generate_module("ghost", tmp.path(), Some("src"), false, false)
            .unwrap();
        crate::generators::registry::register_module(tmp.path(), "ghost", Some("src"), false)
            .unwrap();
        fs::remove_dir_all(tmp.path().join("src").join("modules").join("ghost")).unwrap();

        let issues = check_project(tmp.path(), None).unwrap();
        let stale = issues.iter().find(|i| i.code == "stale-registry-entry");
        assert!(stale.is_some(), "stale entry not detected: {issues:?}");
        assert_eq!(stale.unwrap().severity, Severity::Error);
        assert!(stale.unwrap().fixable);
    }

    #[test]
    fn detects_unregistered_module() {
        let tmp = project();
        fs::create_dir_all(
            tmp.path()
                .join("src")
                .join("modules")
                .join("orphan")
                .join("api"),
        )
        .unwrap();
        fs::write(
            tmp.path()
                .join("src")
                .join("modules")
                .join("orphan")
                .join("api")
                .join("router.py"),
            "router = None\n",
        )
        .unwrap();
        fs::write(
            tmp.path()
                .join("src")
                .join("modules")
                .join("orphan")
                .join("__init__.py"),
            "from .api.router import router\n__all__ = ['router']\n",
        )
        .unwrap();

        let issues = check_project(tmp.path(), None).unwrap();
        let orphan = issues.iter().find(|i| i.code == "unregistered-module");
        assert!(orphan.is_some(), "orphan not detected: {issues:?}");
        assert!(orphan.unwrap().fixable);
    }

    #[test]
    fn detects_missing_auto_mount() {
        let tmp = project();
        let main_py = tmp.path().join("src").join("main.py");
        let text = fs::read_to_string(&main_py).unwrap();
        let stripped: String = text
            .lines()
            .filter(|l| {
                !l.contains("fastgen: auto-mount")
                    && !l.contains("_module")
                    && !l.contains("for _import_path")
            })
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&main_py, stripped).unwrap();

        let issues = check_project(tmp.path(), None).unwrap();
        assert!(issues.iter().any(|i| i.code == "no-auto-mount"));
    }

    #[test]
    fn detects_syntax_error() {
        let tmp = project();
        fs::write(
            tmp.path().join("src").join("modules").join("broken.py"),
            "def (:\n",
        )
        .unwrap();
        let issues = check_project(tmp.path(), None).unwrap();
        assert!(issues.iter().any(|i| i.code == "syntax-error"));
    }

    #[test]
    fn fix_registers_orphans_and_removes_stale_entries() {
        let tmp = project();
        // stale entry
        crate::generators::module::generate_module("ghost", tmp.path(), Some("src"), false, false)
            .unwrap();
        crate::generators::registry::register_module(tmp.path(), "ghost", Some("src"), false)
            .unwrap();
        fs::remove_dir_all(tmp.path().join("src").join("modules").join("ghost")).unwrap();
        // orphan
        fs::create_dir_all(tmp.path().join("src").join("modules").join("orphan")).unwrap();

        let actions = fix_project(tmp.path(), None).unwrap();
        assert!(!actions.is_empty());
        let issues = check_project(tmp.path(), None).unwrap();
        assert!(
            !issues
                .iter()
                .any(|i| i.code == "stale-registry-entry" || i.code == "unregistered-module"),
            "issues remain after fix: {issues:?}"
        );
        let registry = read_registry(tmp.path(), None);
        assert!(registry.iter().any(|(name, _)| name == "orphan"));
        assert!(!registry.iter().any(|(name, _)| name == "ghost"));
    }

    #[test]
    fn fix_reinjects_auto_mount() {
        let tmp = project();
        let main_py = tmp.path().join("src").join("main.py");
        let text = fs::read_to_string(&main_py).unwrap();
        let stripped: String = text
            .lines()
            .filter(|l| {
                !l.contains("fastgen: auto-mount")
                    && !l.contains("_module")
                    && !l.contains("for _import_path")
            })
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&main_py, stripped).unwrap();

        fix_project(tmp.path(), None).unwrap();
        assert!(fs::read_to_string(&main_py)
            .unwrap()
            .contains("fastgen: auto-mount"));
    }

    #[test]
    fn issues_serialize_to_json() {
        let issue = Issue::warning("unregistered-module", "msg", true);
        let json = issue.to_json();
        assert_eq!(json["severity"], "warning");
        assert_eq!(json["code"], "unregistered-module");
        assert_eq!(json["fixable"], true);
    }
}
