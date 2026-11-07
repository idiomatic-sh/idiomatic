//! The walk's cases: the library's tree of a file, written out by hand, and the adapter's tree held to it, as it is and with a piece
//! moved out of the node it belongs in.

use std::borrow::Cow;

use super::walked;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::{kind_name, kind_of};
use crate::outbound::syntax::adapters::toml_parser::adapter::SyntaxTomlParserAdapter;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::SourceText;
use ryo::languages::contract::{Leafness, Walked};
use ryo::languages::{self, ByteSlice, Elements, Position, Symbols};

/// What is read of a file.
#[derive(Clone, Copy, Debug)]
enum Reading {
    /// The walk of the library's tree, written out.
    Walk,
    /// Whether the adapter's tree is the walk's.
    Adapter,
    /// Whether the adapter's tree is the walk's, once the last piece of its deepest first nested node is moved out of it.
    Hoisted,
}

/// TOML text, and what is read of it.
type Input = (&'static str, Reading);
/// The tree written out, or whether the adapter's is the library's shape.
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_dotted_key_value_holds_its_key_and_its_nested_value"),
        word: Word::Succeeds,
        input: || ("a . b = [1, {c = 2}]\n", Reading::Walk),
        expect: Expect::Value(Cow::Borrowed(
            "toml 0..21 [keyval 0..20 [key 0..5 [unquoted-key 0..1 ws 1..2 dot-sep 2..3 ws 3..4 unquoted-key 4..5] ws 5..6 keyval-sep 6..7 ws 7..8 array 8..20 [array-open 8..9 integer 9..10 array-sep 10..11 ws 11..12 inline-table 12..19 [inline-table-open 12..13 keyval 13..18 [key 13..14 [unquoted-key 13..14] ws 14..15 keyval-sep 15..16 ws 16..17 integer 17..18] inline-table-close 18..19] array-close 19..20]] newline 20..21]",
        )),
    },
    Case {
        name: CaseName("a_table_holds_its_header_and_its_entries"),
        word: Word::Succeeds,
        input: || ("[t]\nk = 1 # c\n", Reading::Walk),
        expect: Expect::Value(Cow::Borrowed(
            "toml 0..14 [std-table 0..14 [std-table-open 0..1 key 1..2 [unquoted-key 1..2] std-table-close 2..3 newline 3..4 keyval 4..9 [key 4..5 [unquoted-key 4..5] ws 5..6 keyval-sep 6..7 ws 7..8 integer 8..9] ws 9..10 comment 10..13 newline 13..14]]",
        )),
    },
    Case {
        name: CaseName("the_adapter_has_the_shape_of_a_nested_value"),
        word: Word::Succeeds,
        input: || ("a . b = [1, {c = 2}]\n", Reading::Adapter),
        expect: Expect::Value(Cow::Borrowed("the library's shape")),
    },
    Case {
        name: CaseName("the_adapter_has_the_shape_of_a_table"),
        word: Word::Succeeds,
        input: || ("[t]\nk = 1 # c\n", Reading::Adapter),
        expect: Expect::Value(Cow::Borrowed("the library's shape")),
    },
    Case {
        name: CaseName("a_piece_moved_out_of_its_inline_table_is_caught"),
        word: Word::Succeeds,
        input: || ("a . b = [1, {c = 2}]\n", Reading::Hoisted),
        expect: Expect::Value(Cow::Borrowed("not the library's shape")),
    },
    Case {
        name: CaseName("a_piece_moved_out_of_its_table_is_caught"),
        word: Word::Succeeds,
        input: || ("[t]\nk = 1 # c\n", Reading::Hoisted),
        expect: Expect::Value(Cow::Borrowed("not the library's shape")),
    },
];

/// The piece at `at` of the adapter's table, with everything under it.
fn table_piece(elements: &Elements, at: Position) -> Walked {
    Walked {
        kind: languages::kind_at(elements, at),
        leaf: if languages::flags_at(elements, at).0 & languages::TOKEN.0 == 0 {
            Leafness::Node
        } else {
            Leafness::Token
        },
        start: languages::start_at(elements, at),
        end: languages::end_at(elements, at),
        children: languages::children(elements, at)
            .map(|each| table_piece(elements, each))
            .collect(),
    }
}

/// The adapter's tree of `text`.
fn adapter_tree(text: &str) -> Walked {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    SyntaxTomlParserAdapter.read(
        SourceText(text.as_bytes()),
        &mut languages::table_builder(&mut elements, &mut symbols, ByteSlice(text.as_bytes())),
    );
    table_piece(&elements, Position(0))
}

/// Moves the last piece of the deepest node holding a node out of it, to just after it: a node closed one piece early.
fn hoist(node: &mut Walked) -> bool {
    for at in 0..node.children.len() {
        let Some(child) = node.children.get_mut(at) else {
            continue;
        };
        if child.leaf == Leafness::Token {
            continue;
        }
        if hoist(child) {
            return true;
        }
        if let Some(last) = child.children.pop() {
            child.end = last.start;
            node.children.insert(at + 1, last);
            return true;
        }
    }
    false
}

/// The tree written out: each piece as its kind and bytes, the pieces under a node in brackets.
fn shown(piece: &Walked) -> String {
    let name = kind_name(kind_of(piece.kind)).0;
    let mut text = format!("{name} {}..{}", piece.start.0, piece.end.0);
    if !piece.children.is_empty() {
        let under: Vec<String> = piece.children.iter().map(shown).collect();
        text.push_str(" [");
        text.push_str(&under.join(" "));
        text.push(']');
    }
    text
}

fn subject((text, reading): Input) -> Result<Value, Failure> {
    let library = walked(text);
    let ours = match reading {
        Reading::Walk => return Ok(Cow::Owned(shown(&library))),
        Reading::Adapter => adapter_tree(text),
        Reading::Hoisted => {
            let mut tree = adapter_tree(text);
            // A tree with nothing to hoist cannot be the weaker one the row asks for.
            if !hoist(&mut tree) {
                return Err(());
            }
            tree
        }
    };
    Ok(Cow::Borrowed(if ours == library {
        "the library's shape"
    } else {
        "not the library's shape"
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
