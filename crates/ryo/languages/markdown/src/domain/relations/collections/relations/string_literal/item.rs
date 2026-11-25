//! The `string_literal` relation of the language: every piece of code, in a line of text or a block of its own, as the library
//! decoded it, in its place. Markdown has no other literal: its text is prose, and code is the one text it holds as written.

use crate::domain::file_view::service::{node_cell, parent as walked_parent, span_at, word};
use crate::domain::file_view::types::MarkdownWalk;
use crate::domain::walked_prose::service::code_text;
use ryo::languages::node_table;
use crate::domain::model::types::Sections;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{code_value, kind_bits, parent};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `string_literal` relation to, which reads its rows from a file's tree.
pub(crate) struct StringLiteralItem;

ryo::reads! {
    StringLiteralItem => languages::StringLiteral, model(Sections) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::InlineCode), Kind::Node(Node::Code), Kind::Node(Node::Code)] as kind_bits,
        row {
            occurrence: node(at),
            owner: node(parent(tree, at)),
            value: value(code_value(tree, symbols, at)),
            file: file,
            span: span(at),
            import: value(languages::Flag::Lowered),
        }
    }
}

// The engine fills `string_literal` from the walked file: the code of every span and every block.
ryo::fills! {
    StringLiteralItem => languages::StringLiteral, walk(MarkdownWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StringLiteral as languages::CatalogRelation>::Row;
        let syntax = file.syntax;
        for held in [Node::InlineCode, Node::Code] {
            for &id in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Node(held))) {
                code_text(&syntax, id, file.text);
                rows.write(Row {
                    occurrence: node_cell(&syntax, id),
                    owner: node_cell(&syntax, walked_parent(&syntax, id)),
                    value: word(words, file.text),
                    file: syntax.file,
                    span: span_at(&syntax, id),
                    import: languages::Flag::Lowered,
                });
            }
        }
    }
}
