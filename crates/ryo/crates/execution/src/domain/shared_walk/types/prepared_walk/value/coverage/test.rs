//! Expected eligibility never reads prepared classes, routes, census or trace events.
// ignore: idiomatic.arch/file-length - the route controls share one corpus and one refusal vocabulary with the head controls; splitting would duplicate both
use super::fixtures::{
    self, Eligibility, FirstInput, Meaning, Place, Route, head_route, meaning, operations,
};
use super::super::{PreparedWalk, WalkBarrier, WalkRoute, source_barrier};
use crate::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::executable_program::types::{
        ExecutableProgram, Instruction, InputSource, LogicalInstruction,
    },
};
use ryo_relations::domain::relations::{
    service::{ALL, columns},
    types::{Column, Relation},
};
use ryo_values::domain::{
    observations::types::ObservationBindingId,
    plan_parts::types::{Input as Source, Operand, RepeatedRows, StepId},
    plans::{
        service::kind_of,
        types::{Operation, OperationKind, StepScope},
    },
};
use std::collections::HashSet;

// Every catalog relation has a typed borrowed-row representation. Source loading
// and demand availability are runtime obligations, not static head eligibility.
pub(super) fn supported_head(instruction: &Instruction) -> Option<Relation> {
    let (source, projected, predicates) = match &instruction.operation {
        Operation::Select {
            from: Source::Relation(source),
            kept,
            comparisons,
        } => (*source, kept, comparisons.as_slice()),
        Operation::Project {
            from: Source::Relation(source),
            columns,
            distinct: RepeatedRows::Kept,
        } => (*source, columns, &[][..]),
        _ => return None,
    };
    let source_columns = columns(source);
    let width = source_columns.len();
    fn valid(operand: &Operand, columns: &[Column]) -> Option<()> {
        match operand {
            Operand::Column(column) => columns.get(column.0 as usize).map(|_| ()),
            Operand::Function(function) => function
                .arguments
                .iter()
                .try_for_each(|argument| valid(argument, columns)),
            Operand::Parameter(_) | Operand::Literal(_) => Some(()),
        }
    }
    (ALL.contains(&source)
        && projected.iter().all(|column| (column.0 as usize) < width)
        && predicates.iter().all(|predicate| {
            valid(&predicate.left, &source_columns).is_some()
                && valid(&predicate.right, &source_columns).is_some()
        }))
    .then_some(source)
}
pub(super) fn expected(program: &ExecutableProgram) -> HashSet<LogicalInstruction> {
    program
        .regions()
        .iter()
        .filter(|region| region.phase == StepScope::PerFile)
        .filter_map(|region| region.instructions.first())
        .filter(|instruction| supported_head(instruction).is_some())
        .map(|instruction| instruction.logical)
        .collect()
}
#[derive(Debug, PartialEq)]
enum Refusal {
    EligibleHeadMissing,
    IneligibleHeadShared,
}
fn verify(program: &ExecutableProgram, walk: &PreparedWalk) -> Result<(), Refusal> {
    let expected = expected(program);
    let actual = program
        .regions()
        .iter()
        .flat_map(|region| &region.instructions)
        .filter(|instruction| walk.class_for(instruction).is_some())
        .map(|instruction| instruction.logical)
        .collect::<HashSet<_>>();
    if !expected.is_subset(&actual) {
        return Err(Refusal::EligibleHeadMissing);
    }
    if actual != expected {
        return Err(Refusal::IneligibleHeadShared);
    }
    Ok(())
}
fn internally_consistent(walk: &PreparedWalk) {
    let census = walk.census();
    assert_eq!(
        census.instructions.0,
        census.shared.0
            + census.representation.0
            + census.dependent.0
            + census.observation.0
            + census.row.0
            + census.derived.0
            + census.request.0
            + census.continuation.0
    );
    assert_eq!(
        census.shared.0,
        walk.physical
            .iter()
            .filter(|route| matches!(route, Some(WalkRoute::Shared(_))))
            .count()
    );
}
#[derive(Clone, Copy)]
enum Scenario {
    Catalog,
    Misclassified,
    Removed,
    ProjectPhase,
}
type Input = Scenario;
type Value = ();
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every eligible physical catalog head is classified independently"),
        word: Word::Succeeds,
        input: || Scenario::Catalog,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "a consistent census cannot conceal an eligible head relabelled as a barrier",
        ),
        word: Word::Rejects,
        input: || Scenario::Misclassified,
        expect: Expect::Error(Refusal::EligibleHeadMissing),
    },
    Case {
        name: CaseName("a consistent reduced census cannot conceal a removed eligible head"),
        word: Word::Rejects,
        input: || Scenario::Removed,
        expect: Expect::Error(Refusal::EligibleHeadMissing),
    },
    Case {
        name: CaseName("a consistent census cannot classify a project phase head as file work"),
        word: Word::Rejects,
        input: || Scenario::ProjectPhase,
        expect: Expect::Error(Refusal::IneligibleHeadShared),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    let super::fixtures::CompiledCorpus { program, mut walk } = super::fixtures::compiled();
    let eligible = expected(&program);
    assert!(!eligible.is_empty());
    for relation in ALL {
        assert!(
            program
                .regions()
                .iter()
                .filter(|region| region.phase == StepScope::PerFile)
                .filter_map(|region| region.instructions.first())
                .any(|head| supported_head(head) == Some(*relation)),
            "catalog source absent: {relation:?}"
        );
    }
    for relation in ALL {
        let heads = || {
            program
                .regions()
                .iter()
                .filter(|region| region.phase == StepScope::PerFile)
                .filter_map(|region| region.instructions.first())
        };
        assert!(heads().any(|head| matches!(&head.operation, Operation::Select { from: Source::Relation(source), .. } if source == relation)));
        assert!(heads().any(|head| matches!(&head.operation, Operation::Project { from: Source::Relation(source), distinct: RepeatedRows::Kept, columns } if source == relation && columns.iter().all(|column| (column.0 as usize) < ryo_relations::domain::relations::service::columns(*source).len()))));
        assert!(heads().any(|head| matches!(&head.operation, Operation::Project { from: Source::Relation(source), distinct: RepeatedRows::Dropped, .. } if source == relation)));
    }
    assert!(
        program
            .regions()
            .iter()
            .any(|region| region.phase == StepScope::CrossFile)
    );
    assert!(
        program
            .regions()
            .iter()
            .any(|region| region.instructions.len() > 1)
    );
    assert!(
        program
            .regions()
            .iter()
            .filter_map(|region| region.instructions.first())
            .any(|head| matches!(
                &head.operation,
                Operation::Select {
                    from: Source::Step(_),
                    ..
                }
            ))
    );
    assert!(
        walk.census().dependent.0 > 0,
        "ordinary normalized Step inputs retain their dependent barrier"
    );
    verify(&program, &walk).unwrap();
    let logical = *eligible.iter().next().unwrap();
    match scenario {
        Scenario::Catalog => {}
        Scenario::Misclassified => {
            walk.physical[logical.0] = Some(WalkRoute::Barrier(WalkBarrier::Representation))
        }
        Scenario::Removed => walk.physical[logical.0] = None,
        Scenario::ProjectPhase => {
            let project = program
                .regions()
                .iter()
                .find(|region| region.phase == StepScope::CrossFile)
                .unwrap()
                .instructions
                .first()
                .unwrap();
            walk.physical[project.logical.0] = walk.physical[logical.0];
        }
    }
    internally_consistent(&walk);
    verify(&program, &walk)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn rejects() {
    run(CASES, Word::Rejects, subject);
}

fn engine(route: Option<WalkRoute>) -> Option<Route> {
    Some(match route? {
        WalkRoute::Shared(_) => Route::Shared,
        WalkRoute::Barrier(WalkBarrier::Representation) => Route::Representation,
        WalkRoute::Barrier(WalkBarrier::DependentInput) => Route::Dependent,
        WalkRoute::Barrier(WalkBarrier::Observation) => Route::Observation,
        WalkRoute::Barrier(WalkBarrier::RowSemantics) => Route::Row,
        WalkRoute::Barrier(WalkBarrier::DerivedOrOrdered) => Route::Derived,
        WalkRoute::Barrier(WalkBarrier::RequestOwnedSource) => Route::Request,
        WalkRoute::Barrier(WalkBarrier::Continuation) => Route::Continuation,
    })
}
fn barrier(route: WalkBarrier) -> Route {
    engine(Some(WalkRoute::Barrier(route))).unwrap()
}

#[derive(Debug, PartialEq)]
enum RouteRefusal {
    RouteMismatch,
    VariantUncovered,
}

/// Every instruction of the compiled corpus routes as its meaning says.
fn verify_program(corpus: &fixtures::Corpus) -> Result<(), RouteRefusal> {
    let shared = expected(&corpus.program);
    let mut kinds = HashSet::new();
    let mut routes = HashSet::new();
    for region in corpus.program.regions() {
        for (at, instruction) in region.instructions.iter().enumerate() {
            let place = if at > 0 {
                Place::Later
            } else if shared.contains(&instruction.logical) {
                Place::Head(Eligibility::Eligible)
            } else {
                Place::Head(Eligibility::Ineligible)
            };
            let want = fixtures::route(&corpus.program, region, instruction, place);
            if engine(corpus.walk.physical[instruction.logical.0]) != Some(want) {
                return Err(RouteRefusal::RouteMismatch);
            }
            if at == 0 {
                kinds.insert(kind_of(&instruction.operation));
            }
            routes.insert(want);
        }
    }
    let all = OperationKind::ALL.iter().copied().collect::<HashSet<_>>();
    for needed in [
        Route::Shared,
        Route::Representation,
        Route::Dependent,
        Route::Row,
        Route::Derived,
        Route::Request,
        Route::Continuation,
    ] {
        if !routes.contains(&needed) {
            return Err(RouteRefusal::VariantUncovered);
        }
    }
    if kinds == all { Ok(()) } else { Err(RouteRefusal::VariantUncovered) }
}

/// The classifier agrees with the meaning for every variant and every input kind.
fn verify_classifier() -> Result<(), RouteRefusal> {
    let mut seen = HashSet::new();
    for operation in operations() {
        seen.insert(kind_of(&operation));
        let want = head_route(&operation, FirstInput::Ordinary, Eligibility::Ineligible);
        if barrier(source_barrier(&operation, &[])) != want {
            return Err(RouteRefusal::RouteMismatch);
        }
    }
    let selecting = |from| {
        [
            Operation::Select { from, comparisons: vec![], kept: vec![] },
            Operation::Project { from, columns: vec![], distinct: RepeatedRows::Kept },
        ]
    };
    let observed = [InputSource::Observation(0)];
    for operation in selecting(Source::Step(StepId(0))) {
        if barrier(source_barrier(&operation, &[])) != Route::Dependent
            || barrier(source_barrier(&operation, &observed)) != Route::Observation
        {
            return Err(RouteRefusal::RouteMismatch);
        }
    }
    for operation in selecting(Source::BoundObservation(ObservationBindingId(0))) {
        if barrier(source_barrier(&operation, &[])) != Route::Observation {
            return Err(RouteRefusal::RouteMismatch);
        }
    }
    if seen.len() == OperationKind::ALL.len()
        && OperationKind::ALL.iter().all(|kind| meaning(*kind) != Meaning::RequestOwned || seen.contains(kind))
    {
        Ok(())
    } else {
        Err(RouteRefusal::VariantUncovered)
    }
}

#[derive(Clone, Copy)]
enum RouteScenario {
    Classifier,
    Program,
    MisclassifiedExtend,
    RowsSource,
    RelabelledBarrier,
}
const ROUTE_CASES: &[Case<RouteScenario, (), Refusal>] = &[
    Case {
        name: CaseName("every variant's classifier route matches its meaning"),
        word: Word::Succeeds,
        input: || RouteScenario::Classifier,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("every instruction of the compiled corpus routes as its meaning says"),
        word: Word::Succeeds,
        input: || RouteScenario::Program,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an extend head relabelled as a derived barrier is refused"),
        word: Word::Rejects,
        input: || RouteScenario::MisclassifiedExtend,
        expect: Expect::Error(RouteRefusal::RouteMismatch),
    },
    Case {
        name: CaseName("a rows source relabelled as row semantics is refused"),
        word: Word::Rejects,
        input: || RouteScenario::RowsSource,
        expect: Expect::Error(RouteRefusal::RouteMismatch),
    },
    Case {
        name: CaseName("a join head relabelled as a continuation is refused"),
        word: Word::Rejects,
        input: || RouteScenario::RelabelledBarrier,
        expect: Expect::Error(RouteRefusal::RouteMismatch),
    },
];
fn head_of(corpus: &fixtures::Corpus, kind: OperationKind) -> LogicalInstruction {
    corpus
        .program
        .regions()
        .iter()
        .filter_map(|region| region.instructions.first())
        .find(|instruction| kind_of(&instruction.operation) == kind)
        .expect("a head of every kind")
        .logical
}
fn route_subject(scenario: RouteScenario) -> Result<(), RouteRefusal> {
    match scenario {
        RouteScenario::Classifier => verify_classifier(),
        RouteScenario::Program => verify_program(&fixtures::compiled_operations()),
        RouteScenario::MisclassifiedExtend => {
            let mut corpus = fixtures::compiled_operations();
            let at = head_of(&corpus, OperationKind::Extend);
            corpus.walk.physical[at.0] = Some(WalkRoute::Barrier(WalkBarrier::DerivedOrOrdered));
            verify_program(&corpus)
        }
        RouteScenario::RowsSource => {
            let mut corpus = fixtures::compiled_operations();
            let at = head_of(&corpus, OperationKind::Rows);
            corpus.walk.physical[at.0] = Some(WalkRoute::Barrier(WalkBarrier::RowSemantics));
            verify_program(&corpus)
        }
        RouteScenario::RelabelledBarrier => {
            let mut corpus = fixtures::compiled_operations();
            let at = head_of(&corpus, OperationKind::Join);
            corpus.walk.physical[at.0] = Some(WalkRoute::Barrier(WalkBarrier::Continuation));
            verify_program(&corpus)
        }
    }
}
#[test]
fn routes_succeed() {
    run(ROUTE_CASES, Word::Succeeds, route_subject);
}
#[test]
fn routes_reject() {
    run(ROUTE_CASES, Word::Rejects, route_subject);
}

/// Guard: `meaning` and `kind_of` are exhaustive without a wildcard, so a 27th variant must
/// choose a route to compile; this asserts the catalog and the corpus agree on the count.
#[test]
fn every_catalog_kind_has_a_head_in_the_corpus() {
    let kinds = operations().iter().map(kind_of).collect::<HashSet<_>>();
    assert_eq!(kinds.len(), operations().len(), "one operation per kind");
    assert_eq!(kinds.len(), OperationKind::ALL.len());
}
