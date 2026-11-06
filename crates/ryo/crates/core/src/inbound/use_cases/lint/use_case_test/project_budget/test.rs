//! What the `project_budget` use case promises, against the fakes of the ports it calls, from its fixtures.
//! Real lint routes preserve budget failures and stop later project cohorts.
use super::fixtures::{BudgetRoute, BudgetScenario, MemoryPressure, ReductionCount, observe};
use crate::{
    cases::{Case, CaseName, Expect, Word, run, selects},
    inbound::use_cases::lint::errors::LintError,
    outbound::memory::errors::MemoryError,
};
type Input = BudgetScenario;
type Value = ReductionCount;
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
            selects(
                error
                    == &LintError::MemoryBudgetExceeded {
                        used: 2_000_000,
                        budget: 1_000_000,
                        plans: vec!["second".into()],
                    },
            )
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
            selects(
                error
                    == &LintError::from(MemoryError::Unreadable {
                        reason: "after first project reduction".into(),
                    }),
            )
        }),
    },
    Case {
        name: CaseName("ordinary_allowed_after_actual_first_cohort"),
        word: Word::Succeeds,
        input: || BudgetScenario {
            route: BudgetRoute::Ordinary,
            pressure: MemoryPressure::Allowed,
        },
        expect: Expect::Value(ReductionCount(2)),
    },
    Case {
        name: CaseName("named_budget_after_actual_first_cohort"),
        word: Word::Refuses,
        input: || BudgetScenario {
            route: BudgetRoute::Named,
            pressure: MemoryPressure::Budget,
        },
        expect: Expect::Refusal(|error| {
            selects(
                error
                    == &LintError::MemoryBudgetExceeded {
                        used: 2_000_000,
                        budget: 1_000_000,
                        plans: vec!["second".into()],
                    },
            )
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
            selects(
                error
                    == &LintError::from(MemoryError::Unreadable {
                        reason: "after first project reduction".into(),
                    }),
            )
        }),
    },
    Case {
        name: CaseName("named_allowed_after_actual_first_cohort"),
        word: Word::Succeeds,
        input: || BudgetScenario {
            route: BudgetRoute::Named,
            pressure: MemoryPressure::Allowed,
        },
        expect: Expect::Value(ReductionCount(2)),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let observed = observe(input);
    if input.pressure != MemoryPressure::Allowed {
        assert_eq!(
            observed.reductions,
            ReductionCount(1),
            "the later project cohort must not execute after the first budget or memory error"
        );
    }
    let report = observed.report?;
    assert!(
        report.unknowns.is_empty(),
        "actual file frontiers and named members must be complete"
    );
    Ok(observed.reductions)
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
