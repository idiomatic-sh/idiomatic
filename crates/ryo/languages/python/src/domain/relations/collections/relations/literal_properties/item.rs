//! Emit optional primitive-literal properties from parser-owned payloads.

use super::constants::INCOMPLETE;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::literal_facts::service::properties;
use crate::domain::model::names::service::fixed_word;
use crate::domain::node::service::{node_cell, place_of_id, span_at};
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
        let memo = &mut file.model.words.get_mut().fixed;
        let tree = file.syntax.tree;
        let kind = kind_bits(Kind::Node(TypedNode::Constant));
        for &id in languages::node_table::ids_of_kind(file.syntax.table, kind) {
            let Some(place) = place_of_id(tree, id) else { continue };
            if let Some(found) = properties(tree.nodes[usize::from(place)]) {
                rows.write(Row {
                    id: node_cell(&file.syntax, id),
                    kind: fixed_word(words, memo, found.kind),
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
