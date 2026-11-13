//! The engine's sharing contract, language-free: every admitted sharing class is shared by two
//! rules and walks one traversal beside a distinct class, and every barrier kind is accounted
//! for. The fixtures match the engine's own enums exhaustively, so a new class shape or barrier
//! fails to compile until it is decided there, and then has its row here.
use super::fixtures::{
    Accounted, GenerationRun, Shared, accounted, admitted_head, barrier_fixture, counted, program,
    shares,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::shared_walk::types::walk_route::value::WalkBarrier;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::query_operators::types::OperationKind;

/// What the sharing contract is asked: an admitted class head of a kind in a partition, or the
/// fixture holding an instruction behind a barrier.
#[derive(Clone, Copy, Debug)]
enum Sharing {
    Class(StepScope, OperationKind),
    Barrier(WalkBarrier),
}

/// What the compiled program and its files observed.
#[derive(Debug, PartialEq)]
enum Observed {
    Shared(Shared),
    /// The accounting, and the instructions the census counts behind the barrier.
    Barrier {
        accounted: Accounted,
        behind: usize,
    },
}

const ACCOUNTED: Accounted = Accounted {
    every_instruction_bound: true,
    classes_on_shared_routes_only: true,
    census_classes_held: true,
};

/// Two rules share one admitted class beside a distinct class in `scope`, over the source;
/// per file, all three rules complete in one traversal of its three rows.
const fn shared(scope: StepScope) -> Shared {
    Shared {
        accounted: ACCOUNTED,
        shared_heads: true,
        one_class_for_two_rules: true,
        another_class_for_another_predicate: true,
        scopes: [scope, scope],
        sources: [Relation::LanguageTarget, Relation::LanguageTarget],
        files: match scope {
            StepScope::PerFile => Some([
                GenerationRun {
                    completed: 3,
                    passes: 1,
                    source_rows: 3,
                },
                GenerationRun {
                    completed: 3,
                    passes: 1,
                    source_rows: 3,
                },
            ]),
            StepScope::CrossFile => None,
        },
    }
}

type Input = Sharing;
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_admitted_class_is_shared_across_rules_in_one_traversal"),
        word: Word::Succeeds,
        input: || Sharing::Class(StepScope::PerFile, OperationKind::Select),
        expect: Expect::Value(Observed::Shared(shared(StepScope::PerFile))),
    },
    Case {
        name: CaseName("an_admitted_projection_is_shared_across_rules_in_one_traversal"),
        word: Word::Succeeds,
        input: || Sharing::Class(StepScope::PerFile, OperationKind::Project),
        expect: Expect::Value(Observed::Shared(shared(StepScope::PerFile))),
    },
    Case {
        name: CaseName("an_admitted_project_selection_is_shared_across_rules"),
        word: Word::Succeeds,
        input: || Sharing::Class(StepScope::CrossFile, OperationKind::Select),
        expect: Expect::Value(Observed::Shared(shared(StepScope::CrossFile))),
    },
    Case {
        name: CaseName("an_admitted_project_projection_is_shared_across_rules"),
        word: Word::Succeeds,
        input: || Sharing::Class(StepScope::CrossFile, OperationKind::Project),
        expect: Expect::Value(Observed::Shared(shared(StepScope::CrossFile))),
    },
    Case {
        name: CaseName("every_barrier_kind_is_accounted_for"),
        word: Word::Succeeds,
        input: || Sharing::Barrier(WalkBarrier::Representation),
        expect: Expect::Value(Observed::Barrier {
            accounted: ACCOUNTED,
            behind: 1,
        }),
    },
    Case {
        name: CaseName("the_dependent_input_barrier_is_accounted_for"),
        word: Word::Succeeds,
        input: || Sharing::Barrier(WalkBarrier::DependentInput),
        expect: Expect::Value(Observed::Barrier {
            accounted: ACCOUNTED,
            behind: 4,
        }),
    },
    Case {
        name: CaseName("the_observation_barrier_is_accounted_for"),
        word: Word::Succeeds,
        input: || Sharing::Barrier(WalkBarrier::Observation),
        expect: Expect::Value(Observed::Barrier {
            accounted: ACCOUNTED,
            behind: 2,
        }),
    },
    Case {
        name: CaseName("the_row_semantics_barrier_is_accounted_for"),
        word: Word::Succeeds,
        input: || Sharing::Barrier(WalkBarrier::RowSemantics),
        expect: Expect::Value(Observed::Barrier {
            accounted: ACCOUNTED,
            behind: 1,
        }),
    },
    Case {
        name: CaseName("the_derived_or_ordered_barrier_is_accounted_for"),
        word: Word::Succeeds,
        input: || Sharing::Barrier(WalkBarrier::DerivedOrOrdered),
        expect: Expect::Value(Observed::Barrier {
            accounted: ACCOUNTED,
            behind: 1,
        }),
    },
    Case {
        name: CaseName("the_request_owned_source_barrier_is_accounted_for"),
        word: Word::Succeeds,
        input: || Sharing::Barrier(WalkBarrier::RequestOwnedSource),
        expect: Expect::Value(Observed::Barrier {
            accounted: ACCOUNTED,
            behind: 1,
        }),
    },
    Case {
        name: CaseName("the_continuation_barrier_is_accounted_for"),
        word: Word::Succeeds,
        input: || Sharing::Barrier(WalkBarrier::Continuation),
        expect: Expect::Value(Observed::Barrier {
            accounted: ACCOUNTED,
            behind: 2,
        }),
    },
];

/// Compiles and runs the sharing fixture of the class, or the barrier's fixture.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Sharing::Class(scope, kind) => {
            let head = admitted_head(kind).ok_or(OperatingError::WrongOperation)?;
            shares(scope, head).map(Observed::Shared)
        }
        Sharing::Barrier(barrier) => {
            let (census, accounted) = accounted(&program(barrier_fixture(barrier)));
            Ok(Observed::Barrier {
                accounted,
                behind: counted(&census, barrier),
            })
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
