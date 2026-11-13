//! Request-graph dependencies: the producers a rule may read, their schemas and the plans linked beneath them.
use std::borrow::Cow;

use super::fixtures::{binding, bound_input, export, exporting, observing, plan, rule};
use super::service::link::{service::following, service::link};
use super::service::{schedule, schemas};
use super::types::{ResolvedObservationBinding, RuleSchedule};
use crate::domain::rule_intent::errors::RuleIntentError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::observations::types::ObservationProducer;
use ryo_values::domain::plan_parts::types::{ObservationBindingId, StepId};
use ryo_values::domain::plans::types::Plan;
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
        Ask::Link(plans, bindings) => link(plans, bindings).map(|_| Answer::Linked)?,
    })
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
