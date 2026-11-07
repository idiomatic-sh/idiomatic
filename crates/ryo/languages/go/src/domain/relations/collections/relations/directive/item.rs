//! The `directive` relation of the language: every build constraint line of a file's header, `//go:build` and `// +build`, by its kind
//! and its expression, as `go/build` reads them.

use crate::domain::constraints::service::constraint_at;
use crate::domain::constraints::types::Constraint;
use crate::domain::names::service::{directive_kind, directive_text};
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The reader the `parse` contract hands the `directive` relation to, which reads its rows from a file's tree.
pub(crate) struct DirectiveItem;

ryo::reads! {
    DirectiveItem => languages::Directive |tree, at, symbols| {
        kinds: [Kind::Token(Token::Comment)] as kind_bits,
        require (constraint_at(tree, at) != Constraint::Other);
        row {
            file: file,
            span: span(at),
            kind: value(directive_kind(tree, symbols, at)),
            text: value(directive_text(tree, symbols, at)),
        }
    }
}
