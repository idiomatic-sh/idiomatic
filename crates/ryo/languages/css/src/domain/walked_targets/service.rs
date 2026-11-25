//! The `walked_targets` service: free functions over a walked file's syntax that find the token on which the adapter wrote what an
//! `@import` imports and what a `url()` points at, and what each belongs to: the nearest declaration or rule around it, or the
//! stylesheet.

use crate::domain::file_view::service::{children, decoded, kind, parent, span_at, written};
use crate::domain::file_view::types::Syntax;
use crate::domain::nodes::service::category;
use crate::domain::nodes::types::{Category, Node};
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::targets::constants::NAMESPACE;
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Position, node_table};

/// The first child of `at` of `wanted` kind.
fn child_of_kind(syntax: &Syntax<'_>, at: Position, wanted: Kind) -> Option<Position> {
    children(syntax, at).find(|child| kind(syntax, *child) == wanted)
}

/// Whether the element `at` is the root: its own parent.
fn is_root(syntax: &Syntax<'_>, at: Position) -> bool {
    parent(syntax, at) == at
}

/// The token that carries what the `url()` or url-token at `at` points at, when it carries one.
#[must_use]
pub(crate) fn url_token(syntax: &Syntax<'_>, at: Position) -> Option<Position> {
    let found = kind(syntax, at);
    let token = if found == Kind::Token(Token::Url) {
        at
    } else if found == Kind::Node(Node::Function) {
        child_of_kind(syntax, at, Kind::Token(Token::Function))?
    } else {
        return None;
    };
    decoded(syntax, token).map(|_| token)
}

/// The token that carries what the `@import` at `at` imports, when it carries one.
#[must_use]
pub(crate) fn import_token(syntax: &Syntax<'_>, at: Position) -> Option<Position> {
    if kind(syntax, at) != Kind::Node(Node::AtRule) {
        return None;
    }
    let keyword = child_of_kind(syntax, at, Kind::Token(Token::AtKeyword))?;
    decoded(syntax, keyword).map(|_| keyword)
}

/// The reference the `url()` at `at` is, unless it is in an `@import` or an `@namespace`, which name their targets another way.
#[must_use]
pub(crate) fn reference(syntax: &Syntax<'_>, at: Position) -> Option<Position> {
    let mut place = parent(syntax, at);
    while kind(syntax, place) != Kind::Node(Node::AtRule) && !is_root(syntax, place) {
        place = parent(syntax, place);
    }
    let keyword = (kind(syntax, place) == Kind::Node(Node::AtRule))
        .then(|| child_of_kind(syntax, place, Kind::Token(Token::AtKeyword)))
        .flatten();
    let excluded = keyword.is_some_and(|keyword| {
        import_token(syntax, place).is_some() || written(syntax, keyword).eq_ignore_ascii_case(NAMESPACE)
    });
    if excluded { None } else { Some(at) }
}

/// What the element at `at` belongs to: the nearest rule or declaration around it, or the stylesheet.
#[must_use]
pub(crate) fn owner(syntax: &Syntax<'_>, at: Position) -> Position {
    let mut place = parent(syntax, at);
    while matches!(kind(syntax, place), Kind::Node(node) if category(node) == Category::ComponentValue)
        && !is_root(syntax, place)
    {
        place = parent(syntax, place);
    }
    place
}

/// Whether the string at `at` is the first of an `@import`: the one that says what it imports.
#[must_use]
pub(crate) fn specifier(syntax: &Syntax<'_>, at: Position) -> languages::Flag {
    let rule = owner(syntax, at);
    // The first string of the rule: no string token of the rule comes before this one. Tokens are numbered in the order of the file.
    let first = import_token(syntax, rule).is_none()
        || !node_table::ids_of_kind(syntax.table, kind_bits(Kind::Token(Token::String)))
            .iter()
            .any(|earlier| {
                *earlier < at && span_at(syntax, *earlier).start >= span_at(syntax, rule).start
            });
    if import_token(syntax, rule).is_some() && first {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}
