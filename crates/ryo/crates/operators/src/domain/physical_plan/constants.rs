//! The stages of the swap that serve each feature of a program: the one definition of which stage a kind, a reach or a step
//! after the files is served at.

use super::types::ServedAt;

/// The stage whose kernels lower the relational kinds: select, project, extend, join, the set operations and the count.
pub const RELATIONAL_STAGE: ServedAt = ServedAt(1);

/// The stage whose kernels lower the text and span kinds: capture, rank, replace, words and the span and render kinds.
pub const TEXT_STAGE: ServedAt = ServedAt(2);

/// The stage whose kernels lower the graph kinds and a join's reach: axis, closure, ordered walks, cycles and digests.
pub const GRAPH_STAGE: ServedAt = ServedAt(3);

/// The stage that serves what runs after the files: a step across files, a bound input, an observation, rows and hosted
/// tables.
pub const BOUND_STAGE: ServedAt = ServedAt(4);
