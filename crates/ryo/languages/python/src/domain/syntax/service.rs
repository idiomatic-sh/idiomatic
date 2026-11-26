//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no ports. It moves about a file's
//! syntax tree: a position is an element of the table; its kind is a node, a token or an extra, by the standards' names.



use super::types::{Kind};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::service::category as node_category;
use crate::domain::nodes::types::{Category, Node};
use crate::domain::tokens::types::Token;

ryo::languages::kinds!(pub Kind { Node(Node), Token(Token), Extra(Extra) } unknown Kind::Token(Token::Errortoken));

/// The standard's category of a kind; `Other` for a token or an extra.
#[must_use]
pub(crate) const fn category_of(kind: Kind) -> Category {
    match kind {
        Kind::Node(each) => node_category(each),
        Kind::Token(_) | Kind::Extra(_) => Category::Other,
    }
}
