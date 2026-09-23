//! End-to-end tests for the AI-agent flow, using a stub agent injected via
//! [`AgentSelection::Command`] — no real codex/opencode session is ever started.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use fastgen_cli::agent::AgentSelection;
use fastgen_cli::cli::run_with_mode;
use fastgen_cli::generators::{project::generate_project, registry::read_registry};

/// Write an executable stub "agent" that simulates an agent editing the project
/// (fills in the scaffolded module, creates an unregistered extra module).
fn stub_agent(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("stub-agent");
    fs::write(
        &path,
        "#!/bin/sh\n\
         # The prompt is the last argument; cwd is the project root.\n\
         echo \"# filled by stub agent\" >> src/modules/post/domain/model.py\n\
         mkdir -p src/modules/extra/api\n\
         printf '\"\"\"Extra module.\"\"\"' > src/modules/extra/__init__.py\n\
         printf 'router = None' > src/modules/extra/api/router.py\n",
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn scaffold(root: &Path) {
    generate_project("demo", root, None, "", "0.1.0", false, false).unwrap();
}

#[test]
fn stub_agent_fills_scaffold_and_reconcile_registers_new_module() {
    let tmp = tempfile::tempdir().unwrap();
    scaffold(tmp.path());
    let stub = stub_agent(tmp.path());

    run_with_mode(
        [
            "fastgen",
            "make",
            "module",
            "post",
            "--dir",
            tmp.path().to_str().unwrap(),
        ],
        AgentSelection::Command(stub.to_str().unwrap().to_string()),
    )
    .unwrap();

    // The agent's edit landed inside the scaffolded structure.
    let model = fs::read_to_string(
        tmp.path()
            .join("src")
            .join("modules")
            .join("post")
            .join("domain")
            .join("model.py"),
    )
    .unwrap();
    assert!(model.contains("# filled by stub agent"));

    // The module the agent created but did not register was reconciled.
    let registry = read_registry(tmp.path(), None);
    assert!(
        registry
            .iter()
            .any(|(name, path)| name == "extra" && path == "src.modules.extra"),
        "agent-created module should be registered: {registry:?}"
    );
    assert!(
        registry.iter().any(|(name, _)| name == "post"),
        "the scaffolded module should still be registered"
    );
}

#[test]
fn skip_selection_leaves_scaffold_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    scaffold(tmp.path());

    run_with_mode(
        [
            "fastgen",
            "make",
            "module",
            "post",
            "--dir",
            tmp.path().to_str().unwrap(),
        ],
        AgentSelection::Skip,
    )
    .unwrap();

    let model = fs::read_to_string(
        tmp.path()
            .join("src")
            .join("modules")
            .join("post")
            .join("domain")
            .join("model.py"),
    )
    .unwrap();
    assert!(!model.contains("# filled by stub agent"));
    assert!(model.contains("class Post(Base)"));
}

#[test]
fn dry_run_never_runs_the_agent() {
    let tmp = tempfile::tempdir().unwrap();
    scaffold(tmp.path());
    let stub = stub_agent(tmp.path());

    run_with_mode(
        [
            "fastgen",
            "make",
            "module",
            "post",
            "--dir",
            tmp.path().to_str().unwrap(),
            "--dry-run",
        ],
        AgentSelection::Command(stub.to_str().unwrap().to_string()),
    )
    .unwrap();

    // Nothing at all was written, agent or not.
    assert!(!tmp.path().join("src").join("modules").join("post").exists());
}

#[test]
fn new_command_runs_agent_when_not_dry_run() {
    let tmp = tempfile::tempdir().unwrap();
    let stub = stub_agent(tmp.path());

    run_with_mode(
        [
            "fastgen",
            "new",
            "demo",
            "--dir",
            tmp.path().to_str().unwrap(),
        ],
        AgentSelection::Command(stub.to_str().unwrap().to_string()),
    )
    .unwrap();

    // `fastgen new` runs the agent over the whole project; the stub creates the
    // extra module even though the project starts empty.
    let registry = read_registry(&tmp.path().join("demo"), None);
    assert!(registry.iter().any(|(name, _)| name == "extra"));
}
