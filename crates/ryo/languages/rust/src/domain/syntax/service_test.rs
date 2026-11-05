//! The `syntax` service's cases: the kinds, and what a question of a tree written by hand answers, in full.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::RustNode;
use crate::domain::syntax::types::Typeness;
use crate::domain::tokens::types::Token;
use ryo::languages::{self, KindBits, Position, Sample, Tree};

use super::service::{ancestors, child, is_type, kind, kind_bits, kind_of, kinds, node_after};
use super::types::Kind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::kinds::sample;
use crate::outbound::syntax::fixtures::script::table_of;
use crate::outbound::syntax::fixtures::trees::tree_of;

/// What a row asks.
#[derive(Clone, Copy)]
enum Question {
    /// Which kinds of the vocabulary are not the kind their number stands for.
    KindsLostByTheirNumber,
    /// What kind a number nothing stands for is.
    UnknownNumber,
    /// How many kinds are marked unread with no reason.
    UnreadWithoutReason,
    /// Whether the first element of a kind of a source has a child of a kind.
    Child(&'static str, Kind, Kind),
    /// Whether the node after the first token of a kind under the first element of a kind of a source is a type.
    TypeAfter(&'static str, Kind, Kind),
    /// The kinds above the first element of a kind of a source, nearest first.
    Above(&'static str, Kind),
    /// Whether the first element of a kind of a source is a type.
    IsType(&'static str, Kind),
}

/// What an answer is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    /// A count.
    Count(usize),
    /// A kind.
    Kind(Kind),
    /// Whether an element was found.
    Found(bool),
    /// Whether something is a type.
    Type(Typeness),
    /// The kinds above an element.
    Above(&'static [Kind]),
}
type Value = Answer;
type Input = Question;
type Failure = ();

fn with_tree<T>(source: &str, ask: impl FnOnce(&Tree<'_>) -> Option<T>) -> Result<T, Failure> {
    let (elements, _symbols, _) =
        table_of(&tree_of(source).expect("a tree is written for the source"));
    ask(&Tree {
        elements: &elements,
        source: languages::ByteSlice(source.as_bytes()),
        file: languages::FileId(0),
    })
    .ok_or(())
}

/// The first element of kind `wanted`.
fn first(tree: &Tree<'_>, wanted: Kind) -> Option<Position> {
    languages::positions(tree.elements).find(|at| kind(tree, *at) == wanted)
}

fn subject(question: Input) -> Result<Value, Failure> {
    match question {
        Question::KindsLostByTheirNumber => Ok(Answer::Count(
            kinds()
                .filter(|each| kind_of(kind_bits(*each)) != *each)
                .count(),
        )),
        Question::UnknownNumber => Ok(Answer::Kind(kind_of(KindBits(999)))),
        Question::UnreadWithoutReason => Ok(Answer::Count(
            kinds()
                .filter(|each| matches!(sample(*each), Sample::Unread(why) if why.is_empty()))
                .count(),
        )),
        Question::Child(source, parent, wanted) => with_tree(source, |tree| {
            Some(Answer::Found(
                child(tree, first(tree, parent)?, wanted).is_some(),
            ))
        }),
        Question::TypeAfter(source, parent, marker) => with_tree(source, |tree| {
            let annotation = node_after(tree, first(tree, parent)?, marker)?;
            Some(Answer::Type(is_type(tree, annotation)))
        }),
        Question::Above(source, wanted) => with_tree(source, |tree| {
            let above: Vec<Kind> = ancestors(tree, first(tree, wanted)?)
                .map(|each| kind(tree, each))
                .collect();
            Some(Answer::Above(Box::leak(above.into_boxed_slice())))
        }),
        Question::IsType(source, wanted) => with_tree(source, |tree| {
            Some(Answer::Type(is_type(tree, first(tree, wanted)?)))
        }),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_kind_survives_its_number"),
        word: Word::Succeeds,
        input: || Question::KindsLostByTheirNumber,
        expect: Expect::Value(Answer::Count(0)),
    },
    Case {
        name: CaseName("a_number_nothing_stands_for_is_other"),
        word: Word::Succeeds,
        input: || Question::UnknownNumber,
        expect: Expect::Value(Answer::Kind(Kind::Extra(Extra::Other))),
    },
    Case {
        name: CaseName("no_kind_is_marked_unread_without_a_reason"),
        word: Word::Succeeds,
        input: || Question::UnreadWithoutReason,
        expect: Expect::Value(Answer::Count(0)),
    },
    Case {
        name: CaseName("a_child_is_found_by_its_kind"),
        word: Word::Succeeds,
        input: || Question::Child("fn run() {}", Kind::Node(RustNode::Fn), Kind::Node(RustNode::Name)),
        expect: Expect::Value(Answer::Found(true)),
    },
    Case {
        name: CaseName("a_child_of_another_kind_is_not_found"),
        word: Word::Succeeds,
        input: || {
            Question::Child(
                "fn run() {}",
                Kind::Node(RustNode::Fn),
                Kind::Node(RustNode::RetType),
            )
        },
        expect: Expect::Value(Answer::Found(false)),
    },
    Case {
        name: CaseName("the_node_after_a_marker_is_found"),
        word: Word::Succeeds,
        input: || {
            Question::TypeAfter(
                "fn run(value: u8) {}",
                Kind::Node(RustNode::Param),
                Kind::Token(Token::Colon),
            )
        },
        expect: Expect::Value(Answer::Type(Typeness::Type)),
    },
    Case {
        name: CaseName("the_ancestors_run_up_to_the_root"),
        word: Word::Succeeds,
        input: || Question::Above("fn run() {}", Kind::Node(RustNode::Name)),
        expect: Expect::Value(Answer::Above(&[
            Kind::Node(RustNode::Fn),
            Kind::Node(RustNode::SourceFile),
        ])),
    },
    Case {
        name: CaseName("a_result_wrapper_is_not_a_type"),
        word: Word::Succeeds,
        input: || Question::IsType("fn run() -> u8 { 0 }", Kind::Node(RustNode::RetType)),
        expect: Expect::Value(Answer::Type(Typeness::NotType)),
    },
    Case {
        name: CaseName("what_a_result_wrapper_holds_is"),
        word: Word::Succeeds,
        input: || Question::IsType("fn run() -> u8 { 0 }", Kind::Node(RustNode::PathType)),
        expect: Expect::Value(Answer::Type(Typeness::Type)),
    },
    Case {
        name: CaseName("a_token_is_not_a_type"),
        word: Word::Succeeds,
        input: || Question::IsType("fn run() {}", Kind::Token(Token::Ident)),
        expect: Expect::Value(Answer::Type(Typeness::NotType)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
