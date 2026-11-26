//! The `comment` relation of the language: every comment, line or general, with its text without the delimiters, and whether `go/doc`
//! reads it as documentation.


use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

use crate::domain::file_view::types::{GoWalk, Words};
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `comment` relation to, which reads its rows from a file's tree.
pub(crate) struct CommentItem;

// The engine fills `comment` from the walked file: every comment, with its text and whether it documents a declaration.
ryo::fills! {
    CommentItem => languages::Comment, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Comment as languages::CatalogRelation>::Row;
        let tree = file.syntax;
        let words = Words(std::cell::RefCell::new(words));
        for &at in node_table::ids_of_kind(tree.table, kind_bits(Kind::Token(Token::Comment))) {
            rows.write(Row {
                id: walked_syntax::node_cell(&tree, at),
                text: crate::domain::walked::names::comment_text(&tree, &words, at),
                file: tree.file,
                span: walked_syntax::span_of(&tree, at),
                doc: crate::domain::walked::names::documentation(&tree, at),
            });
        }
    }
}
