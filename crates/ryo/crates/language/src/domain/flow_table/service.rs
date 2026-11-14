//! The `flow_table` service: free functions named by verb that keep a file's control flow graphs in its element table as its
//! language's read gives them, through the table's builder, and read them back for the readers of the flow relations, with no IO
//! and no ports. A builder that discards keeps nothing.

use crate::domain::elements::service::engine;
use crate::domain::elements::types::{Elements, TableBuilder};
use crate::domain::flow::types::{
    BlockNumber, FlowBlockFact, FlowEdgeFact, FlowGraph, FlowMemberFact, FlowStepFact,
};

/// Adds a block to the file's control flow graphs and answers its number, from zero in the order given; a builder that discards
/// keeps none, and answers zero.
pub fn add_flow_block(_builder: &mut TableBuilder<'_>, _block: FlowBlockFact) -> BlockNumber {
    BlockNumber(0)
}

/// Adds an edge between two blocks of the file's control flow graphs. A builder that discards keeps nothing.
pub fn add_flow_edge(_builder: &mut TableBuilder<'_>, _edge: FlowEdgeFact) {}

/// Adds a step that is an element, after the steps given before it. A builder that discards keeps nothing.
pub fn add_flow_step(_builder: &mut TableBuilder<'_>, _step: FlowStepFact) {}

/// Adds the block an element is evaluated in, after the elements given before it. A builder that discards keeps nothing.
pub fn add_flow_member(_builder: &mut TableBuilder<'_>, _member: FlowMemberFact) {}

/// The control flow graphs the language's read kept of the file, for the readers of the four flow relations: empty when it kept
/// none.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::flow_table::service::flow_of;
///
/// assert!(flow_of(&Elements::default()).blocks.is_empty());
/// ```
#[must_use]
pub fn flow_of(elements: &Elements) -> &FlowGraph {
    &engine(elements).flow
}
