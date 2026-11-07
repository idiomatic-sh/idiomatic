//! The walk's cases: the library's tree of a file, written out by hand, and the adapter's tree held to it, as it is and with a piece
//! moved out of the node it belongs in.

use ryo::languages::SourcePath;
use std::borrow::Cow;

use super::walked;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::{kind_name, kind_of};
use crate::outbound::syntax::adapters::pulldown_cmark::adapter::SyntaxPulldownCmarkAdapter;
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
    /// Whether the adapter's tree is the walk's, once the last piece of its first nested node is moved out of it.
    Hoisted,
}

/// Markdown text, and what is read of it.
type Input = (&'static str, Reading);
/// The tree written out, or whether the adapter's is the library's shape.
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_heading_holds_its_marker_its_emphasis_and_its_line_end"),
        word: Word::Succeeds,
        input: || ("# *a*\n", Reading::Walk),
        expect: Expect::Value(Cow::Borrowed(
            "root 0..6 [section 0..6 [heading 0..6 [markup 0..2 emphasis 2..5 [markup 2..3 text 3..4 markup 4..5] whitespace 5..6]]]",
        )),
    },
    Case {
        name: CaseName("a_nested_list_nests_its_items"),
        word: Word::Succeeds,
        input: || ("- a\n  - b\n", Reading::Walk),
        expect: Expect::Value(Cow::Borrowed(
            "root 0..10 [list 0..10 [list-item 0..10 [markup 0..2 text 2..3 whitespace 3..6 list 6..10 [list-item 6..10 [markup 6..8 text 8..9 whitespace 9..10]]]]]",
        )),
    },
    Case {
        name: CaseName("the_adapter_has_the_shape_of_a_heading_with_emphasis"),
        word: Word::Succeeds,
        input: || ("# *a*\n", Reading::Adapter),
        expect: Expect::Value(Cow::Borrowed("the library's shape")),
    },
    Case {
        name: CaseName("the_adapter_has_the_shape_of_a_nested_list"),
        word: Word::Succeeds,
        input: || ("- a\n  - b\n", Reading::Adapter),
        expect: Expect::Value(Cow::Borrowed("the library's shape")),
    },
    Case {
        name: CaseName("a_piece_moved_out_of_its_emphasis_is_caught"),
        word: Word::Succeeds,
        input: || ("# *a*\n", Reading::Hoisted),
        expect: Expect::Value(Cow::Borrowed("not the library's shape")),
    },
    Case {
        name: CaseName("a_piece_moved_out_of_its_nested_item_is_caught"),
        word: Word::Succeeds,
        input: || ("- a\n  - b\n", Reading::Hoisted),
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
    SyntaxPulldownCmarkAdapter.read(
        SourceText(text.as_bytes()),
        SourcePath("a.md"),
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
