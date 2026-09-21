//! Entrypoint sync: ensure ``<src>/main.py`` auto-mounts registry routers.
//!
//! `fastgen make module` calls [`sync_main`] so a freshly scaffolded router is
//! served without hand-editing `main.py`. Injection is marker-based and
//! idempotent; existing hand-written code is left untouched.

use std::fs;
use std::path::Path;

use crate::layout::source_dir_name;
use crate::writers::{write_file, GeneratedFile, Status};

pub const MOUNT_MARKER: &str = "# --- fastgen: auto-mount (do not remove) ---";
pub const MOUNT_END_MARKER: &str = "# --- fastgen: end auto-mount ---";

const MOUNT_BLOCK: &[&str] = &[
    MOUNT_MARKER,
    "for _import_path in modules.values():",
    "    _module = importlib.import_module(_import_path)",
    "    app.include_router(_module.router)",
    MOUNT_END_MARKER,
];

fn already_mounted(content: &str) -> bool {
    content.contains(MOUNT_MARKER) || content.contains("app.include_router(_module.router)")
}

/// Split the import region into blank-line-separated groups.
///
/// Returns ``(groups, end)`` where ``end`` is the index just past the region
/// (the next non-import, non-blank line).
fn import_groups(lines: &[String], idx: usize) -> (Vec<Vec<String>>, usize) {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut end = idx;
    while end < lines.len() {
        let line = &lines[end];
        if line.starts_with("import ") || line.starts_with("from ") {
            current.push(line.clone());
        } else if line.is_empty() {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
        } else {
            break;
        }
        end += 1;
    }
    if !current.is_empty() {
        groups.push(current);
    }
    (groups, end)
}

/// Insert a plain ``import x`` after ``from __future__`` and existing plain imports.
fn insert_plain_import(group: &mut Vec<String>, imp: &str) {
    let mut pos = 0;
    while pos < group.len() && group[pos].starts_with("from __future__") {
        pos += 1;
    }
    while pos < group.len() && group[pos].starts_with("import ") && group[pos].as_str() < imp {
        pos += 1;
    }
    group.insert(pos, imp.to_string());
}

/// Place ``from <source>.modules import modules`` in the first-party import group.
///
/// Ruff's isort keeps ``<source>.*`` imports together, so the new import is inserted
/// alphabetically inside the contiguous ``from <source>.`` run (a fresh group is
/// appended when no such run exists).
fn insert_module_import(groups: &mut Vec<Vec<String>>, imp: &str, source: &str) {
    let prefix = format!("from {source}.");
    for group in groups.iter_mut() {
        let Some(run_start) = group.iter().position(|line| line.starts_with(&prefix)) else {
            continue;
        };
        let mut run_end = run_start;
        while run_end + 1 < group.len() && group[run_end + 1].starts_with(&prefix) {
            run_end += 1;
        }
        let mut pos = run_start;
        for line in &group[run_start..=run_end] {
            if line.as_str() < imp {
                pos += 1;
            } else {
                break;
            }
        }
        group.insert(pos, imp.to_string());
        return;
    }
    groups.push(vec![imp.to_string()]);
}

/// Ensure the registry imports exist in the import region of `main.py`.
fn insert_imports(lines: &[String], source: &str) -> Vec<String> {
    let mut lines = lines.to_vec();
    let mut idx = 0;
    if let Some(first) = lines.first() {
        if first.starts_with("\"\"\"") {
            if first.len() >= 6 && first.trim_end().ends_with("\"\"\"") {
                idx = 1;
            } else {
                idx = lines
                    .iter()
                    .enumerate()
                    .skip(1)
                    .find(|(i, line)| *i > 0 && line.trim_end().ends_with("\"\"\""))
                    .map_or(lines.len(), |(i, _)| i + 1);
            }
        }
    }
    let (mut groups, end) = import_groups(&lines, idx);
    let mut trailing = 0;
    let mut cursor = end;
    while cursor > idx && lines[cursor - 1].is_empty() {
        trailing += 1;
        cursor -= 1;
    }
    if !lines
        .iter()
        .any(|line| line.starts_with("import importlib"))
    {
        if groups.is_empty() {
            groups = vec![vec!["import importlib".to_string()]];
        } else {
            insert_plain_import(&mut groups[0], "import importlib");
        }
    }
    let module_imp = format!("from {source}.modules import modules");
    if !lines.iter().any(|line| line.starts_with(&module_imp)) {
        insert_module_import(&mut groups, &module_imp, source);
    }
    let mut region: Vec<String> = Vec::new();
    for (i, group) in groups.iter().enumerate() {
        if i > 0 {
            region.push(String::new());
        }
        region.extend(group.iter().cloned());
    }
    region.extend(std::iter::repeat_n(String::new(), trailing));
    lines.splice(idx..end, region);
    lines
}

/// Inject the registry auto-mount loop into `<src>/main.py` (idempotent).
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) when `main.py` cannot be read or written.
pub fn sync_main(
    project_root: &Path,
    source: Option<&str>,
    dry_run: bool,
) -> std::io::Result<GeneratedFile> {
    let source = source.map_or_else(|| source_dir_name(project_root), ToString::to_string);
    let main_path = project_root.join(&source).join("main.py");
    if !main_path.exists() {
        return Ok(GeneratedFile {
            path: main_path,
            content: String::new(),
            status: Status::Skipped,
        });
    }
    let content = fs::read_to_string(&main_path)?;
    if already_mounted(&content) {
        return Ok(GeneratedFile {
            path: main_path,
            content,
            status: Status::Skipped,
        });
    }

    let mut lines: Vec<String> = content.lines().map(ToOwned::to_owned).collect();
    lines = insert_imports(&lines, &source);
    let start = lines
        .iter()
        .position(|line| line.starts_with("app = FastAPI("));
    let new_content = match start {
        None => format!(
            "{}\n{}",
            content.trim_end_matches('\n'),
            MOUNT_BLOCK.join("\n")
        ),
        Some(start) => {
            let mut depth: usize = 0;
            let mut end = start;
            for (i, line) in lines.iter().enumerate().skip(start) {
                let opens = line.matches('(').count();
                let closes = line.matches(')').count();
                depth = depth.saturating_add(opens).saturating_sub(closes);
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            lines.splice(
                (end + 1)..=end,
                MOUNT_BLOCK.iter().map(|s| (*s).to_string()),
            );
            format!("{}\n", lines.join("\n"))
        }
    };
    write_file(&main_path, &new_content, true, dry_run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_imports_respects_docstring_and_future() {
        let lines: Vec<String> = [
            "\"\"\"Entrypoint.\"\"\"",
            "from __future__ import annotations",
            "",
            "from collections.abc import AsyncGenerator",
            "from fastapi import FastAPI",
            "",
            "from app.core.database import Base, engine",
            "from app.modules.user.router import router as user_router",
            "",
            "app = FastAPI()",
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        let out = insert_imports(&lines, "app");
        let block: Vec<&String> = out
            .iter()
            .filter(|l| l.starts_with("import ") || l.starts_with("from "))
            .collect();
        let index_of = |needle: &str| block.iter().position(|l| l.as_str() == needle).unwrap();
        assert_eq!(index_of("from __future__ import annotations"), 0);
        assert!(index_of("import importlib") < index_of("from fastapi import FastAPI"));
        assert!(
            index_of("from app.modules import modules")
                < index_of("from app.modules.user.router import router as user_router")
        );
        assert!(
            index_of("from fastapi import FastAPI") < index_of("from app.modules import modules")
        );
    }
}
