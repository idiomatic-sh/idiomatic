//! TypeScript `flow_edge` rows: the edges between the basic blocks of the file's control flow graphs, as the read kept them from `oxc_semantic`'s graph.

use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `flow-edge` rows of one TypeScript file from the graphs its read kept.
pub(crate) struct FlowEdgeItem;

ryo::reads! {
    FlowEdgeItem => languages::FlowEdge, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Program)] as kind_bits,
        each (languages::flow_of(tree.elements).edges.iter()) as edge {
            row {
                source: value(languages::Integer(i64::from(edge.source.0))),
                target: value(languages::Integer(i64::from(edge.target.0))),
                edge_kind: value(languages::intern_derived(symbols, tree.source, languages::flow_edge_word(edge.kind))),
                file: file,
            }
        }
    }
}
