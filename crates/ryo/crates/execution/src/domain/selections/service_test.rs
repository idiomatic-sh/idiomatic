//! The `selections` service's cases: the positions a select of the tree keeps and the candidates it visits to keep them.

use super::service::select_tree_with_visits;
use crate::domain::operating::errors::OperatingError;
use crate::domain::positions::service_test::{SmallTree, rows_over, small_tree};
use crate::domain::positions::types::Slots;
use crate::domain::shared_walk::service_test::with_context;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
};

/// What a row of the cases asks of a select of the kind a name says.
#[derive(Clone, Copy)]
enum Asked {
    /// How many positions it keeps.
    Kept(&'static str),
    /// How many candidates it visits.
    Visited(&'static str),
}

/// What a row of the cases asks.
type Input = Asked;
/// A count.
type Value = usize;
/// What the select refuses with.
type Failure = OperatingError;

fn subject(input: Input) -> Result<Value, Failure> {
    let (Asked::Kept(kind) | Asked::Visited(kind)) = input;
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let tree = rows_over(&elements);
    let comparisons = [PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(1)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(kind.into()))),
        origin: ComparisonOrigin::Unrecorded,
    }];
    let mut visited = 0;
    let mut kept = Slots::default();
    with_context(&mut symbols, |context| {
        select_tree_with_visits(&tree, &comparisons, context, &mut |_| visited += 1, &mut kept)
    })?;
    Ok(match input {
        Asked::Kept(_) => kept.0.len(),
        Asked::Visited(_) => visited,
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_select_keeps_the_elements_of_the_kind_it_names"),
        word: Word::Succeeds,
        input: || Asked::Kept("word"),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_select_visits_only_the_candidates_of_the_kind_it_names"),
        word: Word::Succeeds,
        input: || Asked::Visited("word"),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_select_of_a_kind_no_element_has_visits_nothing"),
        word: Word::Succeeds,
        input: || Asked::Visited("absent-kind"),
        expect: Expect::Value(0),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
