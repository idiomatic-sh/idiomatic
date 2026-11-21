//! The constants of the `ordering` pass: how far a rank looks ahead, and the bounds an order is taken by.

use super::types::{PassRate, RunWork};
use ryo_relations::domain::relations::types::Count;

/// How many steps of an unlocked run a step's rank looks through.
pub(crate) const LOOKAHEAD: Count = Count(8);

/// The least work a rank divides by.
pub(crate) const LEAST_WORK: RunWork = RunWork(1e-9);

/// The share of a level's modelled work a new order must save to be taken.
pub(crate) const LEAST_GAIN: PassRate = PassRate(0.02);

/// The rows a step that passed none on the sample is taken to pass: half a row, never none.
pub(crate) const UNPASSED_ROWS: PassRate = PassRate(0.5);

/// The fewest steps of a level worth ordering: with fewer than two that may move, there is one order.
pub(crate) const LEAST_MOVABLE: Count = Count(2);
