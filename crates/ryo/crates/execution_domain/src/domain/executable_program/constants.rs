//! The constants of the `executable_program` concept.

use ryo_operators::domain::physical_plan::types::ServedAt;

/// The stage that serves a step after the files, a bound input and the rows and hosted kinds.
pub const BOUND_STAGE: ServedAt = ServedAt(4);
