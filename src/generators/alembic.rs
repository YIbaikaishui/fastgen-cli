//! Alembic migration scaffolding generator.
//!
//! Writes ``alembic.ini`` + ``migrations/`` into a project so schema changes can be
//! applied with ``uv run alembic upgrade head`` instead of deleting the dev DB.
//! Files are only written when missing (idempotent, non-destructive), matching the
//! ``core/`` behaviour.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use minijinja::{context, Value};

use crate::layout::source_dir_name;
use crate::render::render_tree;
use crate::writers::GeneratedFile;

const ALEMBIC_TEMPLATE: &str = "alembic";

/// # Errors
///
/// Returns an error when a template fails to render or a file cannot be written.
pub fn generate_alembic(
    project_root: &Path,
    source: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Vec<GeneratedFile>> {
    let source = source.map_or_else(|| source_dir_name(project_root), ToString::to_string);
    let ctx: Value = context! {
        source => source,
        create_date => now_utc_string(),
    };
    render_tree(ALEMBIC_TEMPLATE, &ctx, project_root, false, dry_run)
}

/// Current UTC time as ``YYYY-MM-DD HH:MM:SS`` (the format Alembic stamps on revisions).
fn now_utc_string() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let days = i64::try_from(secs / 86_400).unwrap_or(i64::MAX);
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year,
        month,
        day,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Days since the Unix epoch to a (year, month, day) proleptic-Gregorian date.
///
/// Howard Hinnant's `chrono`-compatible algorithm.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_from_days_matches_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_717), (2026, 9, 21));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(20_000), (2024, 10, 4));
    }

    #[test]
    fn now_utc_string_is_formatted() {
        let formatted = now_utc_string();
        assert_eq!(formatted.len(), 19);
        assert_eq!(&formatted[4..5], "-");
        assert_eq!(&formatted[10..11], " ");
    }
}
