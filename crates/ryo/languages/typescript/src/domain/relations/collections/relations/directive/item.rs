//! typescript `directive` rows: the directives of a prologue, by the word between their quotes.
use crate::domain::file_view::service::{node_cell, of_kinds, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::syntax::reading::WordSink;
use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::prologues::service::{argument, directive};
use crate::domain::syntax::{service::kind_bits, types::Kind};
use ryo::languages;

/// Reads the `directive` rows of one TypeScript file from its syntax tree.
pub(crate) struct DirectiveItem;
ryo::reads! {
    DirectiveItem => languages::Directive, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::ExpressionStatement)] as kind_bits,
        given word = directive(tree, symbols, at)?;
        row { file: file, span: span(at), kind: value(word), text: value(argument(tree, symbols)) }
    }
}

// The engine fills `directive` from the walked file: every row of the file, once.
ryo::fills! {
    DirectiveItem => languages::Directive, walk(TypeScriptWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Directive as languages::CatalogRelation>::Row;
        let mut words = WordSink::new(words);
        let file = &*file;
        for at in of_kinds(file, &[Kind::Node(Node::ExpressionStatement)]) {
            let Some(word) = directive(file, &mut words, at) else {
                continue;
            };
            rows.write(Row {
                file: file.file,
                span: span_at(file, at),
                kind: word,
                text: argument(file, &mut words),
            });
        }
    }
}
