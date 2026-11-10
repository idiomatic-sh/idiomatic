// ignore: idiomatic.arch/file-length - one case table for the one presence fold: every case shares its plan fixtures and helpers, and the fold has no parts of its own to split along.
//! The presence fold: every operation's declaration, the failure term of each prepared contract, the column origins a
//! test is grouped by, and a region table total over the compiled program.
use super::service::{ALWAYS, for_regions, of_graph};
use super::types::{FormulaId, GraphPresence, PresenceFormula, PresenceTable};
use crate::domain::file_program::types::RequestGraph;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::presence::{InputMask, PresenceMask, RowFailure};
use crate::domain::operator::contract::service::{prepare_for, semantics};
use crate::domain::operator::contract::types::EmptyRepresentation;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as Source, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, ReachLookup, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_operators::types::OperationKind;

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
fn graph(
    plans: &[Plan],
) -> (
    RequestGraph,
    crate::domain::file_program::types::RequestOperators,
) {
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
    of_graph(
        &graph(plans).0,
        &|_| super::types::RelationScope::CurrentFile,
        crate::domain::matching::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
        &crate::domain::matching::types::prepared::value::PreparedMatchers::default(),
    )
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
        PresenceFormula::All(parts) => parts
            .iter()
            .flat_map(|part| conjuncts(table, *part))
            .collect(),
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

fn replace_can_err_on_rows_unless_a_per_file_step_reads_a_symbol_or_file_column()
-> Result<(), OperatingError> {
    let kinds = [CellKind::Node, CellKind::Symbol, CellKind::File];
    assert_eq!(replace(1, &kinds, StepScope::PerFile), RowFailure::Never);
    assert_eq!(replace(2, &kinds, StepScope::PerFile), RowFailure::Never);
    assert_eq!(
        replace(0, &kinds, StepScope::PerFile),
        RowFailure::OnRows(InputMask(1))
    );
    assert_eq!(
        replace(1, &kinds, StepScope::CrossFile),
        RowFailure::OnRows(InputMask(1))
    );
    assert_eq!(
        replace(9, &kinds, StepScope::PerFile),
        RowFailure::OnRows(InputMask(1))
    );
    Ok(())
}

fn the_row_fallible_operators_name_the_inputs_whose_rows_can_err() -> Result<(), OperatingError> {
    let kinds = kinds(Relation::Function);
    let shape = [
        InputShape { kinds, tree: None },
        InputShape { kinds, tree: None },
    ];
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

/// A source search over `span` of `from`, with an optional named capture.
fn search(from: Source, span: u32, pattern: &str, capture: Option<&str>) -> PlanStep {
    step(Operation::SourceMatches {
        from,
        span: ColumnIndex(span),
        base_line: None,
        pattern: LiteralText(pattern.into()),
        capture: capture.map(|name| LiteralText(name.into())),
        capture_text: ryo_values::domain::query_operators::types::TextOutput::Omitted,
        capture_line: ryo_values::domain::query_operators::types::LineOutput::Omitted,
    })
}
fn project(from: Source, columns: &[u32]) -> PlanStep {
    step(Operation::Project {
        from,
        columns: columns.iter().copied().map(ColumnIndex).collect(),
        distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
    })
}
/// The fold with `shared` read as a relation of the project, not the file, and the request's slots declared from the
/// plans, as the program compiler declares them.
fn fold_sharing(plans: &[Plan], shared: Option<Relation>) -> GraphPresence {
    let prepared = crate::domain::matching::types::prepared::value::PreparedMatchers::new(plans);
    of_graph(
        &graph(plans).0,
        &|relation| {
            if Some(relation) == shared {
                super::types::RelationScope::Shared
            } else {
                super::types::RelationScope::CurrentFile
            }
        },
        crate::domain::matching::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
        &prepared,
    )
}
/// The provider span columns the fold relies on, as (relation, columns).
fn relied(presence: &GraphPresence) -> Vec<(Relation, Vec<u32>)> {
    presence
        .table
        .spans
        .iter()
        .map(|check| {
            (
                check.relation,
                check.columns.iter().map(|column| column.0).collect(),
            )
        })
        .collect()
}

/// L1: a source search declares `UnlessLocated` only as a per-file step over a span column with no base line and no row
/// error; each other case keeps its first input's rows fallible.
fn source_matches_can_err_on_rows_unless_per_file_over_a_span_without_base_line_or_row_error()
-> Result<(), OperatingError> {
    let kinds = kinds(Relation::Comment);
    let shape = [InputShape { kinds, tree: None }];
    let failure = |operation: Operation, scope: StepScope| {
        prepare_for(OperationKind::SourceMatches, &operation, &shape, &[], scope)
            .map(|contract| contract.failure)
    };
    let searching =
        |span: u32,
         base_line: Option<u32>,
         capture: Option<&str>,
         capture_text: ryo_values::domain::query_operators::types::TextOutput| {
            Operation::SourceMatches {
                from: Source::Relation(Relation::Comment),
                span: ColumnIndex(span),
                base_line: base_line.map(ColumnIndex),
                pattern: LiteralText("a".into()),
                capture: capture.map(|name| LiteralText(name.into())),
                capture_text,
                capture_line: ryo_values::domain::query_operators::types::LineOutput::Omitted,
            }
        };
    use ryo_values::domain::query_operators::types::TextOutput::{Appended, Omitted};
    assert_eq!(
        failure(searching(3, None, Some("x"), Appended), StepScope::PerFile),
        Ok(RowFailure::UnlessLocated(ColumnIndex(3)))
    );
    // A cross-file step.
    assert_eq!(
        failure(searching(3, None, None, Omitted), StepScope::CrossFile),
        Ok(RowFailure::OnRows(InputMask(1)))
    );
    // A base line, read from the row, can be below one.
    assert_eq!(
        failure(searching(3, Some(0), None, Omitted), StepScope::PerFile),
        Ok(RowFailure::OnRows(InputMask(1)))
    );
    // A row error: captured text appended with no capture.
    assert_eq!(
        failure(searching(3, None, None, Appended), StepScope::PerFile),
        Ok(RowFailure::OnRows(InputMask(1)))
    );
    // A column that is no span.
    assert_eq!(
        failure(searching(1, None, None, Omitted), StepScope::PerFile),
        Ok(RowFailure::OnRows(InputMask(1)))
    );
    Ok(())
}

/// L1 and L2: a search over a provider span adds no failure term, so a narrowing downstream decides the skip on its own,
/// and the table lists the span column the file must validate.
fn a_search_over_a_provider_span_adds_no_failure_term() -> Result<(), OperatingError> {
    let plans = [plan(vec![
        search(Source::Relation(Relation::Comment), 3, "a", None),
        select(at(0), vec![equal(1, "never")]),
    ])];
    let presence = fold_sharing(&plans, None);
    let formula = last(&presence, &plans);
    assert!(
        matches!(&presence.table.formulas[formula.0], PresenceFormula::All(_)),
        "no failure term is OR-ed in: {:?}",
        presence.table.formulas[formula.0]
    );
    assert_eq!(relied(&presence), vec![(Relation::Comment, vec![3])]);
    Ok(())
}

/// L1: an oversized pattern, a missing named capture, a base line or a row error each keep the failure term.
fn a_search_that_can_still_fail_keeps_its_failure_term() -> Result<(), OperatingError> {
    let open = |first: PlanStep| {
        let plans = [plan(vec![first, select(at(0), vec![equal(1, "never")])])];
        let presence = fold_sharing(&plans, None);
        let formula = last(&presence, &plans);
        (
            matches!(&presence.table.formulas[formula.0], PresenceFormula::Any(_)),
            relied(&presence),
        )
    };
    let from = Source::Relation(Relation::Comment);
    assert_eq!(
        open(search(from, 3, r"\w{1000}{1000}", None)),
        (true, vec![])
    );
    assert_eq!(open(search(from, 3, "(?P<x>a)", Some("y"))), (true, vec![]));
    assert_eq!(
        open(search(from, 3, "(?P<x>a)", Some("x"))),
        (false, vec![(Relation::Comment, vec![3])])
    );
    // A base line: the start line of a source location.
    let numbered = [plan(vec![
        step(Operation::SourceMatches {
            from: Source::Relation(Relation::SourceLocation),
            span: ColumnIndex(2),
            base_line: Some(ColumnIndex(3)),
            pattern: LiteralText("a".into()),
            capture: None,
            capture_text: ryo_values::domain::query_operators::types::TextOutput::Omitted,
            capture_line: ryo_values::domain::query_operators::types::LineOutput::Omitted,
        }),
        select(at(0), vec![]),
    ])];
    let presence = fold_sharing(&numbered, None);
    assert_ne!(
        last(&presence, &numbered),
        ALWAYS,
        "the plan is conditional"
    );
    assert_eq!(relied(&presence), vec![]);
    let row_error = step(Operation::SourceMatches {
        from,
        span: ColumnIndex(3),
        base_line: None,
        pattern: LiteralText("a".into()),
        capture: None,
        capture_text: ryo_values::domain::query_operators::types::TextOutput::Appended,
        capture_line: ryo_values::domain::query_operators::types::LineOutput::Omitted,
    });
    assert_eq!(open(row_error), (true, vec![]));
    Ok(())
}

/// L2: the noqa grammar's shape. Heads are two searches over the comment span, unioned; codes search the heads' capture
/// span, and tokens the codes' capture span. Each is located, so no search adds a failure term; a union with a side read
/// from the project is not located, and its search keeps the term.
fn located_spans_pass_a_union_of_located_sides() -> Result<(), OperatingError> {
    let comment = Source::Relation(Relation::Comment);
    // Steps 0-3: the heads, comment columns 0-4 then marker 5 and suffix span 6.
    let heads = vec![
        search(comment, 3, "#(?P<suffix>.*)$", Some("suffix")),
        search(comment, 3, "#:(?P<suffix>.*)$", Some("suffix")),
        project(at(0), &[0, 2, 5, 6]),
        project(at(1), &[0, 2, 5, 6]),
        step(Operation::Union {
            left: at(2),
            right: at(3),
        }),
    ];
    let mut codes = heads.clone();
    codes.extend([
        // The codes list over the suffix span: columns 0-3, then list 4 and codes span 5.
        search(at(4), 3, "^:(?P<codes>.*)$", Some("codes")),
        // A token over the codes span.
        search(at(5), 5, "(?P<code>[A-Z]+[0-9]+)", Some("code")),
        select(at(6), vec![]),
    ]);
    let plans = [plan(codes)];
    let presence = fold_sharing(&plans, None);
    assert_eq!(relied(&presence), vec![(Relation::Comment, vec![3])]);
    let formula = last(&presence, &plans);
    assert!(
        !matches!(&presence.table.formulas[formula.0], PresenceFormula::Any(_)),
        "no search holds the skip open: {:?}",
        presence.table.formulas[formula.0]
    );
    // One side read from a relation of the project: its spans are not located, and the union's are not either.
    let mixed = [plan(vec![
        project(comment, &[3]),
        project(Source::Relation(Relation::SourceLocation), &[2]),
        step(Operation::Union {
            left: at(0),
            right: at(1),
        }),
        search(at(2), 0, "a", None),
    ])];
    let presence = fold_sharing(&mixed, Some(Relation::SourceLocation));
    assert_eq!(relied(&presence), vec![]);
    let both = fold_sharing(&mixed, None);
    assert_eq!(
        relied(&both),
        vec![
            (Relation::SourceLocation, vec![2]),
            (Relation::Comment, vec![3])
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
    );
    Ok(())
}

fn every_always_gated_empty_result_lies_inside_the_presence_mask() -> Result<(), OperatingError> {
    let node = kinds(Relation::SyntaxChild);
    let shapes = [
        InputShape {
            kinds: node,
            tree: None,
        },
        InputShape {
            kinds: node,
            tree: None,
        },
        InputShape {
            kinds: node,
            tree: None,
        },
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
                assert_ne!(
                    mask.0 & (1 << gate.input.0),
                    0,
                    "{kind:?} gates on input {}",
                    gate.input.0
                );
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
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(1),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(1),
                },
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
                output: vec![JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(0),
                }],
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
        step(Operation::Union {
            left: at(0),
            right: at(1),
        }),
        select(at(2), vec![equal(2, "z")]),
    ])];
    let presence = fold(&plans);
    let formula = last(&presence, &plans);
    let PresenceFormula::Any(branches) = &presence.table.formulas[formula.0] else {
        panic!(
            "a union is any of its branches: {:?}",
            presence.table.formulas[formula.0]
        );
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
        step(Operation::SpanBounds {
            from: at(0),
            span: ColumnIndex(4),
        }),
        select(at(1), vec![equal(1, "never")]),
    ])];
    let presence = fold(&plans);
    let formula = last(&presence, &plans);
    let PresenceFormula::Any(parts) = &presence.table.formulas[formula.0] else {
        panic!(
            "the failure term is OR-ed in: {:?}",
            presence.table.formulas[formula.0]
        );
    };
    assert_eq!(parts.len(), 2);
    Ok(())
}

/// Each region takes its last node's formula; a synthesized region, which is no node, and a region upstream of an
/// ordered-existence source are `Always`.
fn the_region_table_takes_each_regions_node() -> Result<(), OperatingError> {
    let plans = [plan(vec![select(
        Source::Relation(Relation::SyntaxTree),
        vec![equal(1, "x")],
    )])];
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

/// An operation that raises an error from what reaches it, and the condition it raises it on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Raiser {
    /// On the values of its input rows.
    Rows(OperationKind),
    /// On a literal pattern that does not compile, when a row first reaches it.
    Pattern(OperationKind),
}

/// What can raise a fault: the values of rows that reach an operator, or something fixed before any row is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    /// Raised by these operations on rows of an input: each must declare the input fallible, through its prepared
    /// contract's failure mask or, for a literal pattern, through the fold.
    RowFallible(&'static [Raiser]),
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

const DFS: &[Raiser] = &[Raiser::Rows(OperationKind::OrderedDfs)];
const ORDERED: &[Raiser] = &[Raiser::Rows(OperationKind::OrderedMatch)];
const SPAN: &[Raiser] = &[Raiser::Rows(OperationKind::SpanBetween)];
const SOURCE_SPAN: &[Raiser] = &[Raiser::Rows(OperationKind::SourceMatches)];
const BOUNDS: &[Raiser] = &[Raiser::Rows(OperationKind::SpanBounds)];
const RENDER: &[Raiser] = &[
    Raiser::Rows(OperationKind::RenderSyntax),
    Raiser::Rows(OperationKind::FitsRendered),
];
const TEXT_GATE: &[Raiser] = &[
    Raiser::Rows(OperationKind::Replace),
    Raiser::Rows(OperationKind::SourceMatches),
];
const CAPTURE: &[Raiser] = &[Raiser::Pattern(OperationKind::SourceMatches)];
const REGEX: &[Raiser] = &[
    Raiser::Rows(OperationKind::Select),
    Raiser::Rows(OperationKind::Join),
    Raiser::Rows(OperationKind::Axis),
    Raiser::Pattern(OperationKind::Select),
    Raiser::Pattern(OperationKind::Join),
    Raiser::Pattern(OperationKind::Axis),
    Raiser::Pattern(OperationKind::Replace),
    Raiser::Pattern(OperationKind::SourceMatches),
];

/// R6: every operating error and every portable operator fault, audited as row-dependent or not. The matches are
/// exhaustive, so a new variant cannot land without a verdict, and a row-dependent verdict names the operations that
/// raise it.
fn operating(error: &OperatingError) -> Fault {
    match error {
        OperatingError::DuplicateDfsOrder | OperatingError::DfsOrdinalOverflow => {
            Fault::RowFallible(DFS)
        }
        OperatingError::DuplicateOrder => Fault::RowFallible(ORDERED),
        OperatingError::InvalidSpan => Fault::RowFallible(SPAN),
        OperatingError::InvalidSourceMatchSpan => Fault::RowFallible(SOURCE_SPAN),
        OperatingError::InvalidSpanBounds => Fault::RowFallible(BOUNDS),
        OperatingError::SemanticRender(_) | OperatingError::SemanticBatchShape => {
            Fault::RowFallible(RENDER)
        }
        // A regular expression read from the row, or a literal one that does not compile.
        OperatingError::InvalidRegex { .. } => Fault::RowFallible(REGEX),
        OperatingError::WrongTest => Fault::Comparison,
        // Replace's text gate and SourceMatches' span gate; any other source is a schema check.
        OperatingError::ShapeMismatch => Fault::RowFallible(TEXT_GATE),
        OperatingError::Spelling(_)
        | OperatingError::ObservationConfiguration(_)
        | OperatingError::MissingInput { .. }
        | OperatingError::WrongOperation
        | OperatingError::Unbound
        | OperatingError::WrongBinding
        | OperatingError::Table(_) => Fault::Fixed,
        OperatingError::NotRun | OperatingError::NotRead => Fault::Engine,
        OperatingError::ObservationIncomplete { .. }
        | OperatingError::SemanticIncomplete { .. } => Fault::Incomplete,
    }
}
fn portable(fault: &ryo_operators::domain::contract::types::PortableOperatorFault) -> Fault {
    use ryo_operators::domain::contract::types::PortableOperatorFault::*;
    match fault {
        ShapeMismatch => Fault::RowFallible(TEXT_GATE),
        InvalidSpanBounds => Fault::RowFallible(BOUNDS),
        InvalidSpan => Fault::RowFallible(SPAN),
        InvalidSourceMatchSpan => Fault::RowFallible(SOURCE_SPAN),
        UnsupportedRender => Fault::RowFallible(RENDER),
        NoSuchColumn { .. } | WrongOperation | UnboundKey | WrongBinding | Unbound => Fault::Fixed,
        // A named capture the matcher lacks, raised when a row first reaches the search.
        MissingCapture => Fault::RowFallible(CAPTURE),
        Interrupted | NotRun => Fault::Engine,
    }
}

/// An operation of this kind in the condition its raiser names: a node column for Replace's text gate, a pattern read
/// from the row for a comparison, and the literal pattern `(` for a literal one.
fn raising(raiser: Raiser) -> Operation {
    let from = Source::Relation(Relation::Function);
    let comparisons = |pattern: Operand| {
        vec![PlanComparison {
            test: Test::Matches,
            left: Operand::Column(ColumnIndex(0)),
            right: pattern,
            origin: ComparisonOrigin::Unrecorded,
        }]
    };
    let (kind, pattern) = match raiser {
        Raiser::Rows(kind) => (kind, Operand::Column(ColumnIndex(1))),
        Raiser::Pattern(kind) => (
            kind,
            Operand::Literal(QueryLiteral::Text(LiteralText("(".into()))),
        ),
    };
    let literal = match &pattern {
        Operand::Literal(QueryLiteral::Text(text)) => text.clone(),
        _ => LiteralText("a".into()),
    };
    match kind {
        OperationKind::Select => Operation::Select {
            from,
            comparisons: comparisons(pattern),
            kept: vec![],
        },
        OperationKind::Join => Operation::Join {
            left: from,
            right: from,
            left_on: vec![],
            right_on: vec![],
            output: vec![],
            filter: comparisons(pattern),
            view: vec![],
            reach: None,
        },
        OperationKind::Axis => Operation::Axis {
            from,
            edges: from,
            domain: from,
            direction: ryo_values::domain::plan_parts::types::Direction::Descendant,
            id: ColumnIndex(0),
            parent: ColumnIndex(0),
            child: ColumnIndex(1),
            domain_id: ColumnIndex(0),
            domain_filter: comparisons(pattern),
            domain_view: vec![],
            output: vec![],
        },
        OperationKind::Replace => Operation::Replace {
            from,
            // A node column has no text, so the text gate raises on its rows.
            text: ColumnIndex(if matches!(raiser, Raiser::Rows(_)) {
                0
            } else {
                1
            }),
            pattern: literal,
            replacement: LiteralText("b".into()),
        },
        // A per-file search over the comment span, so its contract declares `UnlessLocated`.
        OperationKind::SourceMatches => Operation::SourceMatches {
            from: Source::Relation(Relation::Comment),
            span: ColumnIndex(3),
            base_line: None,
            pattern: literal,
            capture: None,
            capture_text: ryo_values::domain::query_operators::types::TextOutput::Omitted,
            capture_line: ryo_values::domain::query_operators::types::LineOutput::Omitted,
        },
        OperationKind::OrderedDfs => Operation::OrderedDfs {
            from,
            edges: from,
            root: ColumnIndex(0),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            order: ColumnIndex(0),
            edge: ColumnIndex(0),
            root_keys: vec![],
            edge_keys: vec![],
        },
        OperationKind::OrderedMatch => Operation::OrderedMatch {
            left: from,
            right: from,
            left_on: vec![],
            right_on: vec![],
            left_order: ColumnIndex(0),
            right_order: ColumnIndex(0),
            output: vec![],
        },
        OperationKind::SpanBetween => Operation::SpanBetween {
            from,
            start: ColumnIndex(0),
            end: ColumnIndex(0),
        },
        OperationKind::SpanBounds => Operation::SpanBounds {
            from,
            span: ColumnIndex(0),
        },
        OperationKind::RenderSyntax => Operation::RenderSyntax {
            from,
            node: ColumnIndex(0),
            context: ryo_language::domain::semantics::types::RenderContext::AssignmentTarget,
        },
        OperationKind::FitsRendered => Operation::FitsRendered {
            from,
            texts: vec![],
            literals: vec![],
            span: ColumnIndex(0),
            max_line: ryo_values::domain::query_operators::types::MaxLine(88),
            tab_width: ryo_values::domain::query_operators::types::TabWidth(8),
        },
        other => panic!("no raising operation for {other:?}"),
    }
}

/// One fault the catalogue audits: an operating error or a portable operator fault.
#[derive(Clone, Copy, Debug)]
enum Raised<'a> {
    Operating(&'a OperatingError),
    Portable(&'a ryo_operators::domain::contract::types::PortableOperatorFault),
}

/// The proof that lets the fold drop an `UnlessLocated` failure term for a fault: the file's admission of the relied spans
/// through the search's own span gate (`validate_spans`), or the prepared slot's compile of the pattern and its named
/// capture (`relied`). The matches are exhaustive, so a new fault cannot land without saying whether a proof covers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LocatedProof {
    SpanGate,
    MatcherSlot,
}
fn located_proof(raised: Raised<'_>) -> Option<LocatedProof> {
    use ryo_operators::domain::contract::types::PortableOperatorFault as P;
    match raised {
        Raised::Operating(
            OperatingError::InvalidSourceMatchSpan | OperatingError::ShapeMismatch,
        )
        | Raised::Portable(P::InvalidSourceMatchSpan | P::ShapeMismatch) => {
            Some(LocatedProof::SpanGate)
        }
        Raised::Operating(OperatingError::InvalidRegex { .. })
        | Raised::Portable(P::MissingCapture) => Some(LocatedProof::MatcherSlot),
        Raised::Operating(
            OperatingError::DuplicateDfsOrder
            | OperatingError::DfsOrdinalOverflow
            | OperatingError::DuplicateOrder
            | OperatingError::InvalidSpan
            | OperatingError::InvalidSpanBounds
            | OperatingError::SemanticRender(_)
            | OperatingError::SemanticBatchShape
            | OperatingError::WrongTest
            | OperatingError::Spelling(_)
            | OperatingError::ObservationConfiguration(_)
            | OperatingError::MissingInput { .. }
            | OperatingError::WrongOperation
            | OperatingError::Unbound
            | OperatingError::WrongBinding
            | OperatingError::Table(_)
            | OperatingError::NotRun
            | OperatingError::NotRead
            | OperatingError::ObservationIncomplete { .. }
            | OperatingError::SemanticIncomplete { .. },
        )
        | Raised::Portable(
            P::InvalidSpanBounds
            | P::InvalidSpan
            | P::UnsupportedRender
            | P::NoSuchColumn { .. }
            | P::WrongOperation
            | P::UnboundKey
            | P::WrongBinding
            | P::Unbound
            | P::Interrupted
            | P::NotRun,
        ) => None,
    }
}

/// Each operating error's place in the catalogue's list, by declaration order: a new variant must take a place here,
/// and then the list must hold an instance of it.
fn operating_position(error: &OperatingError) -> usize {
    match error {
        OperatingError::Spelling(_) => 0,
        OperatingError::ObservationConfiguration(_) => 1,
        OperatingError::ObservationIncomplete { .. } => 2,
        OperatingError::SemanticRender(_) => 3,
        OperatingError::SemanticBatchShape => 4,
        OperatingError::SemanticIncomplete { .. } => 5,
        OperatingError::DuplicateDfsOrder => 6,
        OperatingError::DfsOrdinalOverflow => 7,
        OperatingError::InvalidSpan => 8,
        OperatingError::InvalidSourceMatchSpan => 9,
        OperatingError::InvalidSpanBounds => 10,
        OperatingError::DuplicateOrder => 11,
        OperatingError::MissingInput { .. } => 12,
        OperatingError::WrongOperation => 13,
        OperatingError::NotRun => 14,
        OperatingError::NotRead => 15,
        OperatingError::Unbound => 16,
        OperatingError::WrongBinding => 17,
        OperatingError::WrongTest => 18,
        OperatingError::InvalidRegex { .. } => 19,
        OperatingError::ShapeMismatch => 20,
        OperatingError::Table(_) => 21,
    }
}
/// Each portable operator fault's place in the catalogue's list, by declaration order.
fn portable_position(
    fault: &ryo_operators::domain::contract::types::PortableOperatorFault,
) -> usize {
    use ryo_operators::domain::contract::types::PortableOperatorFault::*;
    match fault {
        Interrupted => 0,
        NoSuchColumn { .. } => 1,
        ShapeMismatch => 2,
        InvalidSpanBounds => 3,
        InvalidSpan => 4,
        InvalidSourceMatchSpan => 5,
        MissingCapture => 6,
        WrongOperation => 7,
        UnboundKey => 8,
        UnsupportedRender => 9,
        WrongBinding => 10,
        Unbound => 11,
        NotRun => 12,
    }
}

/// Whether the raiser declares the input it raises this fault on: the prepared contract's failure mask, or the fold's
/// check of a literal pattern. `UnlessLocated` declares a fault only when a proof the fold requires before dropping the
/// term covers it.
fn declared(raiser: Raiser, raised: Raised<'_>) -> bool {
    let operation = raising(raiser);
    let kind = ryo_values::domain::plans::service::kind_of(&operation);
    let relation = match kind {
        OperationKind::SourceMatches => Relation::Comment,
        _ => Relation::Function,
    };
    let shapes = [InputShape {
        kinds: kinds(relation),
        tree: None,
    }; 3];
    let failure = prepare_for(kind, &operation, &shapes, &[], StepScope::PerFile)
        .map(|contract| contract.failure);
    let rows = match failure {
        Ok(RowFailure::OnRows(mask)) => mask != InputMask(0),
        Ok(RowFailure::UnlessLocated(_)) => located_proof(raised).is_some(),
        Ok(RowFailure::Never) | Err(_) => false,
    };
    match raiser {
        Raiser::Rows(_) => rows,
        Raiser::Pattern(_) => {
            rows || super::narrowing::invalid_patterns(
                &operation,
                crate::domain::matching::types::PatternMatcher(
                    &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
                ),
                &crate::domain::matching::types::prepared::value::PreparedMatchers::default(),
            )
            .is_some()
        }
    }
}

fn every_fault_has_a_verdict() -> Result<(), OperatingError> {
    use ryo_operators::domain::contract::types::PortableOperatorFault;
    let reason = || String::new();
    // One instance of every variant, in declaration order: `operating_position` and `portable_position` are exhaustive,
    // and each list must map onto 0..len, so a new variant cannot be audited by omission.
    let errors = [
        OperatingError::Spelling(
            crate::domain::spelling::errors::SpellingError::EntityInTwoSources {
                first: reason(),
                second: reason(),
            },
        ),
        OperatingError::ObservationConfiguration(
            crate::domain::rule_intent::errors::RuleIntentError::TooManyOutputs,
        ),
        OperatingError::ObservationIncomplete { reason: reason() },
        OperatingError::SemanticRender(
            ryo_language::domain::semantics::errors::SemanticsError::StaleNode {
                node: ryo_relations::domain::relations::types::NodeId(0),
            },
        ),
        OperatingError::SemanticBatchShape,
        OperatingError::SemanticIncomplete { reason: reason() },
        OperatingError::DuplicateDfsOrder,
        OperatingError::DfsOrdinalOverflow,
        OperatingError::InvalidSpan,
        OperatingError::InvalidSourceMatchSpan,
        OperatingError::InvalidSpanBounds,
        OperatingError::DuplicateOrder,
        OperatingError::MissingInput { position: 0 },
        OperatingError::WrongOperation,
        OperatingError::NotRun,
        OperatingError::NotRead,
        OperatingError::Unbound,
        OperatingError::WrongBinding,
        OperatingError::WrongTest,
        OperatingError::InvalidRegex { reason: reason() },
        OperatingError::ShapeMismatch,
        OperatingError::Table(ryo_values::domain::rows::errors::RowsError::NoSuchColumn {
            index: 0,
        }),
    ];
    let faults = [
        PortableOperatorFault::Interrupted,
        PortableOperatorFault::NoSuchColumn {
            index: ColumnIndex(0),
        },
        PortableOperatorFault::ShapeMismatch,
        PortableOperatorFault::InvalidSpanBounds,
        PortableOperatorFault::InvalidSpan,
        PortableOperatorFault::InvalidSourceMatchSpan,
        PortableOperatorFault::MissingCapture,
        PortableOperatorFault::WrongOperation,
        PortableOperatorFault::UnboundKey,
        PortableOperatorFault::UnsupportedRender,
        PortableOperatorFault::WrongBinding,
        PortableOperatorFault::Unbound,
        PortableOperatorFault::NotRun,
    ];
    assert_eq!(
        errors.iter().map(operating_position).collect::<Vec<_>>(),
        (0..errors.len()).collect::<Vec<_>>()
    );
    assert_eq!(
        faults.iter().map(portable_position).collect::<Vec<_>>(),
        (0..faults.len()).collect::<Vec<_>>()
    );
    let verdicts = errors
        .iter()
        .map(|error| (Raised::Operating(error), operating(error)))
        .chain(
            faults
                .iter()
                .map(|fault| (Raised::Portable(fault), portable(fault))),
        );
    let mut searched = Vec::new();
    let mut proven = Vec::new();
    for (raised, verdict) in verdicts {
        if let Fault::RowFallible(raisers) = verdict {
            for raiser in raisers {
                assert!(
                    declared(*raiser, raised),
                    "{raiser:?} raises {raised:?} on rows and does not declare it"
                );
            }
            if raisers.iter().any(|raiser| {
                matches!(
                    raiser,
                    Raiser::Rows(OperationKind::SourceMatches)
                        | Raiser::Pattern(OperationKind::SourceMatches)
                )
            }) {
                searched.push(format!("{raised:?}"));
            }
        }
        if located_proof(raised).is_some() {
            proven.push(format!("{raised:?}"));
        }
    }
    // The faults a source search raises on rows are exactly the faults an L-span proof covers.
    assert_eq!(searched, proven);
    // A literal pattern over the compiled size limit is refused at first use like a syntax error, and declared alike.
    let oversized = Operation::Select {
        from: Source::Relation(Relation::Function),
        comparisons: vec![PlanComparison {
            test: Test::Matches,
            left: Operand::Column(ColumnIndex(1)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(r"\w{1000}{1000}".into()))),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: vec![],
    };
    assert_eq!(
        super::narrowing::invalid_patterns(
            &oversized,
            crate::domain::matching::types::PatternMatcher(
                &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
            ),
            &crate::domain::matching::types::prepared::value::PreparedMatchers::default(),
        ),
        Some(InputMask(1))
    );
    assert_eq!(operating(&OperatingError::NotRun), Fault::Engine);
    assert_eq!(
        portable(&PortableOperatorFault::MissingCapture),
        Fault::RowFallible(CAPTURE)
    );
    Ok(())
}

type Input = fn() -> Result<Value, Failure>;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(
            every_operation_declares_the_inputs_its_rows_need
        )),
        word: Word::Succeeds,
        input: || every_operation_declares_the_inputs_its_rows_need,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(
            replace_can_err_on_rows_unless_a_per_file_step_reads_a_symbol_or_file_column
        )),
        word: Word::Succeeds,
        input: || replace_can_err_on_rows_unless_a_per_file_step_reads_a_symbol_or_file_column,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(
            the_row_fallible_operators_name_the_inputs_whose_rows_can_err
        )),
        word: Word::Succeeds,
        input: || the_row_fallible_operators_name_the_inputs_whose_rows_can_err,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(
            every_always_gated_empty_result_lies_inside_the_presence_mask
        )),
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
        name: CaseName(stringify!(
            join_right_columns_resolve_through_the_schema_functions
        )),
        word: Word::Succeeds,
        input: || join_right_columns_resolve_through_the_schema_functions,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(
            a_union_is_any_of_its_branches_and_resets_origins
        )),
        word: Word::Succeeds,
        input: || a_union_is_any_of_its_branches_and_resets_origins,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(
            unconditional_nodes_and_their_consumers_are_always
        )),
        word: Word::Succeeds,
        input: || unconditional_nodes_and_their_consumers_are_always,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(
            a_fallible_operator_upstream_holds_the_skip_formula_open
        )),
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
    Case {
        name: CaseName(stringify!(
            source_matches_can_err_on_rows_unless_per_file_over_a_span_without_base_line_or_row_error
        )),
        word: Word::Succeeds,
        input: || source_matches_can_err_on_rows_unless_per_file_over_a_span_without_base_line_or_row_error,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(
            a_search_over_a_provider_span_adds_no_failure_term
        )),
        word: Word::Succeeds,
        input: || a_search_over_a_provider_span_adds_no_failure_term,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(
            a_search_that_can_still_fail_keeps_its_failure_term
        )),
        word: Word::Succeeds,
        input: || a_search_that_can_still_fail_keeps_its_failure_term,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(
            located_spans_pass_a_union_of_located_sides
        )),
        word: Word::Succeeds,
        input: || located_spans_pass_a_union_of_located_sides,
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
