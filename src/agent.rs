//! AI agent integration: shell out to `codex` (preferred) or `opencode` to fill
//! in fastgen's deterministic scaffolds with real code.
//!
//! fastgen owns the *structure*; the agent owns the *code*. Generation commands
//! (`new`, `make module`) always invoke an agent — there is deliberately no
//! degraded "no agent" mode.

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Codex,
    Opencode,
    /// An explicit command (tests, or any other CLI agent).
    Custom,
}

impl AgentKind {
    /// Program name as installed on PATH.
    fn program(self) -> Option<&'static str> {
        match self {
            AgentKind::Codex => Some("codex"),
            AgentKind::Opencode => Some("opencode"),
            AgentKind::Custom => None,
        }
    }

    /// Headless subcommand (no interactive TUI).
    fn subcommand(self) -> &'static [&'static str] {
        match self {
            // --full-auto: workspace-write sandbox, no approval prompts.
            AgentKind::Codex => &["exec", "--full-auto"],
            AgentKind::Opencode => &["run"],
            AgentKind::Custom => &[],
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            AgentKind::Codex => "codex",
            AgentKind::Opencode => "opencode",
            AgentKind::Custom => "custom agent",
        }
    }

    #[must_use]
    pub fn install_hint(self) -> &'static str {
        match self {
            AgentKind::Codex => "npm install -g @openai/codex   (then: codex login)",
            AgentKind::Opencode => "curl -fsSL https://opencode.ai/install | bash",
            AgentKind::Custom => "set the agent command explicitly",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "codex" => Some(Self::Codex),
            "opencode" => Some(Self::Opencode),
            _ => None,
        }
    }

    /// All known kinds, in auto-detection preference order.
    pub const PREFERENCE: [AgentKind; 2] = [AgentKind::Codex, AgentKind::Opencode];
}

/// How the agent is chosen for a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSelection {
    /// Auto-detect: codex first, opencode second.
    Auto,
    /// A specific well-known agent (`--agent codex|opencode`).
    Kind(AgentKind),
    /// Do not run an agent (used by tests and by callers that only want the scaffold).
    Skip,
    /// An explicit command template, e.g. `/path/to/stub --flag` (tests).
    Command(String),
}

#[derive(Debug)]
pub struct Agent {
    pub kind: AgentKind,
    pub program: PathBuf,
    /// Subcommand args (e.g. `["exec", "--full-auto"]` for codex).
    pub args: Vec<String>,
}

/// Find an executable named `name` inside a PATH-style list.
///
/// Pure (takes `paths` explicitly) so tests can inject fake PATH values.
#[must_use]
pub fn find_in_path(name: &str, paths: &OsStr) -> Option<PathBuf> {
    for dir in env::split_paths(paths) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        for candidate in [
            dir.join(name),
            dir.join(format!("{name}.exe")),
            dir.join(format!("{name}.cmd")),
        ] {
            if candidate.is_file() && is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// Resolve a well-known agent kind from the CLI `--agent` value.
///
/// # Errors
///
/// Returns an error when `name` is not a known agent.
pub fn kind_from_name(name: &str) -> anyhow::Result<AgentKind> {
    AgentKind::from_name(name)
        .ok_or_else(|| anyhow!("unknown agent '{name}' (expected: codex, opencode)"))
}

fn find_kind(kind: AgentKind, paths: &OsStr) -> anyhow::Result<Agent> {
    find_in_path(kind.program().unwrap_or_default(), paths)
        .map(|program| Agent {
            kind,
            program,
            args: kind.subcommand().iter().map(|s| (*s).to_string()).collect(),
        })
        .ok_or_else(|| {
            anyhow!(
                "agent '{}' not found on PATH — install with: {}",
                kind.label(),
                kind.install_hint()
            )
        })
}

/// Resolve the selection strategy into a concrete agent (or `None` when skipped).
///
/// # Errors
///
/// Returns an error when a requested agent is not found on PATH, or when no
/// agent at all is installed.
pub fn resolve(selection: &AgentSelection) -> anyhow::Result<Option<Agent>> {
    match selection {
        AgentSelection::Skip => Ok(None),
        AgentSelection::Auto => {
            let paths = env::var_os("PATH").unwrap_or_default();
            for kind in AgentKind::PREFERENCE {
                if let Some(program) = find_in_path(kind.program().unwrap_or_default(), &paths) {
                    return Ok(Some(Agent {
                        kind,
                        program,
                        args: kind.subcommand().iter().map(|s| (*s).to_string()).collect(),
                    }));
                }
            }
            anyhow::bail!(
                "no AI agent found on PATH — install one of:\n  codex:    {}\n  opencode: {}",
                AgentKind::Codex.install_hint(),
                AgentKind::Opencode.install_hint(),
            )
        }
        AgentSelection::Kind(kind) => {
            let paths = env::var_os("PATH").unwrap_or_default();
            find_kind(*kind, &paths).map(Some)
        }
        AgentSelection::Command(command) => {
            let mut parts = command.split_whitespace();
            let program = parts.next().ok_or_else(|| anyhow!("empty agent command"))?;
            Ok(Some(Agent {
                kind: AgentKind::Custom,
                program: PathBuf::from(program),
                args: parts.map(String::from).collect(),
            }))
        }
    }
}

/// Resolve the agent to use: explicit `--agent`, else auto-detect (codex first).
///
/// # Errors
///
/// Returns an error when the requested agent is unknown, not found on PATH, or
/// when no agent at all is installed.
pub fn detect(explicit: Option<&str>) -> anyhow::Result<Agent> {
    let selection = match explicit {
        Some(name) => AgentSelection::Kind(kind_from_name(name)?),
        None => AgentSelection::Auto,
    };
    resolve(&selection)?.ok_or_else(|| anyhow!("agent unexpectedly skipped"))
}

#[cfg(test)]
fn detect_with(explicit: Option<&str>, paths: &OsStr) -> anyhow::Result<Agent> {
    if let Some(name) = explicit {
        let kind = AgentKind::from_name(name)
            .ok_or_else(|| anyhow!("unknown agent '{name}' (expected: codex, opencode)"))?;
        return find_kind(kind, paths);
    }
    for kind in AgentKind::PREFERENCE {
        if let Some(program) = find_in_path(kind.program().unwrap_or_default(), paths) {
            return Ok(Agent {
                kind,
                program,
                args: kind.subcommand().iter().map(|s| (*s).to_string()).collect(),
            });
        }
    }
    anyhow::bail!(
        "no AI agent found on PATH — install one of:\n  codex:    {}\n  opencode: {}",
        AgentKind::Codex.install_hint(),
        AgentKind::Opencode.install_hint(),
    )
}

impl Agent {
    /// Run the agent headless in `cwd` with `prompt`, streaming its output.
    ///
    /// # Errors
    ///
    /// Returns an error when the agent cannot be spawned or exits non-zero.
    pub fn run(&self, prompt: &str, cwd: &Path) -> anyhow::Result<()> {
        // npm installs .cmd shims on Windows; those must go through cmd /C.
        let needs_shell = self
            .program
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"));
        let mut command = if needs_shell {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(&self.program);
            c
        } else {
            Command::new(&self.program)
        };
        command.args(&self.args);
        command.arg(prompt);
        command.current_dir(cwd);
        let status = command.status().with_context(|| {
            format!(
                "failed to run {} at {}",
                self.kind.label(),
                self.program.display()
            )
        })?;
        if !status.success() {
            return Err(anyhow!("{} exited with {status}", self.kind.label()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn fake_path(files: &[(&str, bool)]) -> (TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        for (name, executable) in files {
            let path = dir.path().join(name);
            fs::write(&path, "#!/bin/sh\n").unwrap();
            #[cfg(unix)]
            if *executable {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        let joined = dir.path().to_str().unwrap().to_string();
        (dir, joined)
    }

    #[test]
    fn find_in_path_finds_executable() {
        let (_dir, joined) = fake_path(&[("codex", true)]);
        let paths = OsStr::new(&joined);
        assert!(find_in_path("codex", paths).is_some());
        assert!(find_in_path("opencode", paths).is_none());
    }

    #[test]
    fn find_in_path_ignores_non_executable() {
        let (_dir, joined) = fake_path(&[("codex", false)]);
        assert!(find_in_path("codex", OsStr::new(&joined)).is_none());
    }

    #[test]
    fn detect_prefers_codex() {
        let (_dir, joined) = fake_path(&[("codex", true), ("opencode", true)]);
        let agent = detect_with(None, OsStr::new(&joined)).unwrap();
        assert_eq!(agent.kind, AgentKind::Codex);
    }

    #[test]
    fn detect_falls_back_to_opencode() {
        let (_dir, joined) = fake_path(&[("opencode", true)]);
        let agent = detect_with(None, OsStr::new(&joined)).unwrap();
        assert_eq!(agent.kind, AgentKind::Opencode);
    }

    #[test]
    fn detect_explicit_agent_must_exist() {
        let (_dir, joined) = fake_path(&[("opencode", true)]);
        let err = detect_with(Some("codex"), OsStr::new(&joined)).unwrap_err();
        assert!(err.to_string().contains("codex"));
        assert!(err.to_string().contains("npm install"));
    }

    #[test]
    fn detect_unknown_agent_name_errors() {
        let (_dir, joined) = fake_path(&[("codex", true)]);
        let err = detect_with(Some("cursor"), OsStr::new(&joined)).unwrap_err();
        assert!(err.to_string().contains("unknown agent"));
    }

    #[test]
    fn resolve_skip_returns_none() {
        assert!(resolve(&AgentSelection::Skip).unwrap().is_none());
    }

    #[test]
    fn resolve_command_builds_custom_agent() {
        let agent = resolve(&AgentSelection::Command(
            "/tmp/stub-agent --full-auto".to_string(),
        ))
        .unwrap()
        .unwrap();
        assert_eq!(agent.kind, AgentKind::Custom);
        assert_eq!(agent.program, PathBuf::from("/tmp/stub-agent"));
        assert_eq!(agent.args, vec!["--full-auto".to_string()]);
    }

    #[test]
    fn resolve_auto_picks_first_available() {
        let (_dir, joined) = fake_path(&[("opencode", true)]);
        // resolve() reads env PATH; simulate via detect_with with the same logic.
        let agent = detect_with(None, OsStr::new(&joined)).unwrap();
        assert_eq!(agent.kind, AgentKind::Opencode);
        assert_eq!(agent.args, vec!["run".to_string()]);
    }

    #[test]
    fn detect_without_any_agent_lists_both_installs() {
        let (_dir, joined) = fake_path(&[]);
        let err = detect_with(None, OsStr::new(&joined)).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("@openai/codex"));
        assert!(msg.contains("opencode.ai"));
    }
}
