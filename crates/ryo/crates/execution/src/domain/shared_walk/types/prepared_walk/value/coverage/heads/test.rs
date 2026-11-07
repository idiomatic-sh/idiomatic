//! Expected eligibility never reads prepared classes, routes, census or trace events.
use super::super::fixtures::catalog::{CompiledCorpus, compiled, prepare};
use super::super::fixtures::plan;
use super::{expected, supported_head};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::shared_walk::types::{
    prepared_walk::value::PreparedWalk,
    walk_route::value::{WalkBarrier, WalkRoute},
};
use crate::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::executable_program::types::ExecutableProgram,
};
use ryo_relations::domain::relations::service::ALL;
use ryo_values::domain::{
    plan_parts::types::{Input as Source, RepeatedRows, StepScope},
    plans::types::Operation,
};
use std::collections::HashSet;

#[derive(Debug, PartialEq)]
enum Refusal {
    EligibleHeadMissing,
    IneligibleHeadShared,
    ErroredOriginDropped,
    SiblingRefused,
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
    ErroredOrigin,
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
        name: CaseName(
            "a consistent census cannot conceal a project phase head relabelled as a barrier",
        ),
        word: Word::Rejects,
        input: || Scenario::ProjectPhase,
        expect: Expect::Error(Refusal::EligibleHeadMissing),
    },
    Case {
        name: CaseName("an errored origin refuses only its own rule; a sibling rule still shares"),
        word: Word::Succeeds,
        input: || Scenario::ErroredOrigin,
        expect: Expect::Value(()),
    },
];
/// A rule step beyond its prepared operator contracts has an errored origin. Retention leaves
/// only that rule out of sharing; the sibling rule keeps its class and its shared head.
fn errored_origin_refuses_its_rule() -> Result<Value, Failure> {
    let select = || Operation::Select {
        from: Source::Relation(ALL[0]),
        comparisons: vec![],
        kept: vec![],
    };
    let one = plan(select(), StepScope::PerFile);
    let mut two = plan(select(), StepScope::PerFile);
    two.steps.push(ryo_values::domain::plans::types::PlanStep {
        operation: select(),
        scope: StepScope::PerFile,
    });
    let (program, mut walk) = prepare(&[one.clone(), two], &[one.clone(), one]);
    if walk.retain_program_consumers(&program).is_err() {
        return Err(Refusal::SiblingRefused);
    }
    if !program.rules()[1].original_steps.iter().any(Result::is_err) {
        return Err(Refusal::ErroredOriginDropped);
    }
    let shared = |rule: usize| {
        walk.class_of(RuleIndex(rule))
            .is_some_and(|classes| classes.iter().any(Option::is_some))
    };
    // The errored step joins no class. Retention is stricter than this control: `resolve_class` drops the whole errored rule from
    // sharing, and the control asserts only the errored step.
    let errored_step_shared =
        program.rules()[1]
            .original_steps
            .iter()
            .enumerate()
            .any(|(at, origin)| {
                origin.is_err()
                    && walk
                        .class_of(RuleIndex(1))
                        .and_then(|classes| classes.get(at))
                        .is_some_and(Option::is_some)
            });
    if errored_step_shared {
        return Err(Refusal::ErroredOriginDropped);
    }
    let sibling_head = program.rules()[0]
        .original_steps
        .iter()
        .flatten()
        .next()
        .and_then(|origin| program.logical_regions().get(origin.logical.0))
        .and_then(|region| program.regions().get(region.0))
        .and_then(|region| region.instructions.first());
    match sibling_head {
        Some(head) if shared(0) && walk.class_for(head).is_some() => Ok(()),
        _ => Err(Refusal::SiblingRefused),
    }
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    if matches!(scenario, Scenario::ErroredOrigin) {
        return errored_origin_refuses_its_rule();
    }
    let CompiledCorpus { program, mut walk } = compiled();
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
        Scenario::Catalog | Scenario::ErroredOrigin => {}
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
            walk.physical[project.logical.0] =
                Some(WalkRoute::Barrier(WalkBarrier::Representation));
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
