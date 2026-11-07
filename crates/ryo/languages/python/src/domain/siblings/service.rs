//! The `siblings` service: free functions named by verb over the domain's types, with no IO and no ports, that say where an element
//! stands among the children of its parent: how many nodes come before it, whether one follows, and the keyword or the punctuation
//! before it, which is how a parser tells the fields of a node apart when its walk does not name them: from these, the section a
//! parameter is in, the block a statement is in, the role of a token under its node and of a node under an extra.

use ryo::languages::{self, Position, Tree};

use super::constants::{ASYNC, ELSE, FINALLY};
use super::types::{Holding, Placement, SiblingIndex, Weight, Word};
use crate::domain::extras::types::Extra;
use crate::domain::labels::types::{Field, Section};
use crate::domain::nodes::types::{Category, Node};
use crate::domain::syntax::service::{category, is_token, kind, parent, written};
use crate::domain::syntax::types::{Kind, Leaf};
use crate::domain::tokens::types::Token;

/// What the token at `at` is to the roles of its siblings: trivia, a parenthesis, or written for what it says.
pub(crate) fn weight(tree: &Tree<'_>, at: Position) -> Weight {
    let element = kind(tree, at);
    if matches!(
        element,
        Kind::Extra(Extra::Whitespace)
            | Kind::Token(
                Token::Comment
                    | Token::Nl
                    | Token::Newline
                    | Token::Indent
                    | Token::Dedent
                    | Token::Endmarker
            )
    ) {
        Weight::Trivia
    } else if matches!(element, Kind::Token(Token::Lpar | Token::Rpar)) {
        Weight::Parenthesis
    } else {
        Weight::Written
    }
}

/// The elements under the parent of `at` before it, in order.
pub(crate) fn before<'a>(tree: &'a Tree<'_>, at: Position) -> impl Iterator<Item = Position> + 'a {
    languages::children(tree.elements, parent(tree, at)).take_while(move |each| *each != at)
}

/// How many nodes stand under the parent of `at` before it.
pub(crate) fn index(tree: &Tree<'_>, at: Position) -> SiblingIndex {
    SiblingIndex(
        before(tree, at)
            .filter(|each| is_token(tree, *each) == Leaf::Node)
            .count(),
    )
}

/// Whether a node stands under the parent of `at` after it.
pub(crate) fn placement(tree: &Tree<'_>, at: Position) -> Placement {
    if languages::children(tree.elements, parent(tree, at))
        .skip_while(|each| *each != at)
        .skip(1)
        .any(|each| is_token(tree, each) == Leaf::Node)
    {
        Placement::Followed
    } else {
        Placement::Last
    }
}

/// The last token under the parent of `at` before it whose text is one of `words`, a keyword or a piece of punctuation.
pub(crate) fn last_word<'a>(tree: &Tree<'a>, at: Position, words: &[Word<'_>]) -> Option<Word<'a>> {
    before(tree, at)
        .filter(|each| is_token(tree, *each) == Leaf::Token)
        .map(|each| Word(written(tree, each).0))
        .filter(|text| words.contains(text))
        .last()
}

/// The token right before `at` under its parent, trivia and parentheses passed over; none when a node stands between.
pub(crate) fn preceding(tree: &Tree<'_>, at: Position) -> Option<Kind> {
    let holder = parent(tree, at);
    let mut place = at.0.checked_sub(1)?;
    loop {
        let each = Position(place);
        if each == holder || parent(tree, each) != holder || is_token(tree, each) == Leaf::Node {
            return None;
        }
        if weight(tree, each) == Weight::Written {
            return Some(kind(tree, each));
        }
        place = place.checked_sub(1)?;
    }
}

/// Whether the node at `at` has a token of `wanted` kind right under it.
pub(crate) fn holding(tree: &Tree<'_>, at: Position, wanted: Token) -> Holding {
    if languages::children(tree.elements, at).any(|each| kind(tree, each) == Kind::Token(wanted)) {
        Holding::Holds
    } else {
        Holding::Lacks
    }
}

/// Where a parameter of an `arguments` stands, by the `/` after it and the `*` before it.
pub(crate) fn section(tree: &Tree<'_>, at: Position) -> Section {
    let starred = before(tree, at).any(|each| {
        kind(tree, each) == Kind::Token(Token::Star)
            || (kind(tree, each) == Kind::Node(Node::Arg)
                && holding(tree, each, Token::Star) == Holding::Holds)
    });
    let slashed = languages::children(tree.elements, parent(tree, at))
        .skip_while(|each| *each != at)
        .any(|each| kind(tree, each) == Kind::Token(Token::Slash));
    if starred {
        Section::KeywordOnly
    } else if slashed {
        Section::PositionalOnly
    } else {
        Section::Positional
    }
}

/// The role of a parameter of an `arguments` at `at`, a parameter or a parameter with its default.
pub(crate) fn parameter(tree: &Tree<'_>, at: Position) -> Field {
    if kind(tree, at) == Kind::Node(Node::Arg) {
        if holding(tree, at, Token::Star) == Holding::Holds {
            return Field::Vararg;
        }
        if holding(tree, at, Token::Doublestar) == Holding::Holds {
            return Field::Kwarg;
        }
    }
    match section(tree, at) {
        Section::PositionalOnly => Field::Posonlyargs,
        Section::Positional => Field::Args,
        Section::KeywordOnly => Field::Kwonlyargs,
    }
}

/// The role of a statement of a block that an `else` or a `finally` may close: `body` before them, `orelse` after an `else`,
/// `finalbody` after a `finally`.
pub(crate) fn block(tree: &Tree<'_>, at: Position) -> Field {
    match last_word(tree, at, &[ELSE, FINALLY]) {
        Some(word) if word == ELSE => Field::Orelse,
        Some(_) => Field::Finalbody,
        None => Field::Body,
    }
}

/// The role of a statement of a block an `else` or a `finally` may close, or the other role for every other node.
pub(crate) fn statement_block(tree: &Tree<'_>, at: Position, of: Category, other: Field) -> Field {
    if of == Category::Statement {
        block(tree, at)
    } else {
        other
    }
}

/// The role of the token at `at` under a node of the standard, `holder`: a constant's literal, a name's identifier, a relative
/// import's dots, an operator, an `async` comprehension, an interpolation's conversion; none for every other.
pub(crate) fn token_under(tree: &Tree<'_>, at: Position, holder: Node) -> Field {
    let weight = weight(tree, at);
    let element = kind(tree, at);
    if weight == Weight::Trivia {
        Field::None
    } else if matches!(holder, Node::Constant | Node::MatchSingleton) {
        Field::Value
    } else if holder == Node::Name && element == Kind::Token(Token::Name) {
        Field::Id
    } else if holder == Node::ImportFrom
        && matches!(element, Kind::Token(Token::Dot | Token::Ellipsis))
    {
        Field::Level
    } else if weight == Weight::Parenthesis {
        Field::None
    } else if matches!(
        holder,
        Node::AugAssign | Node::BoolOp | Node::BinOp | Node::UnaryOp
    ) {
        Field::Op
    } else if holder == Node::Compare {
        Field::Ops
    } else if holder == Node::Comprehension && Word(written(tree, at).0) == ASYNC {
        Field::IsAsync
    } else if matches!(holder, Node::FormattedValue | Node::Interpolation)
        && preceding(tree, at) == Some(Kind::Token(Token::Exclamation))
    {
        Field::Conversion
    } else {
        Field::None
    }
}

/// The role of the node at `at` under an extra, `holder`: a part the standard does not have, whose nodes are in the fields of the
/// node of the standard above it.
pub(crate) fn within(tree: &Tree<'_>, at: Position, holder: Extra) -> Field {
    if is_token(tree, at) == Leaf::Token {
        return Field::None;
    }
    let element = kind(tree, at);
    match holder {
        Extra::CallArguments => {
            if element == Kind::Node(Node::Keyword) {
                Field::Keywords
            } else if kind(tree, parent(tree, parent(tree, at))) == Kind::Node(Node::ClassDef) {
                Field::Bases
            } else {
                Field::Args
            }
        }
        Extra::ElifElseClause => {
            if category(tree, at) == Category::Statement {
                Field::Body
            } else {
                Field::Test
            }
        }
        Extra::ParameterWithDefault => {
            let own = parameter(tree, parent(tree, at));
            if element == Kind::Node(Node::Arg) {
                own
            } else if own == Field::Kwonlyargs {
                Field::KwDefaults
            } else {
                Field::Defaults
            }
        }
        Extra::FStringPart | Extra::TStringPart => Field::Values,
        Extra::PatternArguments => {
            if element == Kind::Extra(Extra::PatternKeyword) {
                Field::None
            } else {
                Field::Patterns
            }
        }
        Extra::PatternKeyword => {
            if element == Kind::Extra(Extra::Identifier) {
                Field::KwdAttrs
            } else {
                Field::KwdPatterns
            }
        }
        Extra::Whitespace
        | Extra::Identifier
        | Extra::Decorator
        | Extra::TypeParams
        | Extra::StringPart
        | Extra::BytesPart
        | Extra::IpyEscapeCommand => Field::None,
    }
}
