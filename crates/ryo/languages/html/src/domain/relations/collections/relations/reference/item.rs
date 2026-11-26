//! The `reference` relation, read from the HTML DOM: every URL-valued attribute, with what it points at resolved to a path of the run
//! as a link is (`href="../guide.html#setup"` in `docs/a.html` points at `guide.html#setup`).
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::file_view::types::HtmlWalk;
use crate::domain::file_view::service::{elements, kind as view_kind, node_cell, span_at, target as view_target, word};
use crate::domain::walked_values::service as walked;
use crate::domain::syntax::types::Reference;
use ryo::languages;
/// The item the engine's fill of the `reference` relation is registered on.
pub(crate) struct ReferenceItem;

// The engine fills `reference` from the walked file: every URL-valued attribute, with the path of the run it points at.
ryo::fills! {
    ReferenceItem => languages::Reference, walk(HtmlWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Reference as languages::CatalogRelation>::Row;
        let view = *file;
        for at in elements(&view) {
            if view_kind(&view, at) == Kind::Node(Node::Attribute) && walked::reference(&view, at) == Reference::Url {
                rows.write(Row {
                    occurrence: node_cell(&view, at),
                    owner: node_cell(&view, walked::owner(&view, at)),
                    target: word(words, view_target(&view, at)),
                    file: view.file,
                    span: span_at(&view, at),
                });
            }
        }
    }
}

