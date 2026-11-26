//! The `prose` relation of the language: the text a reader reads, in runs: a paragraph, the title of a heading, a cell of a table, and
//! the text an item of a list holds itself. Its markup is joined, its code, math and raw HTML are masked.

use crate::domain::file_view::service::{node_cell, sectioned, span_at, word};
use crate::domain::file_view::types::MarkdownWalk;
use crate::domain::prose::types::Wording;
use crate::domain::walked_model::service::owner as walked_owner;
use crate::domain::walked_prose::service::{has_words as walked_has_words, run_text};
use ryo::languages::node_table;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The item the engine's fill of the relation is registered on.
pub(crate) struct ProseItem;

// The engine fills `prose` from the walked file: the words of every run that has some.
ryo::fills! {
    ProseItem => languages::Prose, walk(MarkdownWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Prose as languages::CatalogRelation>::Row;
        let (sections, syntax, text) = sectioned(file);
        for held in [Node::Paragraph, Node::Heading, Node::ListItem, Node::TableCell] {
            for &id in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Node(held))) {
                if walked_has_words(&syntax, id) != Wording::Words {
                    continue;
                }
                run_text(&syntax, id, text);
                rows.write(Row {
                    id: node_cell(&syntax, id),
                    owner: walked_owner(&syntax, sections, id),
                    text: word(words, text),
                    file: syntax.file,
                    span: span_at(&syntax, id),
                });
            }
        }
    }
}
