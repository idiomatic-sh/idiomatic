//! The types of the `ordering` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. The files whose rows wait for their turn to leave.

use std::collections::BTreeMap;

use ryo_values::domain::files::types::FileSize;
use ryo_values::domain::row_texts::types::RowTotal;

use crate::domain::render::types::Rendered;

/// A file, by its place in the order its rows leave in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Turn(pub usize);

/// How many bytes of rendered rows wait for their turn to leave.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PendingBytes(pub usize);

/// Whether a file may be read now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Admittance {
    /// It may.
    Admitted,
    /// It waits for the files being read.
    Waits,
}

/// The files whose rows wait for the files before them to leave, so the rows leave in file order.
#[derive(Clone, Debug, Default)]
pub(crate) struct Ordered {
    /// The file whose turn it is.
    pub next: Turn,
    /// The rows of the files finished ahead of their turn.
    pub pending: BTreeMap<Turn, Rendered>,
    /// How many bytes of rows `pending` holds.
    pub held: PendingBytes,
    /// How many bytes of source the files being read hold between them.
    pub inflight: FileSize,
    /// Buffers that left, kept for the next file.
    pub spares: Vec<Rendered>,
    /// How many rows may leave in all.
    pub limit: RowTotal,
    /// How many rows the files that left had.
    pub total: RowTotal,
    /// How many rows left.
    pub shown: RowTotal,
}
