//! Request-graph dependencies: the producers a rule may read, their schemas and the plans linked beneath them.
use std::borrow::Cow;

use super::fixtures::{binding, bound_input, column, export, exporting, observing, plan, rule};
use super::service::link::{service::following, service::link};
use super::service::{schedule, schemas};
use super::types::{ResolvedObservationBinding, ResolvedObservationProducer, RuleSchedule};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::rule_intent::errors::RuleIntentError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::observations::types::{
    ObservationName, ObservationProducer, PlanObservation,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as PlanInput, ObservationBindingId, RepeatedRows, StepId, StepScope,
};
use ryo_values::domain::plans::types::{Operation, Plan, PlanStep};
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::selects;

/// What a row asks of the request graph.
enum Ask {
    /// The order the rules of a request run in.
    Schedule(Vec<RuleSpec>),
    /// The schemas the second rule of a pair reads, the producers compiled to these plans.
    Schemas([RuleSpec; 2], Vec<Option<Plan>>),
    /// The step number after this one.
    Following(StepId),
    /// The plans linked beneath their bindings.
    Link(Vec<Plan>, Vec<Vec<ResolvedObservationBinding>>),
    /// Whether each plan linked beneath its bindings is the plan given, shared, or a new one.
    Shared(Vec<Plan>, Vec<Vec<ResolvedObservationBinding>>),
    /// The steps the first plan linked beneath its bindings keeps, and the room its step list holds.
    Pruned(Vec<Plan>, Vec<Vec<ResolvedObservationBinding>>),
}

/// Whether a linked plan is the plan it was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kept {
    /// The plan given, shared.
    Shared,
    /// A new plan.
    New,
}

/// What the request graph answers.
#[derive(Debug, PartialEq)]
enum Answer {
    /// The rules run in their own order, independently.
    Independent,
    /// The rules in dependency order, and the rules of each ready batch.
    Scheduled {
        order: Cow<'static, [usize]>,
        batches: Cow<'static, [Cow<'static, [usize]>]>,
    },
    /// The schemas were prepared.
    Prepared,
    /// The next step number.
    Next(StepId),
    /// The plans were linked.
    Linked,
    /// Whether each linked plan is the one given.
    Plans(Cow<'static, [Kept]>),
    /// The steps a linked plan keeps, and the room its step list holds.
    Steps { kept: usize, room: usize },
}

type Input = Ask;
type Value = Answer;
type Failure = RuleIntentError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_consumer_reads_the_export_of_an_earlier_producer"),
        word: Word::Succeeds,
        input: || Ask::Schedule(pair("first").into()),
        expect: Expect::Value(Answer::Scheduled {
            order: Cow::Borrowed(&[0, 1]),
            batches: Cow::Borrowed(&[Cow::Borrowed(&[0]), Cow::Borrowed(&[1])]),
        }),
    },
    Case {
        name: CaseName("an_unlisted_producer_is_unknown"),
        word: Word::Refuses,
        input: || Ask::Schedule(pair("absent").into()),
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, RuleIntentError::UnknownProducer { .. }))
        }),
    },
    Case {
        name: CaseName("a_rule_cannot_read_its_own_export"),
        word: Word::Refuses,
        input: || {
            Ask::Schedule(vec![observing(
                "only",
                vec![binding("rows", vec![export("only")])],
            )])
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, RuleIntentError::SelfDependency { .. }))
        }),
    },
    Case {
        name: CaseName("a_listed_rule_cannot_be_declared_without_a_query"),
        word: Word::Refuses,
        input: || {
            Ask::Schedule(vec![observing(
                "only",
                vec![binding(
                    "rows",
                    vec![ObservationProducer::NoQuery {
                        rule: RuleId("only".into()),
                    }],
                )],
            )])
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, RuleIntentError::ExecutableNoQuery { .. }))
        }),
    },
    Case {
        name: CaseName("a_name_is_bound_once_per_rule"),
        word: Word::Refuses,
        input: || {
            Ask::Schedule(vec![observing(
                "only",
                vec![binding("rows", vec![]), binding("rows", vec![])],
            )])
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, RuleIntentError::DuplicateBinding { .. }))
        }),
    },
    Case {
        name: CaseName("rules_that_read_each_other_have_no_order"),
        word: Word::Refuses,
        input: || {
            Ask::Schedule(vec![
                observing("one", vec![binding("rows", vec![export("two")])]),
                observing("two", vec![binding("rows", vec![export("one")])]),
            ])
        },
        expect: Expect::Error(RuleIntentError::Cycle),
    },
    Case {
        name: CaseName("a_producer_without_a_plan_is_not_compiled"),
        word: Word::Refuses,
        input: || Ask::Schemas(pair("first"), vec![None, None]),
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, RuleIntentError::NotCompiled { .. }))
        }),
    },
    Case {
        name: CaseName("a_plan_that_exports_no_such_name_misses_the_output"),
        word: Word::Refuses,
        input: || Ask::Schemas(pair("first"), vec![Some(plan()), None]),
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, RuleIntentError::MissingOutput { .. }))
        }),
    },
    Case {
        name: CaseName("an_export_of_other_columns_is_incompatible"),
        word: Word::Refuses,
        input: || Ask::Schemas(pair("first"), vec![Some(exporting(vec![])), None]),
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                RuleIntentError::IncompatibleObservationColumns { .. }
            ))
        }),
    },
    Case {
        name: CaseName("a_step_the_plan_does_not_have_is_an_invalid_reference"),
        word: Word::Refuses,
        input: || dangling(),
        expect: Expect::Error(RuleIntentError::InvalidReference),
    },
    Case {
        name: CaseName("a_plan_cannot_have_more_steps_than_step_numbers"),
        word: Word::Refuses,
        input: || Ask::Following(StepId(u32::MAX)),
        expect: Expect::Error(RuleIntentError::TooManyOutputs),
    },
    Case {
        name: CaseName("a_plan_linking_prunes_nothing_from_stays_shared"),
        word: Word::Succeeds,
        input: || {
            let (plans, bindings) = demanded();
            Ask::Shared(plans, bindings)
        },
        expect: Expect::Value(Answer::Plans(Cow::Borrowed(&[Kept::Shared, Kept::Shared]))),
    },
    Case {
        name: CaseName("a_pruned_plan_holds_only_its_kept_steps"),
        word: Word::Succeeds,
        input: || {
            let (plans, bindings) = undemanded();
            Ask::Pruned(plans, bindings)
        },
        expect: Expect::Value(Answer::Steps { kept: 2, room: 2 }),
    },
];
fn subject(ask: Input) -> Result<Value, Failure> {
    Ok(match ask {
        Ask::Schedule(rules) => match schedule(&rules)? {
            RuleSchedule::Independent => Answer::Independent,
            RuleSchedule::Dependencies { order, batches } => Answer::Scheduled {
                order: order.iter().map(|rule| rule.0).collect(),
                batches: batches
                    .iter()
                    .map(|batch| batch.iter().map(|rule| rule.0).collect())
                    .collect(),
            },
        },
        Ask::Schemas(rules, compiled) => {
            schemas(&rules[1], &rules, &compiled).map(|_| Answer::Prepared)?
        }
        Ask::Following(step) => Answer::Next(following(step)?),
        Ask::Link(plans, bindings) => link(
            plans.into_iter().map(std::sync::Arc::new).collect(),
            bindings,
        )
        .map(|_| Answer::Linked)?,
        Ask::Shared(plans, bindings) => {
            let given: Vec<_> = plans.into_iter().map(std::sync::Arc::new).collect();
            let linked = link(given.clone(), bindings)?;
            Answer::Plans(
                given
                    .iter()
                    .zip(&linked.plans)
                    .map(|(given, linked)| {
                        if std::sync::Arc::ptr_eq(given, linked) {
                            Kept::Shared
                        } else {
                            Kept::New
                        }
                    })
                    .collect(),
            )
        }
        Ask::Pruned(plans, bindings) => {
            let linked = link(
                plans.into_iter().map(std::sync::Arc::new).collect(),
                bindings,
            )?;
            let first = linked
                .plans
                .first()
                .ok_or(RuleIntentError::InvalidReference)?;
            Answer::Steps {
                kept: first.steps.len(),
                room: first.steps.capacity(),
            }
        }
    })
}

/// A producer whose every step is kept, exporting its first step, and a consumer that reads the export in one step: linking
/// prunes nothing from either.
fn demanded() -> (Vec<Plan>, Vec<Vec<ResolvedObservationBinding>>) {
    let mut consumer = plan();
    consumer.observation_inputs = vec![bound_input("rows")];
    consumer.steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Project {
            from: PlanInput::BoundObservation(ObservationBindingId(0)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
    }];
    consumer.emit.step = StepId(0);
    let bound = ResolvedObservationBinding {
        input: ObservationBindingId(0),
        schema: bound_input("rows"),
        producers: vec![ResolvedObservationProducer::Output {
            rule: RuleIndex(0),
            output: ObservationIndex(0),
        }],
    };
    (
        vec![exporting(vec![column()]), consumer],
        vec![vec![], vec![bound]],
    )
}

/// A plan with a named output no rule reads, whose step nothing else reads: linking keeps only the emitted steps.
fn undemanded() -> (Vec<Plan>, Vec<Vec<ResolvedObservationBinding>>) {
    let mut unread = plan();
    unread.steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: PlanInput::Relation(Relation::Function),
            comparisons: vec![],
            kept: vec![],
        },
    });
    unread.observations.push(PlanObservation {
        name: ObservationName("unread".into()),
        source: PlanInput::Step(StepId(2)),
        columns: vec![column()],
        scope: StepScope::PerFile,
    });
    (vec![unread], vec![vec![]])
}

/// A plan that emits a step it does not have, linked beneath one binding.
fn dangling() -> Ask {
    let mut dangling = plan();
    dangling.emit.step = StepId(99);
    let bound = ResolvedObservationBinding {
        input: ObservationBindingId(0),
        schema: bound_input("rows"),
        producers: vec![],
    };
    Ask::Link(vec![dangling], vec![vec![bound]])
}

/// The two rules of a request in which `second` reads the export of `reads`.
fn pair(reads: &str) -> [RuleSpec; 2] {
    [
        rule("first"),
        observing("second", vec![binding("rows", vec![export(reads)])]),
    ]
}

/// Run every row that resolves.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// Run every row that is refused.
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
