//! The types of the `sharing` concept: plain wrappers, and values declared in their own modules.
use crate::domain::adjacency::types::compact_graph::value::CompactGraph;
use crate::domain::equality::types::KeyIndex;
use std::sync::Arc;

/// A key index shared by every holder of the payload it was built into.
#[derive(Clone, Debug)]
pub struct SharedKeyIndex(pub Arc<KeyIndex>);

/// A compact graph shared by every holder of the payload it was built into.
#[derive(Clone, Debug)]
pub struct SharedGraph(pub Arc<CompactGraph>);

pub mod shared_positions;

pub mod shared_marks;
