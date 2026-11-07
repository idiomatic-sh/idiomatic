//! The types of the `sharing` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use crate::domain::adjacency::types::compact_graph::value::CompactGraph;
use crate::domain::equality::types::KeyIndex;
use std::sync::Arc;

/// Immutable shared membership marks over a dense element space.
#[derive(Clone, Debug)]
pub struct SharedMarks(pub Arc<Vec<bool>>);

/// A key index shared by every holder of the payload it was built into.
#[derive(Clone, Debug)]
pub struct SharedKeyIndex(pub Arc<KeyIndex>);

/// A compact graph shared by every holder of the payload it was built into.
#[derive(Clone, Debug)]
pub struct SharedGraph(pub Arc<CompactGraph>);
