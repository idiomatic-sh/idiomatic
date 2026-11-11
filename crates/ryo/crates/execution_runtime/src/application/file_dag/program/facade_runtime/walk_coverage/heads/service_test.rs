//! Expected eligibility never reads prepared classes, routes, census or trace events.
use super::super::fixtures::catalog::{CompiledCorpus, compiled, prepare};
use super::super::fixtures::plan;
use super::{service::expected, service::supported_head};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram;
use ryo_execution_domain::domain::shared_walk::types::{
    prepared_walk::value::PreparedWalk,
    walk_route::value::{WalkBarrier, WalkRoute},
};
use ryo_relations::domain::relations::service::ALL;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::{
    plan_parts::types::{Input as Source, RepeatedRows, StepScope},
    plans::types::Operation,
};
use std::borrow::Cow;
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
        .flat_map(|region| region.instructions())
        .filter(|instruction| walk.class_for(instruction).is_some())
        .map(|instruction| instruction.logical())
        .collect::<HashSet<_>>();
    if !expected.is_subset(&actual) {
        return Err(Refusal::EligibleHeadMissing);
    }
    if actual != expected {
        return Err(Refusal::IneligibleHeadShared);
    }
    Ok(())
}
/// Whether the census's classes add up to its instructions and its shared count is the shared routes.
fn internally_consistent(walk: &PreparedWalk) -> bool {
    let census = walk.census();
    census.instructions.0
        == census.shared.0
            + census.representation.0
            + census.dependent.0
            + census.observation.0
            + census.row.0
            + census.derived.0
            + census.request.0
            + census.continuation.0
        && census.shared.0
            == walk
                .physical
                .iter()
                .filter(|route| matches!(route, Some(WalkRoute::Shared(_))))
                .count()
}

/// What a row does to the catalog corpus's prepared walk before it is verified.
#[derive(Clone, Copy, Debug)]
enum Tamper {
    /// Leaves the walk as prepared.
    Nothing,
    /// Relabels the first eligible head as a representation barrier.
    RelabelEligibleHead,
    /// Removes the first eligible head's route.
    RemoveEligibleHead,
    /// Relabels the first project-phase head as a representation barrier.
    RelabelProjectHead,
}

/// The program a row verifies.
#[derive(Clone, Copy, Debug)]
enum Corpus {
    /// The compiled catalog corpus, tampered with as stated.
    Catalog(Tamper),
    /// Two rules over the first relation, the second with a step beyond its operator contracts.
    ErroredOrigin,
}

/// What the catalog corpus holds before tampering, and whether the tampered census is consistent.
#[derive(Debug, PartialEq)]
struct Covered {
    /// Whether some head is expected to be eligible.
    eligible: bool,
    /// The relations no per-file head reads, and those missing a select, a kept or a dropped project head.
    unread: Cow<'static, [Relation]>,
    unshaped: Cow<'static, [Relation]>,
    /// Whether a cross-file region, a multi-instruction region and a step-input select exist.
    cross_file: bool,
    multi_instruction: bool,
    step_select: bool,
    /// Whether ordinary step inputs keep their dependent barrier.
    dependent: bool,
    /// Whether the untampered walk verifies.
    verified: bool,
    /// Whether the tampered census is internally consistent.
    consistent: bool,
}

/// The catalog's coverage, or nothing for the errored-origin control.
type Value = Option<Covered>;
type Input = Corpus;
/// The refusal, and whether the tampered census was internally consistent (catalog rows only).
#[derive(Debug, PartialEq)]
struct Rejected {
    refusal: Refusal,
    consistent: Option<bool>,
}

type Failure = Rejected;

const fn concealed(refusal: Refusal) -> Rejected {
    Rejected {
        refusal,
        consistent: Some(true),
    }
}

/// The prepared catalog corpus, which tampering leaves internally consistent.
const CATALOG: Covered = Covered {
    eligible: true,
    unread: Cow::Borrowed(&[]),
    unshaped: Cow::Borrowed(&[]),
    cross_file: true,
    multi_instruction: true,
    step_select: true,
    dependent: true,
    verified: true,
    consistent: true,
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every eligible physical catalog head is classified independently"),
        word: Word::Succeeds,
        input: || Corpus::Catalog(Tamper::Nothing),
        expect: Expect::Value(Some(CATALOG)),
    },
    Case {
        name: CaseName(
            "a consistent census cannot conceal an eligible head relabelled as a barrier",
        ),
        word: Word::Rejects,
        input: || Corpus::Catalog(Tamper::RelabelEligibleHead),
        expect: Expect::Error(concealed(Refusal::EligibleHeadMissing)),
    },
    Case {
        name: CaseName("a consistent reduced census cannot conceal a removed eligible head"),
        word: Word::Rejects,
        input: || Corpus::Catalog(Tamper::RemoveEligibleHead),
        expect: Expect::Error(concealed(Refusal::EligibleHeadMissing)),
    },
    Case {
        name: CaseName(
            "a consistent census cannot conceal a project phase head relabelled as a barrier",
        ),
        word: Word::Rejects,
        input: || Corpus::Catalog(Tamper::RelabelProjectHead),
        expect: Expect::Error(concealed(Refusal::EligibleHeadMissing)),
    },
    Case {
        name: CaseName("an errored origin refuses only its own rule; a sibling rule still shares"),
        word: Word::Succeeds,
        input: || Corpus::ErroredOrigin,
        expect: Expect::Value(None),
    },
];
/// A rule step beyond its prepared operator contracts has an errored origin. Retention leaves
/// only that rule out of sharing; the sibling rule keeps its class and its shared head.
fn errored_origin_refuses_its_rule() -> Result<(), Refusal> {
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
    walk.retain_program_consumers(&program)
        .map_err(|_| Refusal::SiblingRefused)?;
    if !program.rules()[1]
        .original_steps
        .iter()
        .any(|origin| origin.as_ref().err().is_some())
    {
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
                origin.as_ref().err().is_some()
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
        .and_then(|region| region.instructions().first());
    match sibling_head {
        Some(head) if shared(0) && walk.class_for(head).is_some() => Ok(()),
        _ => Err(Refusal::SiblingRefused),
    }
}
/// Verifies the corpus's prepared walk, the catalog after its tampering.
fn subject(corpus: Input) -> Result<Value, Failure> {
    let Corpus::Catalog(tamper) = corpus else {
        return errored_origin_refuses_its_rule()
            .map(|()| None)
            .map_err(|refusal| Rejected {
                refusal,
                consistent: None,
            });
    };
    let CompiledCorpus { program, mut walk } = compiled();
    let eligible = expected(&program);
    let heads = || {
        program
            .regions()
            .iter()
            .filter(|region| region.phase() == StepScope::PerFile)
            .filter_map(|region| region.instructions().first())
    };
    let unread: Vec<Relation> = ALL
        .iter()
        .copied()
        .filter(|relation| !heads().any(|head| supported_head(head) == Some(*relation)))
        .collect();
    let unshaped: Vec<Relation> = ALL
        .iter()
        .copied()
        .filter(|relation| {
            let selected = heads().any(|head| matches!(&head.operation(), Operation::Select { from: Source::Relation(source), .. } if source == relation));
            let kept = heads().any(|head| matches!(&head.operation(), Operation::Project { from: Source::Relation(source), distinct: RepeatedRows::Kept, columns } if source == relation && columns.iter().all(|column| (column.0 as usize) < ryo_relations::domain::relations::service::columns(*source).len())));
            let dropped = heads().any(|head| matches!(&head.operation(), Operation::Project { from: Source::Relation(source), distinct: RepeatedRows::Dropped, .. } if source == relation));
            !(selected && kept && dropped)
        })
        .collect();
    let regions = program.regions();
    let mut covered = Covered {
        eligible: !eligible.is_empty(),
        unread: unread.into(),
        unshaped: unshaped.into(),
        cross_file: regions
            .iter()
            .any(|region| region.phase() == StepScope::CrossFile),
        multi_instruction: regions.iter().any(|region| region.instructions().len() > 1),
        step_select: regions
            .iter()
            .filter_map(|region| region.instructions().first())
            .any(|head| {
                matches!(
                    &head.operation(),
                    Operation::Select {
                        from: Source::Step(_),
                        ..
                    }
                )
            }),
        dependent: walk.census().dependent.0 > 0,
        verified: verify(&program, &walk).err().is_none(),
        consistent: false,
    };
    let first_eligible = eligible.iter().next().copied();
    let project_head = regions
        .iter()
        .find(|region| region.phase() == StepScope::CrossFile)
        .and_then(|region| region.instructions().first())
        .map(|head| head.logical());
    let target = match tamper {
        Tamper::Nothing => None,
        Tamper::RelabelEligibleHead | Tamper::RemoveEligibleHead => first_eligible,
        Tamper::RelabelProjectHead => project_head,
    };
    if let Some(logical) = target {
        walk.physical[logical.0] = match tamper {
            Tamper::RemoveEligibleHead => None,
            _ => Some(WalkRoute::Barrier(WalkBarrier::Representation)),
        };
    }
    covered.consistent = internally_consistent(&walk);
    verify(&program, &walk)
        .map(|()| Some(covered))
        .map_err(|refusal| Rejected {
            refusal,
            consistent: Some(internally_consistent(&walk)),
        })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn rejects() {
    run(CASES, Word::Rejects, subject);
}
