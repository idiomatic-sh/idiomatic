//! The engine's sharing contract, language-free: every admitted sharing class is shared by two
//! rules and walks one traversal beside a distinct class, and every barrier kind is accounted
//! for. Coverage is an exhaustive match over the engine's own enums, so a new class shape,
//! partition or barrier fails to compile until it has a case here.
use super::super::super::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::routes::{Combine, Feed, Fusion, RowRoute};
use super::super::fixtures::semantics::{Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use ryo_execution_domain::domain::shared_walk::types::{
    WalkCensus,
    walk_route::value::{WalkBarrier, WalkRoute},
};
use ryo_values::domain::plan_parts::types::{
    ComparisonOrigin, Operand, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::query_operators::types::OperationKind;
use std::collections::BTreeSet;

fn program(authored: Authored) -> Program {
    Program::new(
        authored,
        Population::Three,
        Residency::Stable,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new()),
    )
    .expect("compile the authored sharing fixture")
}

/// The partitions a class can select, in order; no wildcard, so a new partition needs a case.
fn after_scope(scope: Option<StepScope>) -> Option<StepScope> {
    match scope {
        None => Some(StepScope::PerFile),
        Some(StepScope::PerFile) => Some(StepScope::CrossFile),
        Some(StepScope::CrossFile) => None,
    }
}

/// The head of an admitted sharing class for each operation kind, or none when the kind never
/// heads a class. No wildcard: a new operation kind must decide whether it is admitted.
fn admitted_head(kind: OperationKind) -> Option<Operation> {
    let from = PlanInput::Relation(Relation::LanguageTarget);
    match kind {
        OperationKind::Select => Some(Operation::Select {
            from,
            comparisons: vec![PlanComparison {
                test: Test::GreaterThan,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Literal(QueryLiteral::Number(Integer(2))),
                origin: ComparisonOrigin::Unrecorded,
            }],
            kept: vec![ColumnIndex(1)],
        }),
        OperationKind::Project => Some(Operation::Project {
            from,
            columns: vec![ColumnIndex(1)],
            distinct: RepeatedRows::Kept,
        }),
        OperationKind::Rows
        | OperationKind::Hosted
        | OperationKind::Join
        | OperationKind::Union
        | OperationKind::Intersection
        | OperationKind::Difference
        | OperationKind::Antijoin
        | OperationKind::OrderedMatch
        | OperationKind::Axis
        | OperationKind::OrderedDfs
        | OperationKind::GroupCount
        | OperationKind::GroupDigest
        | OperationKind::Words
        | OperationKind::Closure
        | OperationKind::Cycles
        | OperationKind::Extend
        | OperationKind::Capture
        | OperationKind::Rank
        | OperationKind::Replace
        | OperationKind::SpanBetween
        | OperationKind::SourceMatches
        | OperationKind::RenderSyntax
        | OperationKind::FitsRendered
        | OperationKind::SpanBounds => None,
    }
}

/// The barrier kinds, in order; no wildcard, so a new barrier needs a case.
fn after_barrier(barrier: Option<WalkBarrier>) -> Option<WalkBarrier> {
    match barrier {
        None => Some(WalkBarrier::Representation),
        Some(WalkBarrier::Representation) => Some(WalkBarrier::DependentInput),
        Some(WalkBarrier::DependentInput) => Some(WalkBarrier::Observation),
        Some(WalkBarrier::Observation) => Some(WalkBarrier::RowSemantics),
        Some(WalkBarrier::RowSemantics) => Some(WalkBarrier::DerivedOrOrdered),
        Some(WalkBarrier::DerivedOrOrdered) => Some(WalkBarrier::RequestOwnedSource),
        Some(WalkBarrier::RequestOwnedSource) => Some(WalkBarrier::Continuation),
        Some(WalkBarrier::Continuation) => None,
    }
}

/// The fixture rule set whose compiled program holds at least one instruction behind `barrier`.
fn barrier_fixture(barrier: WalkBarrier) -> Authored {
    match barrier {
        WalkBarrier::Representation => Authored::declined(),
        WalkBarrier::DependentInput => {
            Authored::combined(Combine::Join, Feed::Healthy, Feed::Healthy, Fusion::Held)
        }
        WalkBarrier::Observation => Authored::named(),
        WalkBarrier::RowSemantics => Authored::row_route(RowRoute::Capture),
        WalkBarrier::DerivedOrOrdered => Authored::grouped(),
        WalkBarrier::RequestOwnedSource => Authored::row_route(RowRoute::Rows),
        WalkBarrier::Continuation => {
            Authored::combined(Combine::Join, Feed::Healthy, Feed::Healthy, Fusion::Fused)
        }
    }
}

/// The census counter of one barrier kind.
fn counted(census: &WalkCensus, barrier: WalkBarrier) -> usize {
    match barrier {
        WalkBarrier::Representation => census.representation.0,
        WalkBarrier::DependentInput => census.dependent.0,
        WalkBarrier::Observation => census.observation.0,
        WalkBarrier::RowSemantics => census.row.0,
        WalkBarrier::DerivedOrOrdered => census.derived.0,
        WalkBarrier::RequestOwnedSource => census.request.0,
        WalkBarrier::Continuation => census.continuation.0,
    }
}

/// Every physical instruction of the compiled program is bound to a route, and a class is
/// held by shared routes only.
fn accounted(program: &Program) -> WalkCensus {
    let walk = program.prepared.walk();
    let compiled = program.prepared.program();
    let census = walk.census();
    let physical = compiled
        .regions()
        .iter()
        .flat_map(|region| region.instructions().iter())
        .collect::<Vec<_>>();
    assert_eq!(
        census.instructions.0,
        physical.len(),
        "every physical instruction is bound to a route"
    );
    for instruction in &physical {
        assert_eq!(
            walk.class_for(instruction).is_some(),
            matches!(walk.route_of(instruction), Some(WalkRoute::Shared(_))),
            "a class is held by shared routes only"
        );
    }
    assert_eq!(
        census.classes.len(),
        physical
            .iter()
            .filter_map(|instruction| walk.class_for(instruction))
            .collect::<BTreeSet<_>>()
            .len(),
        "the census classes are the classes the shared heads hold"
    );
    census
}

/// Rules 0 and 1 author the same admitted `head` in `scope`, rule 2 another predicate over
/// the same source: one class across two rules, a distinct class beside it, and per file one
/// traversal for both classes.
fn shares(scope: StepScope, head: Operation) {
    let program = program(Authored::sharing(scope, head));
    let census = accounted(&program);
    assert!(census.shared.0 > 0, "the admitted heads are shared");
    let pair = program
        .head_class(RuleIndex(0))
        .expect("the admitted head is a shared class");
    assert_eq!(
        program
            .head_class(RuleIndex(1))
            .expect("the second rule's admitted head is a shared class"),
        pair,
        "two rules authoring the same admitted head share one class"
    );
    let other = program
        .head_class(RuleIndex(2))
        .expect("the distinct selection is a shared class");
    assert_ne!(pair, other, "another predicate is another class");
    let classes = program.prepared.walk().classes();
    for class in [pair, other] {
        assert_eq!(classes[class.0].scope, scope, "a class stays in its partition");
        assert_eq!(classes[class.0].source, Relation::LanguageTarget);
    }
    match scope {
        StepScope::PerFile => {
            // Two file generations: each walks its own source once for both classes.
            for _ in 0..2 {
                let mut symbols = Symbols::default();
                let mut file = program.open(&mut symbols).unwrap();
                let read = |relation| program.read(relation);
                for rule in 0..3 {
                    assert!(
                        matches!(
                            file.rule(RuleIndex(rule), demand(), &read, &mut symbols)
                                .unwrap()
                                .emit,
                            RootOutcome::Complete(_)
                        ),
                        "every rule of the shared classes completes"
                    );
                }
                let work = file.walk_counts();
                assert_eq!(
                    work.passes.0, 1,
                    "distinct classes over one source share one traversal"
                );
                assert_eq!(work.source_rows.0, 3, "the source rows are walked once");
                file.finish().unwrap();
            }
        }
        // The project stage's one pass per population is the runtime proof of
        // `producer_lifetime::project::sharing`; here its classes are bound and partitioned.
        StepScope::CrossFile => {}
    }
}

#[test]
fn every_admitted_class_is_shared_across_rules_in_one_traversal() {
    let mut scope = after_scope(None);
    let mut cases = 0;
    while let Some(within) = scope {
        for kind in OperationKind::ALL.iter().copied() {
            if let Some(head) = admitted_head(kind) {
                shares(within, head);
                cases += 1;
            }
        }
        scope = after_scope(Some(within));
    }
    assert!(cases > 0, "at least one admitted class shape");
}

#[test]
fn every_barrier_kind_is_accounted_for() {
    let mut barrier = after_barrier(None);
    while let Some(kind) = barrier {
        let census = accounted(&program(barrier_fixture(kind)));
        assert!(
            counted(&census, kind) > 0,
            "the {kind:?} fixture holds an instruction behind its barrier"
        );
        barrier = after_barrier(Some(kind));
    }
}
