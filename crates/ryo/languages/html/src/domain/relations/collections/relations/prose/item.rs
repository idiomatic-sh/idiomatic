//! The `prose` relation, read from the HTML DOM.
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::{Kind, Wording};
use crate::domain::file_view::types::HtmlWalk;
use crate::domain::file_view::service::{elements, kind as view_kind, node_cell, span_at, word};
use crate::domain::walked_values::service as walked;
use ryo::languages;
/// The item the engine's fill of the `prose` relation is registered on.
pub(crate) struct ProseItem;

// The engine fills `prose` from the walked file: every block element that holds a run of words, with the run.
ryo::fills! {
    ProseItem => languages::Prose, walk(HtmlWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Prose as languages::CatalogRelation>::Row;
        let view = *file;
        for at in elements(&view) {
            if view_kind(&view, at) == Kind::Node(Node::Element) && walked::run_words(&view, at) == Wording::Words {
                rows.write(Row {
                    id: node_cell(&view, at),
                    owner: node_cell(&view, walked::run_owner(&view, at)),
                    text: word(words, &walked::run_name(&view, at)),
                    file: view.file,
                    span: span_at(&view, at),
                });
            }
        }
    }
}

