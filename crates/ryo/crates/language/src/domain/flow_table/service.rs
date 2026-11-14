//! The `flow_table` service: free functions named by verb that keep a file's control flow graphs in its element table as its
//! language's read gives them, through the table's builder, and read them back for the readers of the flow relations, with no IO
//! and no ports. A builder that discards keeps nothing.

use crate::domain::builders::service::builder_engine;
use crate::domain::elements::service::{engine, engine_mut};
use crate::domain::elements::types::{Elements, TableBuilder, TableWriting};
use crate::domain::flow::types::{
    BlockNumber, FlowBlockFact, FlowEdgeFact, FlowGraph, FlowMemberFact, FlowStepFact,
};

/// Runs `keep` on the control flow graphs `builder` writes into; a builder that discards keeps nothing, so it runs nothing.
fn kept_flow(builder: &mut TableBuilder<'_>, keep: impl FnOnce(&mut FlowGraph)) {
    let builder = builder_engine(builder);
    if builder.writing == TableWriting::Writes {
        keep(&mut engine_mut(builder.elements).flow);
    }
}

/// Adds a block to the file's control flow graphs and answers its number, from zero in the order given; a builder that discards
/// keeps none, and answers zero.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::builders::service::table_builder;
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::flow_table::service::{add_flow_block, flow_of};
/// use ryo_language::domain::flow::types::{BlockNumber, FlowBlockFact};
/// use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
/// use ryo_relations::domain::relations::types::{Flag};
///
/// let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
/// let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"f()"));
/// let block = FlowBlockFact { reachable: Flag::Raised, live: Flag::Raised, implicit_return: Flag::Raised };
/// assert_eq!(add_flow_block(&mut builder, block), BlockNumber(0));
/// assert_eq!(add_flow_block(&mut builder, block), BlockNumber(1));
/// assert_eq!(flow_of(&elements).blocks.len(), 2);
/// ```
pub fn add_flow_block(builder: &mut TableBuilder<'_>, block: FlowBlockFact) -> BlockNumber {
    let mut number = BlockNumber(0);
    kept_flow(builder, |graph| {
        number = BlockNumber(u32::try_from(graph.blocks.len()).unwrap_or(u32::MAX));
        graph.blocks.push(block);
    });
    number
}

/// Adds an edge between two blocks of the file's control flow graphs. A builder that discards keeps nothing.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::builders::service::table_builder;
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::flow_table::service::{add_flow_block, add_flow_edge, flow_of};
/// use ryo_language::domain::flow::types::{FlowBlockFact, FlowEdgeFact, FlowEdgeKind};
/// use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
/// use ryo_relations::domain::relations::types::{Flag};
///
/// let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
/// let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"f()"));
/// let block = FlowBlockFact { reachable: Flag::Raised, live: Flag::Raised, implicit_return: Flag::Raised };
/// let (source, target) = (add_flow_block(&mut builder, block), add_flow_block(&mut builder, block));
/// add_flow_edge(&mut builder, FlowEdgeFact { source, target, kind: FlowEdgeKind::Normal });
/// assert_eq!(flow_of(&elements).edges.len(), 1);
/// ```
pub fn add_flow_edge(builder: &mut TableBuilder<'_>, edge: FlowEdgeFact) {
    kept_flow(builder, |graph| graph.edges.push(edge));
}

/// Adds a step that is an element, after the steps given before it. A builder that discards keeps nothing.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::builders::service::table_builder;
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::flow_table::service::{add_flow_block, add_flow_step, flow_of};
/// use ryo_language::domain::flow::types::{FlowBlockFact, FlowStepFact, FlowStepKind, StepOrdinal};
/// use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
/// use ryo_relations::domain::relations::types::{Flag, Position};
///
/// let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
/// let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"f()"));
/// let block = FlowBlockFact { reachable: Flag::Raised, live: Flag::Raised, implicit_return: Flag::Raised };
/// let entry = add_flow_block(&mut builder, block);
/// add_flow_step(&mut builder, FlowStepFact { block: entry, ordinal: StepOrdinal(0), kind: FlowStepKind::Statement, node: Position(0) });
/// assert_eq!(flow_of(&elements).steps.len(), 1);
/// ```
pub fn add_flow_step(builder: &mut TableBuilder<'_>, step: FlowStepFact) {
    kept_flow(builder, |graph| graph.steps.push(step));
}

/// Adds the block an element is evaluated in, after the elements given before it. A builder that discards keeps nothing.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::builders::service::table_builder;
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::flow_table::service::{add_flow_block, add_flow_member, flow_of};
/// use ryo_language::domain::flow::types::{FlowBlockFact, FlowMemberFact};
/// use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
/// use ryo_relations::domain::relations::types::{Flag, Position};
///
/// let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
/// let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"f()"));
/// let block = FlowBlockFact { reachable: Flag::Raised, live: Flag::Raised, implicit_return: Flag::Raised };
/// let entry = add_flow_block(&mut builder, block);
/// add_flow_member(&mut builder, FlowMemberFact { node: Position(0), block: entry });
/// assert_eq!(flow_of(&elements).members.len(), 1);
/// ```
pub fn add_flow_member(builder: &mut TableBuilder<'_>, member: FlowMemberFact) {
    kept_flow(builder, |graph| graph.members.push(member));
}

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
