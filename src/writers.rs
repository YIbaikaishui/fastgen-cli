//! File writing helpers with dry-run and overwrite protection.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use owo_colors::OwoColorize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Created,
    Skipped,
    DryRun,
}

impl Status {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Created => "created",
            Status::Skipped => "skipped",
            Status::DryRun => "dry-run",
        }
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: PathBuf,
    pub content: String,
    pub status: Status,
}

/// Write `content` to `path`, honouring dry-run and never overwriting existing
/// files unless `force` is set.
///
/// # Errors
///
/// Returns an [`io::Error`] when directories or files cannot be created/written.
pub fn write_file(
    path: &Path,
    content: &str,
    force: bool,
    dry_run: bool,
) -> io::Result<GeneratedFile> {
    if dry_run {
        return Ok(GeneratedFile {
            path: path.to_path_buf(),
            content: content.to_string(),
            status: Status::DryRun,
        });
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() && !force {
        return Ok(GeneratedFile {
            path: path.to_path_buf(),
            content: content.to_string(),
            status: Status::Skipped,
        });
    }
    let content = if content.ends_with('\n') {
        content.to_string()
    } else {
        format!("{content}\n")
    };
    fs::write(path, &content)?;
    Ok(GeneratedFile {
        path: path.to_path_buf(),
        content,
        status: Status::Created,
    })
}

/// Print the generated-file report (colors are stripped automatically when
/// stdout is not a terminal, matching the previous rich-based output).
pub fn report(files: &[GeneratedFile], dry_run: bool) {
    let prefix = if dry_run {
        format!("{} ", "[dry-run]".yellow())
    } else {
        String::new()
    };
    for f in files {
        let styled = match f.status {
            Status::Created => "[created]".green().to_string(),
            Status::Skipped => "[skipped]".yellow().to_string(),
            Status::DryRun => "[dry-run]".cyan().to_string(),
        };
        anstream::println!("{prefix}{styled} {}", f.path.display());
    }
}
