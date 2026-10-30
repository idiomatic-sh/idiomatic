//! The library's own tree of a file, in the language's kinds: what the shape rail of the conformance holds our tree to. It parses the
//! text with the library and gathers each node and token the library gives, at its own range; it reads nothing of the adapter, its
//! schemas or the element table, so a tree the adapter writes wrongly is a difference and not the same mistake twice. The library's
//! tree has the objects, the members and the arrays; every token is put in the deepest of those whose bytes hold it, and the bytes
//! between two things of a node are white space and then text no token spans.

use jsonc_parser::ast::Value;
use jsonc_parser::common::Range;
use jsonc_parser::tokens::Token as Scanned;
use jsonc_parser::{
    CollectOptions, CommentCollectionStrategy, ParseOptions, Scanner, ScannerOptions, parse_to_ast,
};
use ryo::languages::Offset;
use ryo::languages::contract::{Leafness, Walked};

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// A construct of the library's tree: what it is, the bytes it spans and the construct it is in (none for the file).
struct Made {
    /// What construct it is.
    kind: Kind,
    /// Where its bytes start.
    start: usize,
    /// Where its bytes end.
    end: usize,
    /// The construct it is in, by its place among those made; none for the file.
    parent: Option<usize>,
}

/// A token of the library: what it is and the bytes it spans.
struct Seen {
    /// What token it is.
    kind: Kind,
    /// Where its bytes start.
    start: usize,
    /// Where its bytes end.
    end: usize,
}

/// The kind of the language a token of the library is.
fn kind_of_token(token: &Scanned<'_>) -> Kind {
    match token {
        Scanned::OpenBrace => Kind::Token(Token::BeginObject),
        Scanned::CloseBrace => Kind::Token(Token::EndObject),
        Scanned::OpenBracket => Kind::Token(Token::BeginArray),
        Scanned::CloseBracket => Kind::Token(Token::EndArray),
        Scanned::Comma => Kind::Token(Token::ValueSeparator),
        Scanned::Colon => Kind::Token(Token::NameSeparator),
        Scanned::String(_) => Kind::Token(Token::String),
        Scanned::Number(_) => Kind::Token(Token::Number),
        Scanned::Boolean(true) => Kind::Token(Token::True),
        Scanned::Boolean(false) => Kind::Token(Token::False),
        Scanned::Null => Kind::Token(Token::Null),
        Scanned::CommentLine(_) => Kind::Extra(Extra::LineComment),
        Scanned::CommentBlock(_) => Kind::Extra(Extra::BlockComment),
        Scanned::Word(_) => Kind::Extra(Extra::Error),
    }
}

/// Adds a construct to `out` and answers where it is there.
fn add(out: &mut Vec<Made>, kind: Node, range: Range, parent: Option<usize>) -> usize {
    out.push(Made {
        kind: Kind::Node(kind),
        start: range.start,
        end: range.end,
        parent,
    });
    out.len() - 1
}

/// The objects, members and arrays of `value` in the order they open, each with the one it is in.
fn made(value: &Value<'_>, parent: Option<usize>, out: &mut Vec<Made>) {
    match value {
        Value::Object(object) => {
            let here = add(out, Node::Object, object.range, parent);
            for property in &object.properties {
                let member = add(out, Node::Member, property.range, Some(here));
                made(&property.value, Some(member), out);
            }
        }
        Value::Array(array) => {
            let here = add(out, Node::Array, array.range, parent);
            for element in &array.elements {
                made(element, Some(here), out);
            }
        }
        Value::StringLit(_)
        | Value::NumberLit(_)
        | Value::BooleanLit(_)
        | Value::NullKeyword(_) => {}
    }
}

/// The walked tokens for the bytes between `from` and `to`: white space, then text the library could not read.
fn gap(text: &str, from: usize, to: usize, out: &mut Vec<Walked>) {
    if from >= to {
        return;
    }
    let blanks = text.get(from..to).map_or(0, |bytes| {
        bytes.len() - bytes.trim_start_matches([' ', '\t', '\n', '\r']).len()
    });
    let token = |kind: Kind, start: usize, end: usize| Walked {
        kind: kind_bits(kind),
        leaf: Leafness::Token,
        start: Offset(u32::try_from(start).unwrap_or(u32::MAX)),
        end: Offset(u32::try_from(end).unwrap_or(u32::MAX)),
        children: Vec::new(),
    };
    if blanks > 0 {
        out.push(token(Kind::Token(Token::Ws), from, from + blanks));
    }
    if from + blanks < to {
        out.push(token(Kind::Extra(Extra::Error), from + blanks, to));
    }
}

/// The walked piece for construct `at` of `all`, with the tokens and constructs in it.
fn piece(text: &str, all: &[Made], seen: &[Seen], at: usize) -> Walked {
    let Some(this) = all.get(at) else {
        return Walked {
            kind: kind_bits(Kind::Node(Node::JsonText)),
            leaf: Leafness::Node,
            start: Offset(0),
            end: Offset(0),
            children: Vec::new(),
        };
    };
    let (children, _) = within(text, all, seen, Some(at), this.start);
    Walked {
        kind: kind_bits(this.kind),
        leaf: Leafness::Node,
        start: Offset(u32::try_from(this.start).unwrap_or(u32::MAX)),
        end: Offset(u32::try_from(this.end).unwrap_or(u32::MAX)),
        children,
    }
}

/// The deepest construct whose bytes hold `start..end`.
fn home(all: &[Made], start: usize, end: usize) -> Option<usize> {
    all.iter()
        .rposition(|each| each.start <= start && end <= each.end)
}

/// What is directly in construct `parent` (the file for none): its constructs and tokens in the order of the file, the bytes between
/// them filled, from `from` on; and where the last of them ends.
fn within(
    text: &str,
    all: &[Made],
    seen: &[Seen],
    parent: Option<usize>,
    from: usize,
) -> (Vec<Walked>, usize) {
    let mut things: Vec<(usize, usize, Option<usize>, Option<&Seen>)> = Vec::new();
    for (at, each) in all.iter().enumerate() {
        if each.parent == parent {
            things.push((each.start, each.end, Some(at), None));
        }
    }
    for each in seen {
        if home(all, each.start, each.end) == parent {
            things.push((each.start, each.end, None, Some(each)));
        }
    }
    things.sort_by_key(|(start, _, _, _)| *start);
    let (mut out, mut cursor) = (Vec::new(), from);
    for (start, end, construct, token) in things {
        gap(text, cursor, start, &mut out);
        match (construct, token) {
            (Some(at), _) => out.push(piece(text, all, seen, at)),
            (None, Some(each)) => out.push(Walked {
                kind: kind_bits(each.kind),
                leaf: Leafness::Token,
                start: Offset(u32::try_from(each.start).unwrap_or(u32::MAX)),
                end: Offset(u32::try_from(each.end).unwrap_or(u32::MAX)),
                children: Vec::new(),
            }),
            (None, None) => {}
        }
        cursor = end;
    }
    (out, cursor)
}

/// The library's tree of `text`.
#[must_use]
pub(crate) fn walked(text: &str) -> Walked {
    let length = text.len();
    let collect = CollectOptions {
        comments: CommentCollectionStrategy::AsTokens,
        tokens: true,
    };
    let grammar = ParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: false,
        allow_trailing_commas: true,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
        allow_bare_decimal_point_numbers: false,
        allow_non_finite_numbers: false,
        allow_extended_string_escapes: false,
    };
    let mut all = Vec::new();
    let mut seen = Vec::new();
    if let Ok(parsed) = parse_to_ast(text, &collect, &grammar) {
        if let Some(value) = &parsed.value {
            made(value, None, &mut all);
        }
        for token in parsed.tokens.iter().flatten() {
            seen.push(Seen {
                kind: kind_of_token(&token.token),
                start: token.range.start,
                end: token.range.end,
            });
        }
    } else {
        let mut scanner = Scanner::new(
            text,
            &ScannerOptions {
                allow_single_quoted_strings: false,
                allow_hexadecimal_numbers: false,
                allow_unary_plus_numbers: false,
                allow_bare_decimal_point_numbers: false,
                allow_non_finite_numbers: false,
                allow_extended_string_escapes: false,
            },
        );
        while let Ok(Some(token)) = scanner.scan() {
            seen.push(Seen {
                kind: kind_of_token(&token),
                start: scanner.token_start(),
                end: scanner.token_end(),
            });
        }
    }
    let (mut children, cursor) = within(text, &all, &seen, None, 0);
    gap(text, cursor, length, &mut children);
    Walked {
        kind: kind_bits(Kind::Node(Node::JsonText)),
        leaf: Leafness::Node,
        start: Offset(0),
        end: Offset(u32::try_from(length).unwrap_or(u32::MAX)),
        children,
    }
}
