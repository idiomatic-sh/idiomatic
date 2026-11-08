//! Positions of the admitted inputs a semantic build reads.

/// The zero-based position of one admitted input of a build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct InputPosition(pub usize);
