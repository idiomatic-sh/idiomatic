//! The one run budget every making is held to.
use super::types::{RowsPerRow, RunBudget};
use ryo_values::domain::rows::types::RowCount;

/// The run budget: a making may write 256 rows for each row it was fed, and 65,536 rows whatever it was fed; past both it
/// stops the run. 256 sits above the healthiest fan-out the django node counters show (151) and far below a product of a
/// file's rows; the floor lets a small file's small product pass. The per-file django distribution fixes it (D3).
pub(crate) const RUN_BUDGET: RunBudget = RunBudget {
    per_fed_row: RowsPerRow(256),
    floor: RowCount(65_536),
};
