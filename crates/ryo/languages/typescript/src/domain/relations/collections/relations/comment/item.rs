//! TypeScript comment rows from its standard syntax tree.
use crate::domain::model::types::ReadIndexes;
use crate::domain::names::service as names;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use crate::domain::tokens::types::Token;
use ryo::languages;

/// Reads the `comment` rows of one TypeScript file from its syntax tree.
pub(crate) struct CommentItem;
ryo::reads! {
 CommentItem => languages::Comment, model(ReadIndexes) |tree, _model, at, symbols| {
 kinds: [Kind::Token(Token::Line), Kind::Token(Token::Block)] as kind_bits,


 row { id: node(at), text: value(names::comment(tree,symbols,at)), file: file, span: span(at), doc: value(crate::domain::syntax::service::documentation(tree, at)) }

 }
}
