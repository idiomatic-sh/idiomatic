//! The `text_gate` service's cases, on comparisons alone: which of them the text of an element decides, which are left, and which
//! columns they read.

use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::FileId;

use super::service::{needed_of, needed_of_rest, text_gate, undecided};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_cells::types::{ColumnPlaces, TreeForm, TreeRows};
use crate::domain::table_scratch::types::{ComparisonNumbers, KindsBy};
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, LiteralText, Operand, ParameterId, QueryLiteral, Test,
};

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Gates the comparisons of a tree of elements: the second and third ask of the text something fixed.
    GatesTextComparisons,
    /// Gates the comparisons of a tree of edges: no text is there to decide them.
    GatesNothingOfEdges,
    /// Leaves what neither the gate nor the kind sets decided, when the sets decide the kind comparisons.
    LeavesTheRestWithSets,
    /// Leaves what the gate has not decided, when nothing decides the kind comparisons.
    LeavesTheRestWithoutSets,
    /// Names the columns every comparison reads, each once.
    NeedsEveryColumnOnce,
    /// Names the columns of the comparisons numbered, each once.
    NeedsTheColumnsOfTheRest,
}

/// What a row of the cases does.
type Input = Scenario;
/// The comparisons or the columns that come out, written as bits: place `n` is the bit `1 << n`.
type Value = usize;
type Failure = ();

fn named(column: u32, test: Test, literal: &str) -> Comparison {
    Comparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(literal.into()))),
        origin: None,
    }
}

/// Four comparisons: a number of column 0, the kind named `word`, the text `a`, and the text `b` written the other way round.
fn comparisons() -> Vec<Comparison> {
    vec![
        Comparison {
            test: Test::GreaterThan,
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Literal(QueryLiteral::Number(Integer(1))),
            origin: None,
        },
        named(1, Test::Equal, "word"),
        named(2, Test::Equal, "a"),
        Comparison {
            test: Test::Equal,
            left: Operand::Literal(QueryLiteral::Text(LiteralText("b".into()))),
            right: Operand::Column(ColumnIndex(2)),
            origin: None,
        },
    ]
}

fn bits(places: &[usize]) -> Value {
    places.iter().fold(0, |all, place| all | (1 << place))
}

fn tree(elements: &Elements, form: TreeForm) -> TreeRows<'_> {
    TreeRows {
        elements,
        file: FileId(0),
        form,
    }
}

fn gated(form: TreeForm) -> Value {
    let elements = Elements::default();
    let mut gate = ComparisonNumbers(vec![9]);
    text_gate(&tree(&elements, form), &comparisons(), &mut gate);
    bits(&gate.0)
}

fn left_after(gate: &[usize], kinds_by: KindsBy) -> Value {
    let mut rest = ComparisonNumbers(vec![9]);
    undecided(
        &comparisons(),
        &ComparisonNumbers(gate.to_vec()),
        kinds_by,
        &mut rest,
    );
    bits(&rest.0)
}

fn columns(which: Option<&[usize]>) -> Value {
    let mut comparisons = comparisons();
    comparisons.push(Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Parameter(ParameterId(0)),
        origin: None,
    });
    let mut needed = ColumnPlaces(vec![9]);
    match which {
        Some(which) => needed_of_rest(
            &comparisons,
            &ComparisonNumbers(which.to_vec()),
            &mut needed,
        ),
        None => needed_of(&comparisons, &mut needed),
    }
    bits(&needed.0)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("comparisons_of_the_text_are_gated"),
        word: Word::Succeeds,
        input: || Scenario::GatesTextComparisons,
        expect: Expect::Value(0b1100),
    },
    Case {
        name: CaseName("a_tree_of_edges_has_no_text_to_gate"),
        word: Word::Succeeds,
        input: || Scenario::GatesNothingOfEdges,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("the_kind_sets_and_the_gate_decide_their_comparisons"),
        word: Word::Succeeds,
        input: || Scenario::LeavesTheRestWithSets,
        expect: Expect::Value(0b0001),
    },
    Case {
        name: CaseName("without_kind_sets_only_the_gate_decides"),
        word: Word::Succeeds,
        input: || Scenario::LeavesTheRestWithoutSets,
        expect: Expect::Value(0b0011),
    },
    Case {
        name: CaseName("every_column_is_needed_once"),
        word: Word::Succeeds,
        input: || Scenario::NeedsEveryColumnOnce,
        expect: Expect::Value(0b111),
    },
    Case {
        name: CaseName("only_the_columns_of_the_rest_are_needed"),
        word: Word::Succeeds,
        input: || Scenario::NeedsTheColumnsOfTheRest,
        expect: Expect::Value(0b101),
    },
];

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::GatesTextComparisons => gated(TreeForm::Elements),
        Scenario::GatesNothingOfEdges => gated(TreeForm::Edges),
        Scenario::LeavesTheRestWithSets => left_after(&[2, 3], KindsBy::Sets),
        Scenario::LeavesTheRestWithoutSets => left_after(&[2, 3], KindsBy::Nothing),
        Scenario::NeedsEveryColumnOnce => columns(None),
        Scenario::NeedsTheColumnsOfTheRest => columns(Some(&[0, 4])),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
