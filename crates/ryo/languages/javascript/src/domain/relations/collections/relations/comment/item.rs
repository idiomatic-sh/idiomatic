//! JavaScript comment facts from the `ESTree` projection.
use crate::domain::syntax::service as syntax;
use crate::domain::syntax::{service::kind_bits, types::Kind};
use crate::domain::tokens::types::Token;
use ryo::languages;
/// Reads the `comment` rows of one JavaScript file from its syntax tree.
pub(crate) struct CommentItem;
ryo::reads! {CommentItem => languages::Comment |tree,at,symbols|{kinds:[Kind::Token(Token::Line),Kind::Token(Token::Block)] as kind_bits,

row{id: node(at),text: value(syntax::comment(tree,symbols,at)),file:file,span:span(at),doc: value(syntax::documentation(tree,at))}}}
