//! The `tree_cells` service's cases: a tree's rows read from its elements, one cell at a time.

use crate::domain::tree_cells::service::{edge_of, element_of, native_form, tree_value};
use crate::domain::tree_cells::types::{TreeForm, TreeRows};
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{FileId, KindBits, Offset, Position, Relation};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::borrow::Cow;

/// A scenario: what is read of the tree.
type Input = fn() -> Result<Value, Failure>;
/// What a scenario answers: the debug text of what was read.
type Value = Cow<'static, str>;
/// Why a scenario is refused: never.
type Failure = ();

/// The tree of `ab`: a `file` node over the tokens `a` (a `word`) and `b` (an `other`), in file 3.
fn tree() -> Elements {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"ab"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    token(&mut builder, KindBits(3), Offset(1), Offset(2));
    leave_node(&mut builder, Offset(2));
    for (bits, name) in [(1, "file"), (2, "word"), (3, "other")] {
        let mut names = completion(&mut elements, &mut symbols, ByteSlice(b"ab"), FileId(3));
        name_kind(&mut names, KindBits(bits), NameText(name));
    }
    elements
}

/// The rows of the tree in `form`.
fn rows(elements: &Elements, form: TreeForm) -> TreeRows<'_> {
    TreeRows { form, elements, file: FileId(3) }
}

/// The element row of the second token.
fn an_element_row_is_the_node_kind_text_file_and_span() -> Value {
    let elements = tree();
    Cow::Owned(format!("{:?}", element_of(&rows(&elements, TreeForm::Elements), Position(2))))
}

/// The edge to the second token.
fn an_edge_row_is_the_parent_child_place_role_and_file() -> Value {
    let elements = tree();
    Cow::Owned(format!("{:?}", edge_of(&rows(&elements, TreeForm::Edges), Position(2))))
}

/// A column past a tree row's five.
fn a_column_past_the_row_is_the_lowered_flag() -> Value {
    let elements = tree();
    Cow::Owned(format!("{:?}", tree_value(&rows(&elements, TreeForm::Elements), Position(0), ColumnIndex(5))))
}

/// The forms of the relations the tree answers, and of one it does not.
fn only_the_tree_relations_have_a_form() -> Value {
    Cow::Owned(format!(
        "{:?}",
        [Relation::SyntaxTree, Relation::SyntaxChild, Relation::Function].map(native_form)
    ))
}

/// The cases, in order.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_element_row_is_the_node_kind_text_file_and_span"),
        word: Word::Succeeds,
        input: || || Ok(an_element_row_is_the_node_kind_text_file_and_span()),
        expect: Expect::Value(Cow::Borrowed("[Node(NodeId(12884901890)), Symbol(Symbol(4)), Symbol(Symbol(1)), File(FileId(3)), Span(Span { start: Offset(1), end: Offset(2) })]")),
    },
    Case {
        name: CaseName("an_edge_row_is_the_parent_child_place_role_and_file"),
        word: Word::Succeeds,
        input: || || Ok(an_edge_row_is_the_parent_child_place_role_and_file()),
        expect: Expect::Value(Cow::Borrowed("[Node(NodeId(12884901888)), Node(NodeId(12884901890)), Number(Integer(1)), Symbol(Symbol(4294967295)), File(FileId(3))]")),
    },
    Case {
        name: CaseName("a_column_past_the_row_is_the_lowered_flag"),
        word: Word::Succeeds,
        input: || || Ok(a_column_past_the_row_is_the_lowered_flag()),
        expect: Expect::Value(Cow::Borrowed("Flag(Lowered)")),
    },
    Case {
        name: CaseName("only_the_tree_relations_have_a_form"),
        word: Word::Succeeds,
        input: || || Ok(only_the_tree_relations_have_a_form()),
        expect: Expect::Value(Cow::Borrowed("[Some(Elements), Some(Edges), None]")),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
