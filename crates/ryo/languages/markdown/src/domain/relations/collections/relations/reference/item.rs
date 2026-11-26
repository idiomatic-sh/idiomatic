//! The `reference` relation of the language: every link and every image, with what it points at, resolved to a path of the run as an
//! import is (`[a](../guide.md#setup)` in `docs/a.md` points at `guide.md#setup`).

use crate::domain::file_view::service::{named_by as walked_named_by, node_cell, sectioned, span_at, word};
use crate::domain::file_view::types::MarkdownWalk;
use crate::domain::walked_model::service::owner as walked_owner;
use ryo::languages::node_table;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The item the engine's fill of the relation is registered on.
pub(crate) struct ReferenceItem;

// The engine fills `reference` from the walked file: every link and image, with the target it names.
ryo::fills! {
    ReferenceItem => languages::Reference, walk(MarkdownWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Reference as languages::CatalogRelation>::Row;
        let (sections, syntax, _text) = sectioned(file);
        for held in [Node::Link, Node::LinkReference, Node::Image, Node::ImageReference] {
            for &id in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Node(held))) {
                let Some(destination) = walked_named_by(&syntax, id) else { continue };
                rows.write(Row {
                    occurrence: node_cell(&syntax, id),
                    owner: walked_owner(&syntax, sections, id),
                    target: word(words, destination),
                    file: syntax.file,
                    span: span_at(&syntax, id),
                });
            }
        }
    }
}
