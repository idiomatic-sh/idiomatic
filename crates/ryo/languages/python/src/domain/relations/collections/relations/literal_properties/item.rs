//! Emit optional primitive-literal properties from parser-owned payloads.

use super::constants::INCOMPLETE;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::literal_facts::service::properties;
use crate::domain::node::service::{fixed, node_cell, nodes_of_kind, span_at};
use crate::domain::node::types::Numbered;
use crate::domain::nodes::types::Node as TypedNode;
use crate::domain::{
    literal_properties::{service::classify, types::LiteralClassification},
    model::types::Declarations,
    nodes::types::Node,
    semantic_syntax::types::Visibility,
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;

/// The optional Python primitive-literal reader.
pub(crate) struct LiteralPropertiesItem;

ryo::reads! {
    LiteralPropertiesItem => languages::LiteralProperties, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits, expanded,
        each (crate::domain::semantic_syntax::service::positions(tree)) as expression {
            require crate::domain::semantic_syntax::service::visibility(tree, model, expression, symbols) == Visibility::Visible;
            when classify(tree, expression) {
                LiteralClassification::NotPrimitive => {}
                LiteralClassification::MissingPayload => {
                    partial INCOMPLETE => languages::Missing::Any;
                }
                LiteralClassification::Known(properties) => {
                    given kind = languages::intern_derived(symbols, tree.source, properties.kind);
                    given zero = properties.zero;
                    given empty = properties.empty;
                    given boolean = properties.boolean;
                    row { id: node(expression), kind: value(kind), zero: value(zero), empty: value(empty), boolean: value(boolean), file: file, span: span(expression), }
                }
            }
        }
    }
}

// The engine fills `literal-properties` from the walked file: every row of the file, once.
ryo::fills! {
    LiteralPropertiesItem => languages::LiteralProperties, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::LiteralProperties as languages::CatalogRelation>::Row;
        let memo = &mut file.buffers.fixed;
        for Numbered { id, node } in nodes_of_kind(&file.syntax, Kind::Node(TypedNode::Constant)) {
            if let Some(found) = properties(node) {
                rows.write(Row {
                    id: node_cell(&file.syntax, id),
                    kind: fixed(words, memo, found.kind),
                    zero: found.zero,
                    empty: found.empty,
                    boolean: found.boolean,
                    file: file.syntax.file,
                    span: span_at(&file.syntax, id),
                });
            }
        }
    }
}
