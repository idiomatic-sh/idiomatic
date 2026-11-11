//! The support of the `routes` cases.
use super::super::super::super::fixtures::{
    meaning::{Eligibility, FirstInput, Meaning, Place, Route, head_route, meaning, route},
    operations::{
        corpus::{Corpus, compiled_operations},
        heads::operations,
    },
};
use super::super::super::super::heads::expected;
use crate::cases::{Case, CaseName, Expect, Word};
use ryo_execution_domain::domain::executable_program::types::{InputSource, LogicalInstruction};
use ryo_execution_domain::domain::shared_walk::types::walk_route::value::{
    WalkBarrier, WalkRoute, source_barrier,
};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::{
    plan_parts::types::{Input as Source, ObservationBindingId, RepeatedRows, StepId},
    plans::{service::kind_of, types::Operation},
};
use std::collections::HashSet;

pub(super) fn engine(route: Option<WalkRoute>) -> Option<Route> {
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

pub(super) fn barrier(route: WalkBarrier) -> Route {
    engine(Some(WalkRoute::Barrier(route))).unwrap()
}

#[derive(Debug, PartialEq)]
pub(super) enum RouteRefusal {
    RouteMismatch,
    VariantUncovered,
}

/// Every instruction of the compiled corpus routes as its meaning says.
pub(super) fn verify_program(corpus: &Corpus) -> Result<(), RouteRefusal> {
    let shared = expected(&corpus.program);
    let mut kinds = HashSet::new();
    let mut routes = HashSet::new();
    for region in corpus.program.regions() {
        for (at, instruction) in region.instructions().iter().enumerate() {
            let place = if at > 0 {
                Place::Later
            } else if shared.contains(&instruction.logical()) {
                Place::Head(Eligibility::Eligible)
            } else {
                Place::Head(Eligibility::Ineligible)
            };
            let want = route(&corpus.program, region, instruction, place);
            if engine(corpus.walk.physical[instruction.logical().0]) != Some(want) {
                return Err(RouteRefusal::RouteMismatch);
            }
            if at == 0 {
                kinds.extend([kind_of(&instruction.operation())]);
            }
            routes.extend([want]);
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
    if kinds == all {
        Ok(())
    } else {
        Err(RouteRefusal::VariantUncovered)
    }
}

/// The classifier agrees with the meaning for every variant and every input kind.
pub(super) fn verify_classifier() -> Result<(), RouteRefusal> {
    let mut seen = HashSet::new();
    for operation in operations() {
        seen.extend([kind_of(&operation)]);
        let want = head_route(&operation, FirstInput::Ordinary, Eligibility::Ineligible);
        if barrier(source_barrier(&operation, &[])) != want {
            return Err(RouteRefusal::RouteMismatch);
        }
    }
    let selecting = |from| {
        [
            Operation::Select {
                from,
                comparisons: vec![],
                kept: vec![],
            },
            Operation::Project {
                from,
                columns: vec![],
                distinct: RepeatedRows::Kept,
            },
        ]
    };
    let observed = [InputSource::Observation(
        ryo_execution_domain::domain::executable_program::types::ObservationPlace(0),
    )];
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
        && OperationKind::ALL
            .iter()
            .all(|kind| meaning(*kind) != Meaning::RequestOwned || seen.contains(kind))
    {
        Ok(())
    } else {
        Err(RouteRefusal::VariantUncovered)
    }
}

#[derive(Clone, Copy)]
pub(super) enum RouteScenario {
    Classifier,
    Program,
    MisclassifiedExtend,
    RowsSource,
    RelabelledBarrier,
}

pub(super) const ROUTE_CASES: &[Case<RouteScenario, (), RouteRefusal>] = &[
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

pub(super) fn head_of(corpus: &Corpus, kind: OperationKind) -> LogicalInstruction {
    corpus
        .program
        .regions()
        .iter()
        .filter_map(|region| region.instructions().first())
        .find(|instruction| kind_of(&instruction.operation()) == kind)
        .expect("a head of every kind")
        .logical()
}

pub(super) fn route_subject(scenario: RouteScenario) -> Result<(), RouteRefusal> {
    match scenario {
        RouteScenario::Classifier => verify_classifier(),
        RouteScenario::Program => verify_program(&compiled_operations()),
        RouteScenario::MisclassifiedExtend => {
            let mut corpus = compiled_operations();
            let at = head_of(&corpus, OperationKind::Extend);
            corpus.walk.physical[at.0] = Some(WalkRoute::Barrier(WalkBarrier::DerivedOrOrdered));
            verify_program(&corpus)
        }
        RouteScenario::RowsSource => {
            let mut corpus = compiled_operations();
            let at = head_of(&corpus, OperationKind::Rows);
            corpus.walk.physical[at.0] = Some(WalkRoute::Barrier(WalkBarrier::RowSemantics));
            verify_program(&corpus)
        }
        RouteScenario::RelabelledBarrier => {
            let mut corpus = compiled_operations();
            let at = head_of(&corpus, OperationKind::Join);
            corpus.walk.physical[at.0] = Some(WalkRoute::Barrier(WalkBarrier::Continuation));
            verify_program(&corpus)
        }
    }
}
