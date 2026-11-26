//! TypeScript comment rows from its standard syntax tree.
use crate::domain::file_view::service::{node_cell, of_kinds, span_at, word, written};
use crate::domain::extras::types::Extra;
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service as names;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use crate::domain::tokens::types::Token;
use ryo::languages;

/// Reads the `comment` rows of one TypeScript file from its syntax tree.
pub(crate) struct CommentItem;
ryo::reads! {
 CommentItem => languages::Comment, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Token(Token::Line), Kind::Token(Token::Block), Kind::Extra(Extra::Hashbang)] as kind_bits,


 row { id: node(at), text: value(names::comment(tree,symbols,at)), file: file, span: span(at), doc: value(crate::domain::syntax::service::documentation(tree, at)) }

 }
}

// The engine fills `comment` from the walked file: every row of the file, once.
ryo::fills! {
    CommentItem => languages::Comment, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Comment as languages::CatalogRelation>::Row;
        for at in of_kinds(file, &[Kind::Token(Token::Line), Kind::Token(Token::Block), Kind::Extra(Extra::Hashbang)]) {
            let raw = written(file, at).0;
            rows.write(Row {
                id: node_cell(file, at),
                text: word(words, languages::NameText(names::comment_content(raw))),
                file: file.file,
                span: span_at(file, at),
                doc: crate::domain::syntax::service::documentation_of(raw),
            });
        }
    }
}
