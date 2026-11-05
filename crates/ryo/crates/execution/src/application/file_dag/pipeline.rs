//! The application's `pipeline` concept: the per-file steps of a plan run as one pass over the rows. A step does not make a table
//! of its rows for the step that reads it: it passes each row on as it makes it, and only what a step must see whole is kept
//! whole (the side of a join that is looked up, what a group counts) or what the plan emits or hands over, or what more than one
//! step reads and is not cheap to make again.

// Historical compact transport cases remain registered while their fixtures
// migrate to the same required graph/executor API; no old evaluator is retained.
#[cfg(test)]
#[path = "pipeline/types/compact/value_test.rs"]
mod compact_transport_test;

#[cfg(test)]
pub(crate) mod service_test;
