// ignore: idiomatic.arch/file-length - one case table for the one presence fold: every case shares its plan fixtures and helpers, and the fold has no parts of its own to split along.
//! The presence fold: every operation's declaration, the failure term of each prepared contract, the column origins a
//! test is grouped by, and a region table total over the compiled program.
use super::service::{ALWAYS, for_regions, of_graph};
use super::types::{FormulaId, GraphPresence, PresenceFormula, PresenceTable};
use crate::domain::file_program::types::RequestGraph;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::service::{prepare_for, semantics};
use crate::domain::operator::contract::types::EmptyRepresentation;
use crate::domain::operator::contract::presence::{InputMask, PresenceMask, RowFailure};
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::{
    CellKind, Relation,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as Source, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, ReachLookup, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

fn equal(column: u32, text: &str) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ComparisonOrigin::Unrecorded,
    }
}
fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}
fn select(from: Source, comparisons: Vec<PlanComparison>) -> PlanStep {
    step(Operation::Select {
        from,
        comparisons,
        kept: vec![],
    })
}
fn at(step: u32) -> Source {
    Source::Step(StepId(step))
}
fn plan(steps: Vec<PlanStep>) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        emit: Emit {
            step: StepId(u32::try_from(steps.len() - 1).unwrap_or(0)),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps,
    }
}
fn graph(plans: &[Plan]) -> (RequestGraph, crate::domain::file_program::types::RequestOperators) {
    let operators = crate::domain::file_program::service::operators::prepare(
        plans,
        &vec![vec![]; plans.len()],
        &[],
        &|relation| kinds(relation),
        &mut |_| Ok(()),
    )
    .expect("the plans prepare");
    let graph = crate::domain::file_program::service::compile_prepared(plans, &operators);
    (graph, operators)
}
fn fold(plans: &[Plan]) -> GraphPresence {
    of_graph(&graph(plans).0, &|_| super::types::RelationScope::CurrentFile, crate::domain::matching::types::PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl))
}
/// The skip formula of the last step of the first plan.
fn last(presence: &GraphPresence, plans: &[Plan]) -> FormulaId {
    let (graph, _) = graph(plans);
    let node = graph.program_nodes[0]
        .last()
        .and_then(|node| node.as_ref().ok())
        .expect("the last step has a node");
    presence.nodes[node.0]
}
/// The tests a formula ANDs, as (relation, comparisons), when it is a conjunction of tests.
fn conjuncts(table: &PresenceTable, formula: FormulaId) -> Vec<(Relation, Vec<PlanComparison>)> {
    let test = |id: &super::types::TestId| {
        let test = &table.tests[id.0];
        (test.relation, test.comparisons.to_vec())
    };
    match &table.formulas[formula.0] {
        PresenceFormula::Test(id) => vec![test(id)],
        PresenceFormula::All(parts) => parts.iter().flat_map(|part| conjuncts(table, *part)).collect(),
        PresenceFormula::Always | PresenceFormula::Any(_) => vec![],
    }
}

fn every_operation_declares_the_inputs_its_rows_need() -> Result<(), OperatingError> {
    use OperationKind::*;
    for kind in OperationKind::ALL {
        let expected = match kind {
            Join | Intersection | OrderedMatch => PresenceMask::AllOf(InputMask(3)),
            Union => PresenceMask::AnyOf(InputMask(3)),
            Rows | Hosted => PresenceMask::Always,
            _ => PresenceMask::AllOf(InputMask(1)),
        };
        assert_eq!(semantics(*kind).presence, expected, "{kind:?}");
    }
    Ok(())
}

fn replace(column: u32, kinds: &[CellKind], scope: StepScope) -> RowFailure {
    prepare_for(
        OperationKind::Replace,
        &Operation::Replace {
            from: Source::Relation(Relation::Comment),
            text: ColumnIndex(column),
            pattern: LiteralText("a".into()),
            replacement: LiteralText("b".into()),
        },
        &[InputShape { kinds, tree: None }],
        &[],
        scope,
    )
    .map(|contract| contract.failure)
    .unwrap_or(RowFailure::OnRows(InputMask(0)))
}

fn replace_can_err_on_rows_unless_a_per_file_step_reads_a_symbol_or_file_column() -> Result<(), OperatingError> {
    let kinds = [CellKind::Node, CellKind::Symbol, CellKind::File];
    assert_eq!(replace(1, &kinds, StepScope::PerFile), RowFailure::Never);
    assert_eq!(replace(2, &kinds, StepScope::PerFile), RowFailure::Never);
    assert_eq!(replace(0, &kinds, StepScope::PerFile), RowFailure::OnRows(InputMask(1)));
    assert_eq!(replace(1, &kinds, StepScope::CrossFile), RowFailure::OnRows(InputMask(1)));
    assert_eq!(replace(9, &kinds, StepScope::PerFile), RowFailure::OnRows(InputMask(1)));
    Ok(())
}

fn the_row_fallible_operators_name_the_inputs_whose_rows_can_err() -> Result<(), OperatingError> {
    let kinds = kinds(Relation::Function);
    let shape = [InputShape { kinds, tree: None }, InputShape { kinds, tree: None }];
    let failure = |kind: OperationKind, operation: Operation| {
        prepare_for(kind, &operation, &shape, &[], StepScope::PerFile)
            .map(|contract| contract.failure)
    };
    assert_eq!(
        failure(
            OperationKind::Select,
            Operation::Select {
                from: Source::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![],
            }
        ),
        Ok(RowFailure::Never)
    );
    assert_eq!(
        failure(
            OperationKind::OrderedMatch,
            Operation::OrderedMatch {
                left: Source::Relation(Relation::Function),
                right: Source::Relation(Relation::Function),
                left_on: vec![],
                right_on: vec![],
                left_order: ColumnIndex(0),
                right_order: ColumnIndex(0),
                output: vec![],
            }
        ),
        Ok(RowFailure::OnRows(InputMask(3)))
    );
    assert_eq!(
        failure(
            OperationKind::SpanBounds,
            Operation::SpanBounds {
                from: Source::Relation(Relation::Function),
                span: ColumnIndex(0),
            }
        ),
        Ok(RowFailure::OnRows(InputMask(1)))
    );
    Ok(())
}

fn every_always_gated_empty_result_lies_inside_the_presence_mask() -> Result<(), OperatingError> {
    let node = kinds(Relation::SyntaxChild);
    let shapes = [
        InputShape { kinds: node, tree: None },
        InputShape { kinds: node, tree: None },
        InputShape { kinds: node, tree: None },
    ];
    let operations = [
        Operation::Antijoin {
            left: Source::Relation(Relation::SyntaxChild),
            right: Source::Relation(Relation::SyntaxChild),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        },
        Operation::Axis {
            from: Source::Relation(Relation::SyntaxChild),
            edges: Source::Relation(Relation::SyntaxChild),
            domain: Source::Relation(Relation::SyntaxChild),
            direction: ryo_values::domain::plan_parts::types::Direction::Descendant,
            id: ColumnIndex(0),
            parent: ColumnIndex(0),
            child: ColumnIndex(1),
            domain_id: ColumnIndex(0),
            domain_filter: vec![],
            domain_view: vec![],
            output: vec![],
        },
    ];
    for operation in operations {
        let kind = ryo_values::domain::plans::service::kind_of(&operation);
        let contract = prepare_for(kind, &operation, &shapes, &[], StepScope::PerFile)
            .expect("the contract prepares");
        let PresenceMask::AllOf(mask) = semantics(kind).presence else {
            panic!("{kind:?} requires its inputs");
        };
        for gate in &contract.empty_results {
            if gate.representation == EmptyRepresentation::Any {
                assert_ne!(mask.0 & (1 << gate.input.0), 0, "{kind:?} gates on input {}", gate.input.0);
            }
        }
    }
    Ok(())
}

/// A self join of one node: `l.kind = "x" and r.kind = "z"` needs a row of each kind, not one row with both.
fn a_self_join_tests_each_side_on_its_own() -> Result<(), OperatingError> {
    let plans = [plan(vec![
        select(Source::Relation(Relation::SyntaxTree), vec![]),
        step(Operation::Join {
            left: at(0),
            right: at(0),
            left_on: vec![ColumnIndex(3)],
            right_on: vec![ColumnIndex(3)],
            output: vec![
                JoinColumn { side: Side::Left, column: ColumnIndex(1) },
                JoinColumn { side: Side::Right, column: ColumnIndex(1) },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        }),
        select(at(1), vec![equal(0, "x"), equal(1, "z")]),
    ])];
    let presence = fold(&plans);
    let mut tests = conjuncts(&presence.table, last(&presence, &plans));
    tests.retain(|(_, comparisons)| !comparisons.is_empty());
    assert_eq!(
        tests,
        vec![
            (Relation::SyntaxTree, vec![equal(1, "x")]),
            (Relation::SyntaxTree, vec![equal(1, "z")]),
        ]
    );
    Ok(())
}

/// A join's right columns resolve as its schema does: through the view over a native tree input, directly with no view,
/// and not at all over a reach join, whose filter reads the closure's columns.
fn join_right_columns_resolve_through_the_schema_functions() -> Result<(), OperatingError> {
    let join = |view: Vec<ColumnIndex>, reach: Option<ReachLookup>, filter: Vec<PlanComparison>| {
        let plans = [plan(vec![
            select(Source::Relation(Relation::Comment), vec![]),
            step(Operation::Join {
                left: at(0),
                right: Source::Relation(Relation::SyntaxTree),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn { side: Side::Right, column: ColumnIndex(0) }],
                filter,
                view,
                reach,
            }),
            select(at(1), vec![equal(0, "x")]),
        ])];
        let presence = fold(&plans);
        let mut tests = conjuncts(&presence.table, last(&presence, &plans));
        tests.retain(|(_, comparisons)| !comparisons.is_empty());
        tests
    };
    assert_eq!(
        join(vec![ColumnIndex(2)], None, vec![]),
        vec![(Relation::SyntaxTree, vec![equal(2, "x")])],
        "a tree view moves the right column"
    );
    assert_eq!(
        join(vec![], None, vec![]),
        vec![(Relation::SyntaxTree, vec![equal(0, "x")])],
        "no view reads the column itself"
    );
    assert_eq!(
        join(vec![], None, vec![equal(1, "y")]),
        vec![
            (Relation::SyntaxTree, vec![equal(1, "y")]),
            (Relation::SyntaxTree, vec![equal(0, "x")]),
        ],
        "the filter tests the raw right input"
    );
    let reach = ReachLookup {
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
        keys: vec![],
        columns: vec![ColumnIndex(0), ColumnIndex(1)],
    };
    assert_eq!(
        join(vec![], Some(reach), vec![equal(1, "y")]),
        vec![],
        "a reach join's right side has no origin and its filter drops out"
    );
    Ok(())
}

/// `or` lowers to a union of selects: either branch may hold, and the union's columns have no origin.
fn a_union_is_any_of_its_branches_and_resets_origins() -> Result<(), OperatingError> {
    let plans = [plan(vec![
        select(Source::Relation(Relation::SyntaxTree), vec![equal(1, "x")]),
        select(Source::Relation(Relation::SyntaxTree), vec![equal(1, "y")]),
        step(Operation::Union { left: at(0), right: at(1) }),
        select(at(2), vec![equal(2, "z")]),
    ])];
    let presence = fold(&plans);
    let formula = last(&presence, &plans);
    let PresenceFormula::Any(branches) = &presence.table.formulas[formula.0] else {
        panic!("a union is any of its branches: {:?}", presence.table.formulas[formula.0]);
    };
    assert_eq!(branches.len(), 2);
    assert!(conjuncts(&presence.table, formula).is_empty());
    Ok(())
}

/// A cross-file step and everything downstream of it may have rows in any file, and so may a step over an invalid schema.
fn unconditional_nodes_and_their_consumers_are_always() -> Result<(), OperatingError> {
    let mut cross = plan(vec![
        select(Source::Relation(Relation::SyntaxTree), vec![equal(1, "x")]),
        select(at(0), vec![equal(1, "y")]),
    ]);
    cross.steps[0].scope = StepScope::CrossFile;
    cross.steps[1].scope = StepScope::CrossFile;
    let plans = [cross];
    let presence = fold(&plans);
    assert_eq!(last(&presence, &plans), ALWAYS);
    let invalid = [plan(vec![
        select(Source::Relation(Relation::SyntaxTree), vec![equal(99, "x")]),
        select(at(0), vec![equal(1, "y")]),
    ])];
    let presence = fold(&invalid);
    assert_eq!(last(&presence, &invalid), ALWAYS);
    Ok(())
}

/// A row-fallible operator upstream makes the skip formula hold whenever its input could have rows, whatever follows.
fn a_fallible_operator_upstream_holds_the_skip_formula_open() -> Result<(), OperatingError> {
    let plans = [plan(vec![
        select(Source::Relation(Relation::SyntaxTree), vec![]),
        step(Operation::SpanBounds { from: at(0), span: ColumnIndex(4) }),
        select(at(1), vec![equal(1, "never")]),
    ])];
    let presence = fold(&plans);
    let formula = last(&presence, &plans);
    let PresenceFormula::Any(parts) = &presence.table.formulas[formula.0] else {
        panic!("the failure term is OR-ed in: {:?}", presence.table.formulas[formula.0]);
    };
    assert_eq!(parts.len(), 2);
    Ok(())
}

/// Each region takes its last node's formula; a synthesized region, which is no node, and a region upstream of an
/// ordered-existence source are `Always`.
fn the_region_table_takes_each_regions_node() -> Result<(), OperatingError> {
    let plans = [plan(vec![select(Source::Relation(Relation::SyntaxTree), vec![equal(1, "x")])])];
    let presence = fold(&plans);
    let node = super::types::FoldedNode(0);
    let table = for_regions(
        presence.clone(),
        [
            super::types::RegionNode {
                node: Some(node),
                certified: super::types::Certification::Free,
            },
            super::types::RegionNode {
                node: None,
                certified: super::types::Certification::Free,
            },
            super::types::RegionNode {
                node: Some(node),
                certified: super::types::Certification::Certifying,
            },
        ],
    );
    assert_eq!(&*table.regions, &[presence.nodes[0], ALWAYS, ALWAYS]);
    assert_ne!(presence.nodes[0], ALWAYS);
    assert_eq!(&*table.reads[0], &[Relation::SyntaxTree]);
    Ok(())
}

/// What can raise a fault: the values of rows that reach an operator, or something fixed before any row is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    /// Raised by an operator the `failure` table names, on rows of the inputs its mask names.
    RowFallible,
    /// Raised by a comparison over a row; every literal comparison is inside its select's presence test, which holds
    /// when the comparison errs, so the select runs and reports it.
    Comparison,
    /// Fixed by the plan, the schema, a binding or the request, whatever the rows hold: such a node is `Always`, or its
    /// error is raised before rows are read.
    Fixed,
    /// The engine's own bookkeeping: raised the same whether a region runs or publishes empty.
    Engine,
    /// Named-input and semantic incompleteness: only downstream of a bound observation or a semantic renderer, both
    /// `Always` or row-fallible.
    Incomplete,
}

/// R6: every operating error and every portable operator fault, audited as row-dependent or not. The matches are
/// exhaustive, so a new variant cannot land without a verdict.
fn operating(error: &OperatingError) -> Fault {
    match error {
        OperatingError::DuplicateDfsOrder
        | OperatingError::DfsOrdinalOverflow
        | OperatingError::InvalidSpan
        | OperatingError::InvalidSourceMatchSpan
        | OperatingError::InvalidSpanBounds
        | OperatingError::DuplicateOrder
        | OperatingError::SemanticRender(_)
        | OperatingError::SemanticBatchShape => Fault::RowFallible,
        OperatingError::InvalidRegex { .. } | OperatingError::WrongTest => Fault::Comparison,
        // Replace's text gate; any other source is a schema check.
        OperatingError::ShapeMismatch => Fault::RowFallible,
        OperatingError::Spelling(_)
        | OperatingError::ObservationConfiguration(_)
        | OperatingError::MissingInput { .. }
        | OperatingError::WrongOperation
        | OperatingError::Unbound
        | OperatingError::WrongBinding
        | OperatingError::Table(_) => Fault::Fixed,
        OperatingError::NotRun | OperatingError::NotRead => Fault::Engine,
        OperatingError::ObservationIncomplete { .. } | OperatingError::SemanticIncomplete { .. } => {
            Fault::Incomplete
        }
    }
}
fn portable(fault: &ryo_operators::domain::contract::types::PortableOperatorFault) -> Fault {
    use ryo_operators::domain::contract::types::PortableOperatorFault::*;
    match fault {
        ShapeMismatch | InvalidSpanBounds | InvalidSpan | InvalidSourceMatchSpan | UnsupportedRender => {
            Fault::RowFallible
        }
        NoSuchColumn { .. } | WrongOperation | UnboundKey | WrongBinding | Unbound | MissingCapture => {
            Fault::Fixed
        }
        Interrupted | NotRun => Fault::Engine,
    }
}

fn every_fault_has_a_verdict() -> Result<(), OperatingError> {
    use ryo_operators::domain::contract::types::PortableOperatorFault;
    assert_eq!(operating(&OperatingError::DuplicateOrder), Fault::RowFallible);
    assert_eq!(operating(&OperatingError::InvalidRegex { reason: String::new() }), Fault::Comparison);
    assert_eq!(operating(&OperatingError::NotRun), Fault::Engine);
    assert_eq!(portable(&PortableOperatorFault::InvalidSpan), Fault::RowFallible);
    assert_eq!(portable(&PortableOperatorFault::MissingCapture), Fault::Fixed);
    Ok(())
}

type Input = fn() -> Result<Value, Failure>;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(every_operation_declares_the_inputs_its_rows_need)),
        word: Word::Succeeds,
        input: || every_operation_declares_the_inputs_its_rows_need,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(replace_can_err_on_rows_unless_a_per_file_step_reads_a_symbol_or_file_column)),
        word: Word::Succeeds,
        input: || replace_can_err_on_rows_unless_a_per_file_step_reads_a_symbol_or_file_column,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(the_row_fallible_operators_name_the_inputs_whose_rows_can_err)),
        word: Word::Succeeds,
        input: || the_row_fallible_operators_name_the_inputs_whose_rows_can_err,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(every_always_gated_empty_result_lies_inside_the_presence_mask)),
        word: Word::Succeeds,
        input: || every_always_gated_empty_result_lies_inside_the_presence_mask,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(a_self_join_tests_each_side_on_its_own)),
        word: Word::Succeeds,
        input: || a_self_join_tests_each_side_on_its_own,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(join_right_columns_resolve_through_the_schema_functions)),
        word: Word::Succeeds,
        input: || join_right_columns_resolve_through_the_schema_functions,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(a_union_is_any_of_its_branches_and_resets_origins)),
        word: Word::Succeeds,
        input: || a_union_is_any_of_its_branches_and_resets_origins,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(unconditional_nodes_and_their_consumers_are_always)),
        word: Word::Succeeds,
        input: || unconditional_nodes_and_their_consumers_are_always,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(a_fallible_operator_upstream_holds_the_skip_formula_open)),
        word: Word::Succeeds,
        input: || a_fallible_operator_upstream_holds_the_skip_formula_open,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(the_region_table_takes_each_regions_node)),
        word: Word::Succeeds,
        input: || the_region_table_takes_each_regions_node,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(every_fault_has_a_verdict)),
        word: Word::Succeeds,
        input: || every_fault_has_a_verdict,
        expect: Expect::Value(()),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
