//! The `fusing` service's cases, on plans written by hand.

use ryo_relations::domain::relations::types::Relation;

use super::service::fuse_selects;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Input as Reads;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// What a row asks of the service.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// A filter of the rows another filter kept is one filter of the relation.
    TwoFiltersAreOne,
    /// A join with the filtered elements of the tree on their ids reads them where the ids say.
    AJoinOnIdsReadsTheFilteredElementsInPlace,
    /// A join with the edges of the tree, viewed as they were once named, by their child or by their parent.
    AJoinOnTheEdgesReadsThemInPlace,
    /// A join on the place of the edge among its parent's is left as it is.
    AJoinOnTheOrdinalOfTheEdgesIsLeft,
    /// An axis answering among the filtered elements reads them where it reaches them.
    AnAxisAmongFilteredElementsReadsThemInPlace,
    /// A join on the stretch the filtered elements cover is left as it is: the tree is walked only from a node.
    ASpanKeyedProducerSelectStaysAStep,
    /// A join on the ids of elements filtered by a pattern of their text is left as it is: the walk decides only what cannot fail.
    AFallibleProducerSelectStaysAStep,
    /// A join on another column of the filtered elements is left as it is.
    AJoinOnAnotherColumnIsLeft,
    /// A filter of a step something else reads too is left to be read by the other.
    FiltersAreFusedAlongsideTheirReaders,
}

/// A number the answer is.
type Value = usize;
type Failure = ();
/// What a row asks of the service.
type Input = Scenario;

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        written: Default::default(),
        operation,
        scope: StepScope::CrossFile,
    }
}

fn two_filters_are_one() -> usize {
    let (kept_first, test) = (
        vec![column(0), column(1), column(4)],
        |left: u32, text: &str| PlanComparison {
            test: Test::Equal,
            left: Operand::Column(column(left)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        },
    );
    let steps = vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::SyntaxTree),
            comparisons: vec![test(2, "x")],
            kept: kept_first,
        }),
        step(Operation::Select {
            from: Reads::Step(StepId(0)),
            comparisons: vec![test(2, "y")],
            kept: vec![column(1), column(0)],
        }),
    ];
    let fused = fuse_selects(steps);
    usize::from(
        matches!(&fused[1].operation, Operation::Select { from: Reads::Relation(Relation::SyntaxTree), comparisons, kept } if comparisons == &[test(2, "x"), test(4, "y")] && kept == &[column(1), column(0)]),
    )
}

fn a_join_on_ids_reads_the_filtered_elements_in_place(scenario: Scenario) -> usize {
    let on = u32::from(scenario != Scenario::AJoinOnIdsReadsTheFilteredElementsInPlace);
    let keep = PlanComparison {
        test: Test::NotEqual,
        left: Operand::Column(column(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(Box::from("")))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let steps = vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::SyntaxTree),
            comparisons: vec![keep.clone()],
            kept: vec![column(0), column(2)],
        }),
        step(Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        step(Operation::Join {
            left: Reads::Step(StepId(1)),
            right: Reads::Step(StepId(0)),
            left_on: vec![column(0)],
            right_on: vec![column(on)],
            output: Vec::new(),
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        }),
    ];
    let fused = fuse_selects(steps);
    let read_in_place = matches!(&fused[2].operation, Operation::Join { right: Reads::Relation(Relation::SyntaxTree), filter, view, .. } if filter == &[keep] && view == &[column(0), column(2)]);
    usize::from(read_in_place == (on == 0))
}

/// A join of the functions with elements filtered by `comparisons` and viewed as their id and span, on the view's column `on`:
/// whether the join still reads the filter as a step of its own.
fn a_producer_select_stays_a_step(comparisons: Vec<PlanComparison>, on: u32) -> usize {
    let steps = vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::SyntaxTree),
            comparisons,
            kept: vec![column(0), column(4)],
        }),
        step(Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        step(Operation::Join {
            left: Reads::Step(StepId(1)),
            right: Reads::Step(StepId(0)),
            left_on: vec![column(0)],
            right_on: vec![column(on)],
            output: Vec::new(),
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        }),
    ];
    let fused = fuse_selects(steps.clone());
    usize::from(fused == steps)
}

/// The element's column `at` compared with the written text `text` by `test`.
fn written(test: Test, at: u32, text: &str) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(column(at)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}

fn a_join_on_the_edges_reads_them_in_place(scenario: Scenario) -> usize {
    // The edges as a rule once named them: the parent, the ordinal, the child, the file.
    let on = if scenario == Scenario::AJoinOnTheEdgesReadsThemInPlace {
        2
    } else {
        1
    };
    let steps = vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::SyntaxChild),
            comparisons: Vec::new(),
            kept: vec![column(0), column(2), column(1), column(3)],
        }),
        step(Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        step(Operation::Join {
            left: Reads::Step(StepId(1)),
            right: Reads::Step(StepId(0)),
            left_on: vec![column(0)],
            right_on: vec![column(on)],
            output: Vec::new(),
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        }),
    ];
    let fused = fuse_selects(steps);
    let read_in_place = matches!(&fused[2].operation, Operation::Join { right: Reads::Relation(Relation::SyntaxChild), view, .. } if view == &[column(0), column(2), column(1), column(3)]);
    usize::from(read_in_place == (on == 2))
}

fn an_axis_among_filtered_elements_reads_them_in_place() -> usize {
    let keep = PlanComparison {
        test: Test::Equal,
        left: Operand::Column(column(2)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(Box::from("")))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let steps = vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::SyntaxTree),
            comparisons: vec![keep.clone()],
            kept: vec![column(0), column(4)],
        }),
        step(Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        step(Operation::Axis {
            from: Reads::Step(StepId(1)),
            edges: Reads::Relation(Relation::SyntaxChild),
            domain: Reads::Step(StepId(0)),
            direction: Direction::Descendant,
            id: column(0),
            parent: column(0),
            child: column(1),
            domain_id: column(0),
            domain_filter: Vec::new(),
            domain_view: Vec::new(),
            output: Vec::new(),
        }),
    ];
    let fused = fuse_selects(steps);
    usize::from(
        matches!(&fused[2].operation, Operation::Axis { domain: Reads::Relation(Relation::SyntaxTree), domain_filter, domain_view, .. } if domain_filter == &[keep] && domain_view == &[column(0), column(4)]),
    )
}

fn filters_are_fused_alongside_their_readers() -> usize {
    let steps = vec![
        step(Operation::Select {
            from: Reads::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept: Vec::new(),
        }),
        step(Operation::Select {
            from: Reads::Step(StepId(0)),
            comparisons: Vec::new(),
            kept: vec![column(1)],
        }),
    ];
    let fused = fuse_selects(steps.clone());
    usize::from(
        fused.len() == 2
            && matches!(&fused[1].operation, Operation::Select { from: Reads::Relation(Relation::Function), kept, .. } if kept == &[column(1)])
            && fused[0].operation == steps[0].operation,
    )
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::TwoFiltersAreOne => two_filters_are_one(),
        Scenario::AJoinOnIdsReadsTheFilteredElementsInPlace
        | Scenario::AJoinOnAnotherColumnIsLeft => {
            a_join_on_ids_reads_the_filtered_elements_in_place(scenario)
        }
        Scenario::ASpanKeyedProducerSelectStaysAStep => {
            a_producer_select_stays_a_step(vec![written(Test::Equal, 2, "")], 1)
        }
        Scenario::AFallibleProducerSelectStaysAStep => a_producer_select_stays_a_step(
            vec![
                written(Test::Equal, 1, "ident"),
                written(Test::Matches, 2, "^(?:r#)?feature$"),
            ],
            0,
        ),
        Scenario::AJoinOnTheEdgesReadsThemInPlace | Scenario::AJoinOnTheOrdinalOfTheEdgesIsLeft => {
            a_join_on_the_edges_reads_them_in_place(scenario)
        }
        Scenario::AnAxisAmongFilteredElementsReadsThemInPlace => {
            an_axis_among_filtered_elements_reads_them_in_place()
        }
        Scenario::FiltersAreFusedAlongsideTheirReaders => {
            filters_are_fused_alongside_their_readers()
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_filter_of_a_filtered_relation_is_one_filter_of_the_relation"),
        word: Word::Succeeds,
        input: || Scenario::TwoFiltersAreOne,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_producer_select_of_a_node_key_rides_the_walk"),
        word: Word::Succeeds,
        input: || Scenario::AJoinOnIdsReadsTheFilteredElementsInPlace,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_join_on_the_child_of_viewed_edges_reads_them_where_the_tree_says"),
        word: Word::Succeeds,
        input: || Scenario::AJoinOnTheEdgesReadsThemInPlace,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_join_on_the_ordinal_of_edges_is_left_alone"),
        word: Word::Succeeds,
        input: || Scenario::AJoinOnTheOrdinalOfTheEdgesIsLeft,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_axis_among_filtered_elements_reads_them_where_it_reaches_them"),
        word: Word::Succeeds,
        input: || Scenario::AnAxisAmongFilteredElementsReadsThemInPlace,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_span_keyed_producer_select_stays_a_step"),
        word: Word::Succeeds,
        input: || Scenario::ASpanKeyedProducerSelectStaysAStep,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_fallible_producer_select_stays_a_step"),
        word: Word::Succeeds,
        input: || Scenario::AFallibleProducerSelectStaysAStep,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_join_on_another_column_of_filtered_elements_is_left_alone"),
        word: Word::Succeeds,
        input: || Scenario::AJoinOnAnotherColumnIsLeft,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_filter_is_fused_into_the_step_it_reads_and_that_step_stays"),
        word: Word::Succeeds,
        input: || Scenario::FiltersAreFusedAlongsideTheirReaders,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
