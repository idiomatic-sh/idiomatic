//! The `scope` relation of the language: the document, which encloses itself, and a section under the section that holds it, the
//! document's for the outermost, as far as the next heading as shallow or shallower.

use crate::domain::file_view::service::{node_cell, sectioned, span_at};
use crate::domain::file_view::types::MarkdownWalk;
use crate::domain::model::types::Scoped;
use crate::domain::walked_model::service::scoped as walked_scoped;
use ryo::languages::node_table;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The item the engine's fill of the relation is registered on.
pub(crate) struct ScopeItem;

// The engine fills `scope` from the walked file: the document, and each section a heading opens.
ryo::fills! {
    ScopeItem => languages::Scope, walk(MarkdownWalk) |file, _words, rows| {
        /// One row of the relation.
        type Row = <languages::Scope as languages::CatalogRelation>::Row;
        let (sections, syntax, _text) = sectioned(file);
        for held in [Node::Root, Node::Heading] {
            for &id in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Node(held))) {
                match walked_scoped(&syntax, sections, id) {
                    Scoped::Document => rows.write(Row {
                        parent: node_cell(&syntax, id),
                        child: node_cell(&syntax, id),
                        file: syntax.file,
                        span: span_at(&syntax, id),
                    }),
                    Scoped::Section(section) => rows.write(Row {
                        parent: node_cell(&syntax, section.parent),
                        child: node_cell(&syntax, id),
                        file: syntax.file,
                        span: languages::Span {
                            start: span_at(&syntax, id).start,
                            end: section.end,
                        },
                    }),
                    Scoped::Nothing => {}
                }
            }
        }
    }
}
