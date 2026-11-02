//! The application's `pipeline` concept: the per-file steps of a plan run as one pass over the rows. A step does not make a table
//! of its rows for the step that reads it: it passes each row on as it makes it, and only what a step must see whole is kept
//! whole (the side of a join that is looked up, what a group counts) or what the plan emits or hands over, or what more than one
//! step reads and is not cheap to make again.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
pub(crate) mod service_test;
