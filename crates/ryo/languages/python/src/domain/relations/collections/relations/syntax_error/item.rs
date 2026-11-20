//! The `syntax_error` relation of the language: every diagnostic of the parser, kept beside the recovered tree.

use crate::domain::file_view::types::PythonWalk;
use crate::domain::model::types::Declarations;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{
    diagnostic_id, diagnostic_message, diagnostic_span, is_root, kind_bits,
};
use crate::domain::syntax::types::{Kind, Rootness};
use ryo::languages;

/// Reads the `syntax-error` rows of one Python file from its syntax tree.
pub(crate) struct SyntaxErrorItem;

ryo::reads! {
    SyntaxErrorItem => languages::SyntaxError, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        require (is_root(tree, at) == Rootness::Root);
        each (languages::diagnostics(tree.elements)) as (number, diagnostic) {
            given phase = languages::intern_derived(symbols, tree.source, languages::phase_name(diagnostic.phase));
            given impact = languages::intern_derived(symbols, tree.source, languages::impact_name(diagnostic.impact));
            given code = languages::intern_derived(symbols, tree.source, languages::NameText(diagnostic.code.0));
            row {
                id: value(diagnostic_id(tree, number)),
                message: value(diagnostic_message(tree, symbols, &diagnostic)),
                phase: value(phase),
                impact: value(impact),
                code: value(code),
                file: file,
                span: value(diagnostic_span(&diagnostic)),
            }
        }
    }
}

// The engine fills `syntax-error` from the walked file: every row of the file, once.
ryo::fills! {
    SyntaxErrorItem => languages::SyntaxError, walk(PythonWalk) |file, words, rows| {
        // One row for each thing the parser and the language said of the file, in the order the walk recorded them.
        languages::facts::syntax_error_rows(file.syntax.table, file.syntax.file, words, rows);
    }
}
