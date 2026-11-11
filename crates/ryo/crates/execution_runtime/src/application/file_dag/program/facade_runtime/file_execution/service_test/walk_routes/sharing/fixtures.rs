//! The support of the `sharing` cases.
use super::super::super::super::service::*;
use super::super::super::super::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::routes::{Combine, Feed, Fusion, RowRoute};
use super::super::fixtures::semantics::{Derived, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::Emit as RowSink;
use ryo_execution_domain::domain::shared_walk::types::{
    WalkCensus,
    walk_route::value::{WalkBarrier, WalkRoute},
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_parts::types::{
    ComparisonOrigin, Operand, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, StepScope},
    plans::types::Operation,
};
use std::collections::BTreeSet;
use std::sync::Arc;

pub(super) fn program(authored: Authored) -> Program {
    Program::new(
        authored,
        Population::Three,
        Residency::Stable,
        Arc::new(Meanings::default()),
        Arc::new(Derived::new()),
    )
    .expect("compile the authored sharing fixture")
}

/// The head of an admitted sharing class for each operation kind, or none when the kind never
/// heads a class. No wildcard: a new operation kind must decide whether it is admitted.
pub(super) fn admitted_head(kind: OperationKind) -> Option<Operation> {
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

/// The fixture rule set whose compiled program holds at least one instruction behind `barrier`.
pub(super) fn barrier_fixture(barrier: WalkBarrier) -> Authored {
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
pub(super) fn counted(census: &WalkCensus, barrier: WalkBarrier) -> usize {
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

/// Whether every physical instruction of a compiled program is bound to a route, a class is
/// held by shared routes only, and the census classes are the classes the shared heads hold.
#[derive(Debug, PartialEq)]
pub(super) struct Accounted {
    pub(super) every_instruction_bound: bool,
    pub(super) classes_on_shared_routes_only: bool,
    pub(super) census_classes_held: bool,
}

/// The program's walk census and its accounting.
pub(super) fn accounted(program: &Program) -> (WalkCensus, Accounted) {
    let walk = program.prepared.walk();
    let compiled = program.prepared.program();
    let census = walk.census();
    let physical = compiled
        .regions()
        .iter()
        .flat_map(|region| region.instructions().iter())
        .collect::<Vec<_>>();
    let accounting = Accounted {
        every_instruction_bound: census.instructions.0 == physical.len(),
        classes_on_shared_routes_only: physical.iter().all(|instruction| {
            walk.class_for(instruction).is_some()
                == matches!(walk.route_of(instruction), Some(WalkRoute::Shared(_)))
        }),
        census_classes_held: census.classes.len()
            == physical
                .iter()
                .filter_map(|instruction| walk.class_for(instruction))
                .collect::<BTreeSet<_>>()
                .len(),
    };
    (census, accounting)
}

/// One file generation's walk: the rules that completed, the passes and the source rows.
#[derive(Debug, PartialEq)]
pub(super) struct FileWalk {
    pub(super) completed: usize,
    pub(super) passes: u64,
    pub(super) source_rows: u64,
}

/// What two rules authoring one admitted head beside a third rule's other predicate observed.
#[derive(Debug, PartialEq)]
pub(super) struct Shared {
    pub(super) accounted: Accounted,
    pub(super) shared_heads: bool,
    pub(super) one_class_for_two_rules: bool,
    pub(super) another_class_for_another_predicate: bool,
    pub(super) scopes: [StepScope; 2],
    pub(super) sources: [Relation; 2],
    /// Per file generation, in a per-file partition; none in the project's.
    pub(super) files: Option<[FileWalk; 2]>,
}

/// Rules 0 and 1 author the same admitted `head` in `scope`, rule 2 another predicate over
/// the same source: one class across two rules, a distinct class beside it, and per file one
/// traversal for both classes.
pub(super) fn shares(scope: StepScope, head: Operation) -> Result<Shared, OperatingError> {
    let program = program(Authored::sharing(scope, head));
    let (census, accounted) = accounted(&program);
    let pair = program.head_class(RuleIndex(0))?;
    let second = program.head_class(RuleIndex(1))?;
    let other = program.head_class(RuleIndex(2))?;
    let classes = program.prepared.walk().classes();
    let files = match scope {
        StepScope::PerFile => {
            // Two file generations: each walks its own source once for both classes.
            let mut files = Vec::new();
            for _ in 0..2 {
                let mut symbols = Symbols::default();
                let mut file = program.open(&mut symbols)?;
                let read = |relation| program.read(relation);
                let mut completed = 0;
                for rule in 0..3 {
                    completed += usize::from(matches!(
                        file.rule(RuleIndex(rule), demand(), &read, &mut symbols)?.emit,
                        RootOutcome::Complete(_)
                    ));
                }
                let work = file.walk_counts();
                files.push(FileWalk {
                    completed,
                    passes: work.passes.0,
                    source_rows: work.source_rows.0,
                });
                file.finish()?;
            }
            let [first, second]: [FileWalk; 2] =
                files.try_into().map_err(|_| OperatingError::NotRun)?;
            Some([first, second])
        }
        // The project stage's one pass per population is the runtime proof of
        // `producer_lifetime::project::sharing`; here its classes are bound and partitioned.
        StepScope::CrossFile => None,
    };
    Ok(Shared {
        accounted,
        shared_heads: census.shared.0 > 0,
        one_class_for_two_rules: second == pair,
        another_class_for_another_predicate: other != pair,
        scopes: [classes[pair.0].scope, classes[other.0].scope],
        sources: [classes[pair.0].source, classes[other.0].source],
        files,
    })
}
