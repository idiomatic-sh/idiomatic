//! The support of the `routes` cases.
use super::super::super::super::fixtures::{
    meaning::{Eligibility, FirstInput, Meaning, Place, Route, head_route, meaning, route},
    operations::{
        corpus::{Corpus, compiled_operations},
        heads::operations,
    },
};
use super::super::super::super::heads::service::expected;
use ryo_execution_domain::domain::interpreted_program::types::{InputSource, LogicalInstruction};
use ryo_execution_domain::domain::shared_walk::types::walk_route::value::{
    WalkBarrier, WalkRoute, source_barrier,
};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::{
    plan_parts::types::{Input as Source, ObservationBindingId, RepeatedRows, StepId},
    plans::{service::kind_of, types::Operation},
};
use std::borrow::Cow;
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

/// Every route, in the order a coverage names them.
const ROUTES: [Route; 8] = [
    Route::Shared,
    Route::Representation,
    Route::Dependent,
    Route::Observation,
    Route::Row,
    Route::Derived,
    Route::Request,
    Route::Continuation,
];

/// What a verified routing covered.
#[derive(Debug, PartialEq)]
pub(super) enum Covered {
    /// The classifier: how many operations it routed and how many kinds they were.
    Classifier { operations: usize, kinds: usize },
    /// The compiled corpus: how many instructions it routed, how many kinds its heads were, and
    /// the routes it took, in [`ROUTES`] order.
    Program {
        instructions: usize,
        kinds: usize,
        routes: Cow<'static, [Route]>,
    },
}

/// Every instruction of the compiled corpus routes as its meaning says.
pub(super) fn verify_program(corpus: &Corpus) -> Result<Covered, RouteRefusal> {
    let shared = expected(&corpus.program);
    let mut kinds = HashSet::new();
    let mut routes = HashSet::new();
    let mut instructions = 0;
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
            instructions += 1;
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
        Ok(Covered::Program {
            instructions,
            kinds: kinds.len(),
            routes: ROUTES
                .into_iter()
                .filter(|route| routes.contains(route))
                .collect::<Vec<_>>()
                .into(),
        })
    } else {
        Err(RouteRefusal::VariantUncovered)
    }
}

/// The classifier agrees with the meaning for every variant and every input kind.
pub(super) fn verify_classifier() -> Result<Covered, RouteRefusal> {
    let mut seen = HashSet::new();
    let catalog = operations();
    for operation in &catalog {
        seen.extend([kind_of(operation)]);
        let want = head_route(operation, FirstInput::Ordinary, Eligibility::Ineligible);
        if barrier(source_barrier(operation, &[])) != want {
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
        ryo_execution_domain::domain::interpreted_program::types::ObservationPlace(0),
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
        Ok(Covered::Classifier {
            operations: catalog.len(),
            kinds: seen.len(),
        })
    } else {
        Err(RouteRefusal::VariantUncovered)
    }
}

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

/// The compiled corpus with the head of `kind` relabelled to `barrier`, verified.
pub(super) fn relabelled(
    kind: OperationKind,
    barrier: WalkBarrier,
) -> Result<Covered, RouteRefusal> {
    let mut corpus = compiled_operations();
    let at = head_of(&corpus, kind);
    corpus.walk.physical[at.0] = Some(WalkRoute::Barrier(barrier));
    verify_program(&corpus)
}
