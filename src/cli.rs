//! fastgen CLI entrypoint.

use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use comfy_table::{presets::UTF8_FULL, Table};
use owo_colors::OwoColorize;

use crate::agent::{self, resolve as resolve_agent, Agent, AgentSelection};
use crate::generators::{
    alembic, core, main as main_sync, module as module_gen, project as project_gen,
    registry as registry_gen,
};
use crate::layout::source_dir_name;
use crate::layouts::{self, LayoutSource};
use crate::naming::to_snake;
use crate::prompt;
use crate::reconcile;
use crate::validate;
use crate::writers::{report, Status};

const NEXT_STEPS: &str = "Next steps:\n\
\x20 1. Add the dependency:  uv add alembic\n\
\x20 2. Apply the baseline:   uv run alembic upgrade head\n\
\x20 3. After model changes:  uv run alembic revision --autogenerate -m '<message>' && uv run alembic upgrade head\n\
\x20 If the DB was already created without Alembic (create_all), adopt it with `uv run alembic stamp head`,\n\
\x20 or delete the dev DB and re-create it via steps 2-3.";

#[derive(Parser)]
#[command(
    name = "fastgen",
    version,
    about = "FastAPI feature-based module manager (nest-cli style).",
    arg_required_else_help = true
)]
struct Cli {
    /// Force a specific agent: `codex` or `opencode` (default: auto-detect).
    #[arg(long, global = true)]
    agent: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a new best-practice `FastAPI` project.
    ///
    /// Generates a `src`-layout project: `.env`, `src/main.py`, `src/core/`
    /// (config + async database), the `src/modules/` registry, a `tests/` suite,
    /// plus `pyproject.toml` / `.gitignore` / `.python-version`. The project is
    /// immediately manageable with `fastgen make module` / `fastgen list`.
    New {
        /// Project name, or '.' to scaffold into --dir.
        name: String,
        /// Parent directory for the new project.
        #[arg(long, short = 'd', default_value = ".")]
        dir: PathBuf,
        /// Human-readable app title.
        #[arg(long)]
        title: Option<String>,
        /// Short project description.
        #[arg(long, default_value = "")]
        description: String,
        /// Optional: hand a natural-language spec to an AI agent (codex/opencode)
        /// that fills the scaffold with real code. Omit for the bare template.
        #[arg(long)]
        ai: Option<String>,
        /// Project layout: `advanced`, `basic`, `local`, or a git URL/path.
        /// Prompts interactively when omitted in a terminal.
        #[arg(long)]
        layout: Option<String>,
        /// Shorthand for `--layout <url>`: clone any layout repository.
        #[arg(long, short = 'r')]
        repo: Option<String>,
        /// Preview files without writing (skips the agent).
        #[arg(long)]
        dry_run: bool,
        /// Overwrite an existing project.
        #[arg(long, short = 'f')]
        force: bool,
    },
    /// Scaffold modules and core infrastructure.
    #[command(subcommand_required = true, arg_required_else_help = true)]
    Make {
        #[command(subcommand)]
        command: MakeCommands,
    },
    /// Add optional infrastructure to an existing project.
    #[command(subcommand_required = true, arg_required_else_help = true)]
    Init {
        #[command(subcommand)]
        command: InitCommands,
    },
    /// List registered modules and their boundaries.
    List {
        /// Target project root.
        #[arg(long, short = 'd', default_value = ".")]
        dir: PathBuf,
    },
    /// Check a project for structure drift (stale registry entries, unmounted
    /// modules, broken auto-mount, syntax errors). Exit code 1 on errors —
    /// CI-gateable.
    Doctor {
        /// Target project root.
        #[arg(long, short = 'd', default_value = ".")]
        dir: PathBuf,
        /// Repair what is fixable: register orphans, drop stale entries,
        /// re-inject the auto-mount block.
        #[arg(long)]
        fix: bool,
        /// Also exit 1 on warnings.
        #[arg(long)]
        strict: bool,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum MakeCommands {
    /// Scaffold a feature module and register it.
    ///
    /// Generates a minimal module skeleton (schemas / service / router /
    /// __init__) that outlines the module's shape; fill in the entity fields and
    /// business logic yourself. The shared core/ database scaffolding and the
    /// module registry are created automatically.
    Module {
        /// Feature name, e.g. user
        feature: String,
        /// Target project root.
        #[arg(long, short = 'd', default_value = ".")]
        dir: PathBuf,
        /// Optional: hand a natural-language spec to an AI agent (codex/opencode)
        /// that fills the module with real code. Omit for the skeleton only.
        #[arg(long)]
        ai: Option<String>,
        /// Preview files without writing (skips the agent).
        #[arg(long)]
        dry_run: bool,
        /// Overwrite existing files.
        #[arg(long, short = 'f')]
        force: bool,
    },
}

#[derive(Subcommand)]
enum InitCommands {
    /// Add Alembic migration scaffolding to an existing project.
    ///
    /// Writes `alembic.ini` and `migrations/` (async env + an empty baseline
    /// revision), wiring `DATABASE_URL` from the project's settings and
    /// `Base.metadata` from the module registry. Existing files are never
    /// overwritten.
    Alembic {
        /// Target project root.
        #[arg(long, short = 'd', default_value = ".")]
        dir: PathBuf,
        /// Preview files without writing.
        #[arg(long)]
        dry_run: bool,
    },
}

/// Run the CLI from the process arguments.
///
/// # Errors
///
/// Returns an error when argument parsing fails or a command fails.
/// Run the CLI from the process arguments (`FASTGEN_AGENT_CMD` overrides agent
/// detection; `--agent` wins over it).
///
/// # Errors
///
/// Returns an error when argument parsing fails or a command fails.
pub fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let selection = agent_selection(cli.agent.as_deref())?;
    match cli.command {
        None => Ok(()),
        Some(command) => execute(command, &selection),
    }
}

/// Testable entrypoint: parse an explicit argument vector and run it.
///
/// # Errors
///
/// Returns an error when argument parsing fails or a command fails.
/// # Errors
///
/// Returns an error when argument parsing fails or a command fails.
pub fn run_with<I, T>(args: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::try_parse_from(args)?;
    let selection = agent_selection(cli.agent.as_deref())?;
    match cli.command {
        None => Ok(()),
        Some(command) => execute(command, &selection),
    }
}

/// Test entrypoint: run with an explicit agent selection (`Skip` for scaffold-only tests).
///
/// # Errors
///
/// Returns an error when argument parsing fails or a command fails.
#[allow(clippy::needless_pass_by_value)]
pub fn run_with_mode<I, T>(args: I, selection: AgentSelection) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::try_parse_from(args)?;
    match cli.command {
        None => Ok(()),
        Some(command) => execute(command, &selection),
    }
}

/// Resolve the default `.` to the current directory (matching Python's `Path.cwd()`).
fn resolve_dir(dir: &Path) -> PathBuf {
    if dir == Path::new(".") {
        std::env::current_dir().unwrap_or_else(|_| dir.to_path_buf())
    } else {
        dir.to_path_buf()
    }
}

/// Build the agent selection: `--agent` flag > `FASTGEN_AGENT_CMD` > auto-detect.
fn agent_selection(flag: Option<&str>) -> anyhow::Result<AgentSelection> {
    if let Some(name) = flag {
        return Ok(AgentSelection::Kind(agent::kind_from_name(name)?));
    }
    if let Some(command) = std::env::var("FASTGEN_AGENT_CMD")
        .ok()
        .filter(|c| !c.trim().is_empty())
    {
        return Ok(AgentSelection::Command(command));
    }
    Ok(AgentSelection::Auto)
}

fn execute(command: Commands, selection: &AgentSelection) -> anyhow::Result<()> {
    match command {
        Commands::New {
            name,
            dir,
            title,
            description,
            ai,
            layout,
            repo,
            dry_run,
            force,
        } => cmd_new(
            &name,
            &dir,
            title.as_deref(),
            &description,
            ai.as_deref(),
            layout.as_deref(),
            repo.as_deref(),
            selection,
            dry_run,
            force,
        ),
        Commands::Make { ref command } => cmd_make(command, selection),
        Commands::Doctor {
            ref dir,
            fix,
            strict,
            json,
        } => cmd_doctor(dir, fix, strict, json),
        Commands::Init { ref command } => cmd_init(command),
        Commands::List { ref dir } => {
            cmd_list(dir);
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_new(
    name: &str,
    dir: &Path,
    title: Option<&str>,
    description: &str,
    ai: Option<&str>,
    layout: Option<&str>,
    repo: Option<&str>,
    selection: &AgentSelection,
    dry_run: bool,
    force: bool,
) -> anyhow::Result<()> {
    let dir = resolve_dir(dir);
    let target = if name == "." {
        dir.clone()
    } else {
        dir.join(name)
    };
    // Layout: -r wins, then --layout, then an interactive picker (TTY only),
    // then the built-in local scaffold (keeps CI and scripts offline).
    let source = if let Some(repo) = repo {
        LayoutSource::Clone(repo.to_string())
    } else if let Some(spec) = layout {
        layouts::resolve_layout(spec)
    } else if !dry_run && std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        layouts::prompt_layout()?
    } else {
        LayoutSource::Local
    };

    let files = match source {
        LayoutSource::Local => {
            if target.exists() && fs::read_dir(&target)?.next().is_some() && !force {
                anyhow::bail!(
                    "Directory {} already exists and is not empty. Use --force to overwrite.",
                    target.display()
                );
            }
            project_gen::generate_project(
                name,
                &target,
                title,
                description,
                "0.1.0",
                force,
                dry_run,
            )?
        }
        LayoutSource::Clone(repo) => {
            anstream::println!(
                "{}",
                format!("Cloning layout from {repo} (needs git + network)...").bold()
            );
            layouts::clone_layout(&repo, &target, name)?
        }
    };
    report(&files, dry_run);
    anstream::println!(
        "{}",
        format!("Project created at {}", target.display()).green()
    );
    if dry_run {
        anstream::println!(
            "{}",
            format!(
                "cd {} && uv sync && uv run uvicorn src.main:app --reload",
                target.display()
            )
            .cyan()
        );
        return Ok(());
    }
    // The agent is opt-in: `fastgen new` alone is a pure deterministic scaffold.
    if let Some(spec) = ai {
        let prompt = prompt::build_project_prompt(name, "src", &target, Some(spec))?;
        run_agent_flow(selection, &prompt, &target, &target.join("src"))?;
    }
    anstream::println!(
        "{}",
        format!(
            "cd {} && uv sync && uv run uvicorn src.main:app --reload",
            target.display()
        )
        .cyan()
    );
    Ok(())
}

fn cmd_make(command: &MakeCommands, selection: &AgentSelection) -> anyhow::Result<()> {
    let MakeCommands::Module {
        feature,
        dir,
        ai,
        dry_run,
        force,
    } = command;
    let dir = resolve_dir(dir);
    let mut files = module_gen::generate_module(feature, &dir, None, *force, *dry_run)?;
    files.extend(core::generate_core(&dir, None, *dry_run)?);
    files.push(registry_gen::register_module(
        &dir, feature, None, *dry_run,
    )?);
    files.push(main_sync::sync_main(&dir, None, *dry_run)?);
    report(&files, *dry_run);
    let skipped = files.iter().filter(|f| f.status == Status::Skipped).count();
    if skipped > 0 && !force {
        anstream::println!(
            "{}",
            format!("{skipped} file(s) already exist. Re-run with --force to overwrite.").yellow()
        );
    }
    if *dry_run {
        return Ok(());
    }
    // The agent is opt-in: `fastgen make module` alone is a pure deterministic scaffold.
    let Some(spec) = ai else {
        return Ok(());
    };
    let source = source_dir_name(&dir);
    let prompt = prompt::build_module_prompt(feature, &source, &dir, Some(spec))?;
    let module_root = dir.join(&source).join("modules").join(to_snake(feature));
    run_agent_flow(selection, &prompt, &dir, &module_root)
}

fn cmd_init(command: &InitCommands) -> anyhow::Result<()> {
    let InitCommands::Alembic { dir, dry_run } = command;
    let dir = resolve_dir(dir);
    let files = alembic::generate_alembic(&dir, None, *dry_run)?;
    report(&files, *dry_run);
    anstream::println!("{}", NEXT_STEPS.cyan());
    Ok(())
}

fn cmd_doctor(dir: &Path, fix: bool, strict: bool, json: bool) -> anyhow::Result<()> {
    use crate::doctor::{check_project, fix_project, Issue};

    if fix {
        let actions = fix_project(dir, None)?;
        if json {
            anstream::println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!(actions))?
            );
        } else if actions.is_empty() {
            anstream::println!("{}", "nothing to fix".green());
        } else {
            for action in &actions {
                anstream::println!("{}", format!("[fixed] {action}").green());
            }
        }
    }

    let issues = check_project(dir, None)?;
    let errors = issues
        .iter()
        .filter(|i| i.severity == crate::doctor::Severity::Error)
        .count();
    let warnings = issues.len() - errors;

    if json {
        let payload = serde_json::json!({
            "errors": errors,
            "warnings": warnings,
            "issues": issues.iter().map(Issue::to_json).collect::<Vec<_>>(),
        });
        anstream::println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if issues.is_empty() {
        anstream::println!("{}", "\u{2713} project structure is clean".green());
    } else {
        for issue in &issues {
            let label = match issue.severity {
                crate::doctor::Severity::Error => "error".red().to_string(),
                crate::doctor::Severity::Warning => "warning".yellow().to_string(),
            };
            let hint = if issue.fixable {
                " (fixable: --fix)"
            } else {
                ""
            };
            anstream::println!("{} {} {}{}", label, issue.code, issue.message, hint);
        }
        anstream::println!(
            "{}",
            format!("{errors} error(s), {warnings} warning(s)").bold()
        );
    }

    if errors > 0 || (strict && warnings > 0) {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_list(dir: &Path) {
    let dir = resolve_dir(dir);
    let rows = registry_gen::list_registered(&dir, None);
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["module", "path", "description"]);
    for (name, import_path, doc) in &rows {
        table.add_row(vec![name.clone(), import_path.clone(), doc.clone()]);
    }
    anstream::println!("{}", "Registered modules".bold());
    anstream::println!("{table}");
    if rows.is_empty() {
        anstream::println!(
            "{}",
            "No modules registered yet. Run `fastgen make module <name>`.".yellow()
        );
    }
}

/// Render \u{2192} agent \u{2192} governance: run the agent over `project_root`, then reconcile
/// the registry, report what it touched, and syntax-check the result.
fn run_agent_flow(
    selection: &AgentSelection,
    prompt: &str,
    project_root: &Path,
    report_root: &Path,
) -> anyhow::Result<()> {
    let Some(agent): Option<Agent> = resolve_agent(selection)? else {
        return Ok(());
    };
    anstream::println!(
        "{}",
        format!(
            "Running {} to fill in the scaffold (this can take a minute)...",
            agent.kind.label()
        )
        .bold()
    );
    let before = validate::snapshot(report_root).unwrap_or_default();
    agent.run(prompt, project_root)?;
    let after = validate::snapshot(report_root).unwrap_or_default();

    // Governance: the registry and the auto-mount block agree with disk again.
    let reconciled = reconcile::reconcile_registry(project_root, None)?;
    for file in &reconciled {
        if file.status == Status::Created {
            anstream::println!(
                "{}",
                format!("[registered] {}", file.path.display()).green()
            );
        }
    }

    let changed = validate::changed_since(&before, &after);
    if !changed.is_empty() {
        anstream::println!(
            "{}",
            format!(
                "{} file(s) written by {}:",
                changed.len(),
                agent.kind.label()
            )
            .bold()
        );
        for path in &changed {
            let shown = path.strip_prefix(project_root).unwrap_or(path);
            anstream::println!("  {}", shown.display());
        }
    }

    let failures = validate::compile_check(report_root);
    if !failures.is_empty() {
        anstream::println!(
            "{}",
            format!("{} file(s) failed `python -m py_compile`:", failures.len()).yellow()
        );
        for (path, err) in &failures {
            eprintln!("--- {} ---\n{err}", path.display());
        }
    }
    Ok(())
}
