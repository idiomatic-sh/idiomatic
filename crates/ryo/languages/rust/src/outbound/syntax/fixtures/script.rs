//! A syntax tree written by hand in the language's own kinds: its nodes and tokens, and the text they spell. The tests of the domain
//! build the tree they read from one, so they depend on no parser.

use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Offset};

/// A piece of a tree written by hand.
#[derive(Clone, Debug)]
pub(crate) enum Piece {
    /// A node, with the pieces under it.
    RustNode(Kind, Vec<Piece>),
    /// A token with its text.
    Token(Kind, &'static str),
    /// Something the parser said it could not read, at this point of the text.
    Error(&'static str),
}

/// A node of this kind holding these pieces.
#[must_use]
pub(crate) fn node(kind: Kind, children: Vec<Piece>) -> Piece {
    Piece::Node(kind, children)
}

/// A token of this kind spelling this text.
#[must_use]
pub(crate) const fn token(kind: Kind, text: &'static str) -> Piece {
    Piece::Token(kind, text)
}

/// A token of white space.
#[must_use]
pub(crate) const fn space(text: &'static str) -> Piece {
    Piece::Token(Kind::Token(Token::Whitespace), text)
}

/// `piece`, a node, with the parser's complaint added after what it holds.
#[must_use]
pub(crate) fn with_error(piece: Piece, message: &'static str) -> Piece {
    match piece {
        Piece::Node(kind, mut children) => {
            children.push(Piece::Error(message));
            Piece::Node(kind, children)
        }
        kept @ (Piece::Token(..) | Piece::Error(_)) => kept,
    }
}

/// The text the tokens of `piece` spell, in order.
#[must_use]
pub(crate) fn text_of(piece: &Piece) -> String {
    match piece {
        Piece::Node(_, children) => children.iter().map(text_of).collect(),
        Piece::Token(_, text) => (*text).to_owned(),
        Piece::Error(_) => String::new(),
    }
}

/// Writes the whole tree `piece` into `builder`, from the start of the file.
pub(super) fn replay(piece: &Piece, builder: &mut languages::TableBuilder<'_>) {
    replayed(piece, &mut Offset(0), builder);
}

/// Writes `piece` into `builder` from the byte `at`, and moves `at` on to the byte it ends at, where the next sibling starts.
fn replayed(piece: &Piece, at: &mut Offset, builder: &mut languages::TableBuilder<'_>) {
    match piece {
        Piece::Node(kind, children) => {
            languages::enter_node(builder, kind_bits(*kind), *at);
            for child in children {
                replayed(child, at, builder);
            }
            languages::leave_node(builder, *at);
        }
        Piece::Token(kind, text) => {
            let start = *at;
            let end = Offset(start.0 + u32::try_from(text.len()).unwrap_or(u32::MAX));
            let plain = text
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .filter(|between| !between.contains('\\'));
            match plain {
                Some(value) if *kind == Kind::Token(Token::String) => languages::decoded_token(
                    builder,
                    kind_bits(*kind),
                    start,
                    end,
                    languages::NameText(value),
                ),
                _ => languages::token(builder, kind_bits(*kind), start, end),
            }
            *at = end;
        }
        Piece::Error(message) => {
            languages::diagnose(builder, languages::ReportText(message), *at, *at);
        }
    }
}

/// A tree written in the language's kinds: `scripted_tree!(Fn { FnKw("fn"), Whitespace(" "), Name { Ident("f") } })` is a node of kind `Fn`
/// holding two tokens and a node. Each token spells its text.
#[macro_export]
#[doc(hidden)]
macro_rules! scripted_tree {
    ($kind:ident ( $text:expr )) => {
        $crate::outbound::syntax::fixtures::script::token(
            $crate::domain::syntax::types::Kind::Token($crate::domain::tokens::types::Token::$kind),
            $text,
        )
    };
    ($kind:ident { $($child:ident $body:tt),* $(,)? }) => {
        $crate::outbound::syntax::fixtures::script::node(
            $crate::domain::syntax::types::Kind::Node($crate::domain::nodes::types::RustNode::$kind),
            vec![$($crate::scripted_tree!($child $body)),*],
        )
    };
}

/// The table the fake makes of `piece` (it replays the piece, reading nothing of the text), the symbols its tokens are interned in,
/// and the text the tokens spell.
#[must_use]
pub(crate) fn table_of(piece: &Piece) -> (languages::Elements, languages::Symbols, String) {
    let text = text_of(piece);
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    replay(
        piece,
        &mut languages::table_builder(
            &mut elements,
            &mut symbols,
            languages::ByteSlice(text.as_bytes()),
        ),
    );
    (elements, symbols, text)
}
