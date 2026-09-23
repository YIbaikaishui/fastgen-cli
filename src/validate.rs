//! Post-agent validation: report which files the agent touched and syntax-check
//! them. fastgen verifies what the agent wrote — the governance half of the flow.

use std::collections::HashMap;
use std::fs;
use std::hash::Hasher;
use std::hash::{BuildHasher, DefaultHasher};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Syntax-check script: `ast.parse` without writing `__pycache__` into the user's project.
const SYNTAX_CHECK_SCRIPT: &str =
    "import ast, sys\nfor f in sys.argv[1:]:\n    ast.parse(open(f, encoding='utf-8').read(), f)\n";

/// A file's fingerprint: `(byte length, content hash)`.
pub type Fingerprint = (u64, u64);

fn fingerprint(bytes: &[u8]) -> Fingerprint {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    (bytes.len() as u64, hasher.finish())
}

/// Snapshot every file under `root` as path → fingerprint.
///
/// Content hashes (not mtimes) so change detection works even when the agent
/// finishes within the filesystem's mtime granularity.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) when the tree cannot be walked.
pub fn snapshot(root: &Path) -> io::Result<HashMap<PathBuf, Fingerprint>> {
    let mut out = HashMap::new();
    walk(root, &mut |path| {
        if let Ok(bytes) = fs::read(path) {
            out.insert(path.to_path_buf(), fingerprint(&bytes));
        }
    })?;
    Ok(out)
}

/// Files the agent created or modified: absent from `before`, or with a
/// different fingerprint.
#[must_use]
pub fn changed_since<S: BuildHasher>(
    before: &HashMap<PathBuf, Fingerprint, S>,
    after: &HashMap<PathBuf, Fingerprint, S>,
) -> Vec<PathBuf> {
    let mut changed: Vec<PathBuf> = after
        .iter()
        .filter(|(path, print)| before.get(*path).is_none_or(|old| old != *print))
        .map(|(path, _)| path.clone())
        .collect();
    changed.sort();
    changed
}

fn walk(root: &Path, visit: &mut impl FnMut(&Path)) -> io::Result<()> {
    let Ok(entries) = fs::read_dir(root) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|n| n == "__pycache__" || n == "target")
            {
                continue;
            }
            walk(&path, visit)?;
        } else {
            visit(&path);
        }
    }
    Ok(())
}

fn python_interpreter() -> Option<&'static str> {
    ["python3", "python"].into_iter().find(|candidate| {
        Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    })
}

/// Syntax-check every `.py` file under `root`. Returns (path, compiler output)
/// pairs for files that failed to compile. Best-effort: if no Python is found,
/// returns an empty list (nothing to verify with).
#[must_use]
pub fn compile_check(root: &Path) -> Vec<(PathBuf, String)> {
    let Some(python) = python_interpreter() else {
        return Vec::new();
    };
    let mut files = Vec::new();
    let _ = walk(root, &mut |path| {
        if path.extension().is_some_and(|ext| ext == "py") {
            files.push(path.to_path_buf());
        }
    });
    if files.is_empty() {
        return Vec::new();
    }
    let batch = Command::new(python)
        .arg("-c")
        .arg(SYNTAX_CHECK_SCRIPT)
        .args(&files)
        .output();
    match batch {
        Ok(output) if output.status.success() => Vec::new(),
        // Re-run per file to attribute the errors.
        _ => files
            .iter()
            .filter_map(|file| {
                let output = Command::new(python)
                    .arg("-c")
                    .arg(SYNTAX_CHECK_SCRIPT)
                    .arg(file)
                    .output()
                    .ok()?;
                (!output.status.success()).then(|| {
                    (
                        file.clone(),
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    )
                })
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn py_file(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
    }

    #[test]
    fn compile_check_passes_valid_python() {
        let dir = tempfile::tempdir().unwrap();
        py_file(dir.path(), "ok.py", "def f() -> int:\n    return 1\n");
        assert!(compile_check(dir.path()).is_empty());
    }

    #[test]
    fn compile_check_reports_broken_syntax() {
        let dir = tempfile::tempdir().unwrap();
        py_file(dir.path(), "bad.py", "def f(:\n");
        py_file(dir.path(), "ok.py", "x = 1\n");
        let failures = compile_check(dir.path());
        assert_eq!(failures.len(), 1);
        assert!(failures[0].0.ends_with("bad.py"));
        assert!(!failures[0].1.is_empty());
    }

    #[test]
    fn compile_check_leaves_no_pycache() {
        let dir = tempfile::tempdir().unwrap();
        py_file(dir.path(), "ok.py", "x = 1\n");
        let _ = compile_check(dir.path());
        assert!(!dir.path().join("__pycache__").exists());
    }

    #[test]
    fn changed_since_detects_new_and_modified() {
        let a = PathBuf::from("/p/a.py");
        let b = PathBuf::from("/p/b.py");
        let c = PathBuf::from("/p/c.py");
        let before: HashMap<PathBuf, Fingerprint> = [(a.clone(), (10, 1)), (b.clone(), (10, 2))]
            .into_iter()
            .collect();
        let after: HashMap<PathBuf, Fingerprint> = [
            (a.clone(), (11, 3)),
            (b.clone(), (10, 2)),
            (c.clone(), (5, 4)),
        ]
        .into_iter()
        .collect();
        let changed = changed_since(&before, &after);
        assert!(changed.contains(&a), "modified file");
        assert!(changed.contains(&c), "new file");
        assert!(!changed.contains(&b), "untouched file");
    }

    #[test]
    fn snapshot_detects_same_second_writes() {
        // mtime-based detection failed here; fingerprints must not.
        let dir = tempfile::tempdir().unwrap();
        py_file(dir.path(), "m.py", "x = 1\n");
        let before = snapshot(dir.path()).unwrap();
        py_file(dir.path(), "m.py", "x = 2\n"); // same filesystem tick
        let after = snapshot(dir.path()).unwrap();
        assert_eq!(changed_since(&before, &after).len(), 1);
    }
}
