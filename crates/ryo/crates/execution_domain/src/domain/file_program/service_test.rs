//! Request-graph dependencies: the producers a rule may read, their schemas and the plans linked beneath them.
use crate::domain::rule_intent::errors::RuleIntentError;
use super::fixtures::{binding, bound_input, export, exporting, observing, plan, rule};
use super::service::link::{following, link};
use super::service::observations::{schedule, schemas};
use super::types::ResolvedObservationBinding;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::observations::types::ObservationProducer;
use ryo_values::domain::plan_parts::types::{ObservationBindingId, StepId};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::selects;

#[derive(Clone, Copy)]
enum Scenario {
    Chain,
    UnknownProducer,
    SelfRead,
    ExecutableNoQuery,
    RepeatedBinding,
    Cycle,
    NotCompiled,
    MissingOutput,
    IncompatibleColumns,
    DanglingStep,
    StepNumbersExhausted,
}
type Input = Scenario;
type Value = ();
type Failure = RuleIntentError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_consumer_reads_the_export_of_an_earlier_producer"),
        word: Word::Succeeds,
        input: || Scenario::Chain,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_unlisted_producer_is_unknown"),
        word: Word::Refuses,
        input: || Scenario::UnknownProducer,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, RuleIntentError::UnknownProducer { .. }))
        }),
    },
    Case {
        name: CaseName("a_rule_cannot_read_its_own_export"),
        word: Word::Refuses,
        input: || Scenario::SelfRead,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, RuleIntentError::SelfDependency { .. }))
        }),
    },
    Case {
        name: CaseName("a_listed_rule_cannot_be_declared_without_a_query"),
        word: Word::Refuses,
        input: || Scenario::ExecutableNoQuery,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, RuleIntentError::ExecutableNoQuery { .. }))
        }),
    },
    Case {
        name: CaseName("a_name_is_bound_once_per_rule"),
        word: Word::Refuses,
        input: || Scenario::RepeatedBinding,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, RuleIntentError::DuplicateBinding { .. }))
        }),
    },
    Case {
        name: CaseName("rules_that_read_each_other_have_no_order"),
        word: Word::Refuses,
        input: || Scenario::Cycle,
        expect: Expect::Error(RuleIntentError::Cycle),
    },
    Case {
        name: CaseName("a_producer_without_a_plan_is_not_compiled"),
        word: Word::Refuses,
        input: || Scenario::NotCompiled,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, RuleIntentError::NotCompiled { .. }))
        }),
    },
    Case {
        name: CaseName("a_plan_that_exports_no_such_name_misses_the_output"),
        word: Word::Refuses,
        input: || Scenario::MissingOutput,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, RuleIntentError::MissingOutput { .. }))
        }),
    },
    Case {
        name: CaseName("an_export_of_other_columns_is_incompatible"),
        word: Word::Refuses,
        input: || Scenario::IncompatibleColumns,
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
        input: || Scenario::DanglingStep,
        expect: Expect::Error(RuleIntentError::InvalidReference),
    },
    Case {
        name: CaseName("a_plan_cannot_have_more_steps_than_step_numbers"),
        word: Word::Refuses,
        input: || Scenario::StepNumbersExhausted,
        expect: Expect::Error(RuleIntentError::TooManyOutputs),
    },
];
use ryo_values::domain::plan_parts::types::Input as InputSource;
fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Chain => schedule(&pair("first")).map(|_| ()),
        Scenario::UnknownProducer => schedule(&pair("absent")).map(|_| ()),
        Scenario::SelfRead => {
            schedule(&[observing("only", vec![binding("rows", vec![export("only")])])])
                .map(|_| ())
        }
        Scenario::ExecutableNoQuery => {
            schedule(&[observing(
                "only",
                vec![binding(
                    "rows",
                    vec![ObservationProducer::NoQuery {
                        rule: RuleId("only".into()),
                    }],
                )],
            )])
            .map(|_| ())
        }
        Scenario::RepeatedBinding => {
            schedule(&[observing(
                "only",
                vec![binding("rows", vec![]), binding("rows", vec![])],
            )])
            .map(|_| ())
        }
        Scenario::Cycle => {
            schedule(&[
                observing("one", vec![binding("rows", vec![export("two")])]),
                observing("two", vec![binding("rows", vec![export("one")])]),
            ])
            .map(|_| ())
        }
        Scenario::NotCompiled => {
            let rules = pair("first");
            schemas::<Plan>(&rules[1], &rules, &[None, None]).map(|_| ())
        }
        Scenario::MissingOutput => {
            let rules = pair("first");
            schemas(&rules[1], &rules, &[Some(plan()), None]).map(|_| ())
        }
        Scenario::IncompatibleColumns => {
            let rules = pair("first");
            schemas(&rules[1], &rules, &[Some(exporting(vec![])), None]).map(|_| ())
        }
        Scenario::StepNumbersExhausted => {
            following(StepId(u32::MAX)).map(|_| ())
        }
        Scenario::DanglingStep => {
            let mut dangling = plan();
            dangling.emit.step = StepId(99);
            let bound = ResolvedObservationBinding {
                input: ObservationBindingId(0),
                schema: bound_input("rows"),
                producers: vec![],
            };
            link(vec![dangling], vec![vec![bound]]).map(|_| ())
        }
    }
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
