//! TypeScript `flow_edge` rows: the edges between the basic blocks of the file's control flow graphs, as the read kept them from `oxc_semantic`'s graph.

use crate::domain::file_view::service::node_cell;
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
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
                ordinal: value(languages::Integer(i64::from(edge.ordinal.0))),
                file: file,
            }
        }
    }
}

// The engine fills `flow-edge` from the walked file: every edge of the file's control flow graphs, once.
ryo::fills! {
    FlowEdgeItem => languages::FlowEdge, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::FlowEdge as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let Some(flow) = &file.flow else {
            return;
        };
        for edge in &flow.edges {
            rows.write(Row {
                source: languages::Integer(i64::from(edge.source.0)),
                target: languages::Integer(i64::from(edge.target.0)),
                edge_kind: crate::domain::syntax::reading::Interner::derived(&mut words, file.source, languages::flow_edge_word(edge.kind)),
                ordinal: languages::Integer(i64::from(edge.ordinal.0)),
                file: file.file,
            });
        }
    }
}
