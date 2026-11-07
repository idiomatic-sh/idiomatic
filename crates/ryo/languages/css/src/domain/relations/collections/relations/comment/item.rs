//! The `comment` relation of the language: every comment, with its text between `/*` and `*/` as the library read it.

use crate::domain::extras::types::Extra;
use crate::domain::syntax::service::{decoded, kind_bits};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `comment` relation to, which reads its rows from a file's tree.
pub(crate) struct CommentItem;

ryo::reads! {
    CommentItem => languages::Comment |tree, at, symbols| {
        kinds: [Kind::Extra(Extra::Comment)] as kind_bits,
        given text = decoded(tree, symbols, at)?;
        row { id: node(at), text: value(text), file: file, span: span(at), doc: value(languages::Flag::Lowered) }
    }
}
