//! What the `project_budget` use case promises, against the fakes of the ports it calls, from its fixtures.
//! Real lint routes preserve budget failures and stop later project cohorts: the nodes run after the files, counted at the
//! operators port, are the first cohort's alone.
use super::fixtures::{BudgetRoute, BudgetScenario, MemoryPressure, RunCount, observe};
use crate::{
    cases::{Case, CaseName, Expect, Matched, Word, run, selects},
    inbound::use_cases::lint::errors::LintError,
    outbound::memory::errors::MemoryError,
};
type Input = BudgetScenario;
type Value = RunCount;
type Failure = LintError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordinary_budget_after_actual_first_cohort"),
        word: Word::Refuses,
        input: || BudgetScenario {
            route: BudgetRoute::Ordinary,
            pressure: MemoryPressure::Budget,
        },
        expect: Expect::Refusal(|error| {
            selects(Matched(
                error
                    == &LintError::MemoryBudgetExceeded {
                        used: 2_000_000,
                        budget: 1_000_000,
                        plans: vec!["second".into()],
                    },
            ))
        }),
    },
    Case {
        name: CaseName("ordinary_unreadable_after_actual_first_cohort"),
        word: Word::Fails,
        input: || BudgetScenario {
            route: BudgetRoute::Ordinary,
            pressure: MemoryPressure::Unreadable,
        },
        expect: Expect::Refusal(|error| {
            selects(Matched(
                error
                    == &LintError::from(MemoryError::Unreadable {
                        reason: "after first project reduction".into(),
                    }),
            ))
        }),
    },
    Case {
        name: CaseName("ordinary_allowed_after_actual_first_cohort"),
        word: Word::Succeeds,
        input: || BudgetScenario {
            route: BudgetRoute::Ordinary,
            pressure: MemoryPressure::Allowed,
        },
        expect: Expect::Value(RunCount(2)),
    },
    Case {
        name: CaseName("named_budget_after_actual_first_cohort"),
        word: Word::Refuses,
        input: || BudgetScenario {
            route: BudgetRoute::Named,
            pressure: MemoryPressure::Budget,
        },
        expect: Expect::Refusal(|error| {
            selects(Matched(
                error
                    == &LintError::MemoryBudgetExceeded {
                        used: 2_000_000,
                        budget: 1_000_000,
                        plans: vec!["second".into()],
                    },
            ))
        }),
    },
    Case {
        name: CaseName("named_unreadable_after_actual_first_cohort"),
        word: Word::Fails,
        input: || BudgetScenario {
            route: BudgetRoute::Named,
            pressure: MemoryPressure::Unreadable,
        },
        expect: Expect::Refusal(|error| {
            selects(Matched(
                error
                    == &LintError::from(MemoryError::Unreadable {
                        reason: "after first project reduction".into(),
                    }),
            ))
        }),
    },
    Case {
        name: CaseName("named_allowed_after_actual_first_cohort"),
        word: Word::Succeeds,
        input: || BudgetScenario {
            route: BudgetRoute::Named,
            pressure: MemoryPressure::Allowed,
        },
        expect: Expect::Value(RunCount(3)),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let observed = observe(input);
    if input.pressure != MemoryPressure::Allowed {
        // The first cohort's nodes after the files, recorded from a run: one ordinary rule's, or the named producer's two.
        let first_cohort = match input.route {
            BudgetRoute::Ordinary => RunCount(1),
            BudgetRoute::Named => RunCount(2),
        };
        assert_eq!(
            observed.runs, first_cohort,
            "the later project cohort must not execute after the first budget or memory error"
        );
    }
    let report = observed.report?;
    assert!(
        report.unknowns.is_empty(),
        "actual file frontiers and named members must be complete"
    );
    Ok(observed.runs)
}
// Engine lease liveness stays private; execution's lifecycle controls prove release.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
