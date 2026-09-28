//! Project layouts (nunu-style): pick a layout, `git clone` it, customise.
//!
//! Layouts live in their own repositories so they evolve independently of the
//! CLI, can be forked, mirrored (`-r`), or browsed directly on GitHub. The
//! `local` layout is fastgen's built-in scaffold — instant, offline, the
//! fallback when there is no terminal or no network.

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context};

use crate::layout::write_config;
use crate::naming::to_snake;
use crate::writers::GeneratedFile;

pub struct Layout {
    pub name: &'static str,
    pub repo: &'static str,
    pub description: &'static str,
    /// Shown first and offered as the interactive default.
    pub recommended: bool,
}

/// Layouts hosted in their own repositories.
pub const REMOTE_LAYOUTS: &[Layout] = &[
    Layout {
        name: "advanced",
        repo: "https://github.com/YIbaikaishui/fastgen-layout-advanced",
        description: "worked CRUD example (note module) — recommended",
        recommended: true,
    },
    Layout {
        name: "auth",
        repo: "https://github.com/YIbaikaishui/fastgen-layout-auth",
        description: "JWT auth: register, login, protected routes, password hashing",
        recommended: false,
    },
    Layout {
        name: "basic",
        repo: "https://github.com/YIbaikaishui/fastgen-layout-basic",
        description: "minimal project, empty registry",
        recommended: false,
    },
];

/// The built-in, embedded layout: no network, no git, instant.
pub const LOCAL_LAYOUT: &str = "local";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutSource {
    Local,
    Clone(String),
}

/// Resolve a `--layout` value: an alias, `local`, or any git URL/path.
#[must_use]
pub fn resolve_layout(spec: &str) -> LayoutSource {
    if spec == LOCAL_LAYOUT {
        return LayoutSource::Local;
    }
    if let Some(layout) = REMOTE_LAYOUTS.iter().find(|l| l.name == spec) {
        return LayoutSource::Clone(layout.repo.to_string());
    }
    LayoutSource::Clone(spec.to_string())
}

/// Interactive layout picker (only called when stdin is a terminal).
///
/// # Errors
///
/// Returns an error when the prompt is cancelled.
pub fn prompt_layout() -> anyhow::Result<LayoutSource> {
    let mut items: Vec<String> = REMOTE_LAYOUTS
        .iter()
        .map(|l| format!("{} — {}", l.name, l.description))
        .collect();
    items.push(format!(
        "{LOCAL_LAYOUT} — built-in scaffold (instant, offline)"
    ));

    let default = REMOTE_LAYOUTS
        .iter()
        .position(|l| l.recommended)
        .unwrap_or(REMOTE_LAYOUTS.len());
    let selection = dialoguer::Select::new()
        .with_prompt("Pick a project layout")
        .items(&items)
        .default(default)
        .interact()?;

    if selection < REMOTE_LAYOUTS.len() {
        Ok(LayoutSource::Clone(
            REMOTE_LAYOUTS[selection].repo.to_string(),
        ))
    } else {
        Ok(LayoutSource::Local)
    }
}

/// Clone a layout repository into `target` and customise it for `name`.
///
/// # Errors
///
/// Returns an error when `git` is missing, the clone fails, or the layout is
/// not a valid src-layout project.
pub fn clone_layout(repo: &str, target: &Path, name: &str) -> anyhow::Result<Vec<GeneratedFile>> {
    if target.exists() && fs::read_dir(target)?.next().is_some() {
        bail!(
            "Directory {} already exists and is not empty.",
            target.display()
        );
    }
    let status = Command::new("git")
        .args(["clone", "--depth", "1"])
        .arg(repo)
        .arg(target)
        .status()
        .context("failed to run git — install git to use layout repositories")?;
    if !status.success() {
        bail!("git clone of {repo} failed");
    }
    // A layout is a starting point, not a continuation of its history.
    let _ = fs::remove_dir_all(target.join(".git"));

    customize(target, name)
}

/// Post-clone customisation: project name + layout marker.
fn customize(target: &Path, name: &str) -> anyhow::Result<Vec<GeneratedFile>> {
    let mut files = Vec::new();

    let pyproject = target.join("pyproject.toml");
    if pyproject.exists() {
        let text = fs::read_to_string(&pyproject)?;
        let renamed: String = text
            .lines()
            .map(|line| {
                if line.starts_with("name = \"") {
                    format!("name = \"{}\"", to_snake(name))
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&pyproject, format!("{renamed}\n"))?;
        files.push(GeneratedFile {
            path: pyproject,
            content: renamed,
            status: crate::writers::Status::Created,
        });
    }

    files.push(write_config(
        target,
        &serde_json::json!({ "source_dir": "src" }),
        false,
    )?);
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_known_aliases() {
        assert_eq!(resolve_layout("local"), LayoutSource::Local);
        for layout in REMOTE_LAYOUTS {
            assert_eq!(
                resolve_layout(layout.name),
                LayoutSource::Clone(layout.repo.to_string())
            );
        }
        assert_eq!(
            resolve_layout("auth"),
            LayoutSource::Clone("https://github.com/YIbaikaishui/fastgen-layout-auth".into())
        );
    }

    #[test]
    fn exactly_one_recommended_layout() {
        assert_eq!(REMOTE_LAYOUTS.iter().filter(|l| l.recommended).count(), 1);
        assert!(
            REMOTE_LAYOUTS[0].recommended,
            "the first entry is the default"
        );
    }

    #[test]
    fn resolve_arbitrary_url_or_path() {
        assert_eq!(
            resolve_layout("https://example.com/layout.git"),
            LayoutSource::Clone("https://example.com/layout.git".into())
        );
        assert_eq!(
            resolve_layout("/tmp/my-layout"),
            LayoutSource::Clone("/tmp/my-layout".into())
        );
    }

    #[test]
    fn clone_from_local_git_repo_and_customize() {
        // Build a fake layout repo locally (git clone works with plain paths).
        let src = tempfile::tempdir().unwrap();
        std::fs::write(
            src.path().join("pyproject.toml"),
            "[project]\nname = \"starter\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(src.path().join("src").join("modules")).unwrap();
        std::fs::write(src.path().join("src").join("main.py"), "app = None\n").unwrap();
        for args in [
            vec!["init", "-q", "-b", "main"],
            vec!["add", "-A"],
            vec![
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-q",
                "-m",
                "layout",
            ],
        ] {
            let status = Command::new("git")
                .args(&args)
                .current_dir(src.path())
                .status()
                .unwrap();
            assert!(status.success());
        }

        let dest = tempfile::tempdir().unwrap();
        let target = dest.path().join("my-app");
        let files = clone_layout(src.path().to_str().unwrap(), &target, "My App").unwrap();

        // Files landed and the layout's git history did not.
        assert!(target.join("src").join("main.py").exists());
        assert!(!target.join(".git").exists());
        // Customised for the user's project.
        let pyproject = std::fs::read_to_string(target.join("pyproject.toml")).unwrap();
        assert!(pyproject.contains("name = \"my_app\""));
        assert!(target.join(".fastgen.json").exists());
        // Report mentions the renamed pyproject.
        assert!(files.iter().any(|f| f.path.ends_with("pyproject.toml")));
    }

    #[test]
    fn clone_refuses_non_empty_target() {
        let src = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("x.txt"), "x").unwrap();
        for args in [vec!["init", "-q", "-b", "main"], vec!["add", "-A"]] {
            Command::new("git")
                .args(&args)
                .current_dir(src.path())
                .status()
                .unwrap();
        }
        Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-q",
                "-m",
                "x",
            ])
            .current_dir(src.path())
            .status()
            .unwrap();

        let dest = tempfile::tempdir().unwrap();
        std::fs::write(dest.path().join("keep.txt"), "x").unwrap();
        let err = clone_layout(src.path().to_str().unwrap(), dest.path(), "demo").unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }
}
