//! The test support of the `parse` use case: the contract every implementation of its trait runs, and the
//! conformance of a language to the whole catalog.

#[cfg(test)]
pub mod breaking;
pub mod collect;
pub mod configuration;
pub mod conformance;
pub mod contract;
pub mod expected;
pub mod expected_rows;
pub mod findings;
pub mod inputs;
pub mod items;
pub mod judging;
pub mod judgments;
pub mod manifests;
pub mod not_yet_read;
#[cfg(test)]
pub mod ordered;
#[cfg(test)]
pub mod promised;
pub mod rails;
pub mod reading;
pub mod rendered;
pub mod row_checks;
pub mod rows;
pub mod samples;
pub mod semantics;
#[cfg(test)]
pub mod small;
pub mod unread;
pub mod validators;
pub mod wanted;
