//! fastgen CLI entrypoint.

use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use comfy_table::{presets::UTF8_FULL, Table};
use owo_colors::OwoColorize;

use crate::generators::{
    alembic, core, main as main_sync, module as module_gen, project as project_gen,
    registry as registry_gen,
};
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
        /// Preview files without writing.
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
        /// Preview files without writing.
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
pub fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => Ok(()),
        Some(command) => execute(command),
    }
}

/// Testable entrypoint: parse an explicit argument vector and run it.
///
/// # Errors
///
/// Returns an error when argument parsing fails or a command fails.
pub fn run_with<I, T>(args: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::try_parse_from(args)?;
    match cli.command {
        None => Ok(()),
        Some(command) => execute(command),
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

fn execute(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::New {
            name,
            dir,
            title,
            description,
            dry_run,
            force,
        } => {
            let dir = resolve_dir(&dir);
            let target = if name == "." {
                dir.clone()
            } else {
                dir.join(&name)
            };
            if target.exists() && fs::read_dir(&target)?.next().is_some() && !force {
                anyhow::bail!(
                    "Directory {} already exists and is not empty. Use --force to overwrite.",
                    target.display()
                );
            }
            let files = project_gen::generate_project(
                &name,
                &target,
                title.as_deref(),
                &description,
                "0.1.0",
                force,
                dry_run,
            )?;
            report(&files, dry_run);
            if !dry_run {
                anstream::println!(
                    "{}",
                    format!("Project created at {}", target.display()).green()
                );
                anstream::println!(
                    "{}",
                    format!(
                        "cd {} && uv sync && uv run uvicorn src.main:app --reload",
                        target.display()
                    )
                    .cyan()
                );
            }
        }
        Commands::Make {
            command:
                MakeCommands::Module {
                    feature,
                    dir,
                    dry_run,
                    force,
                },
        } => {
            let dir = resolve_dir(&dir);
            let mut files = module_gen::generate_module(&feature, &dir, None, force, dry_run)?;
            files.extend(core::generate_core(&dir, None, dry_run)?);
            files.push(registry_gen::register_module(
                &dir, &feature, None, dry_run,
            )?);
            files.push(main_sync::sync_main(&dir, None, dry_run)?);
            report(&files, dry_run);
            let skipped = files.iter().filter(|f| f.status == Status::Skipped).count();
            if skipped > 0 && !force {
                anstream::println!(
                    "{}",
                    format!("{skipped} file(s) already exist. Re-run with --force to overwrite.")
                        .yellow()
                );
            }
        }
        Commands::Init {
            command: InitCommands::Alembic { dir, dry_run },
        } => {
            let dir = resolve_dir(&dir);
            let files = alembic::generate_alembic(&dir, None, dry_run)?;
            report(&files, dry_run);
            anstream::println!("{}", NEXT_STEPS.cyan());
        }
        Commands::List { dir } => {
            let dir = resolve_dir(&dir);
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
    }
    Ok(())
}
