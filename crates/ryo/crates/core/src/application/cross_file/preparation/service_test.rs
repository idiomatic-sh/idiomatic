//! The declared-slot owner refuses omissions, substitutions and repeated accounting.
use super::{errors::BatchPreparationError, service::Preparation, types::Occurrence};
use crate::application::cross_file::errors::CrossFileError;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::execution::types::{OperatingError, RuleIndex};
use ryo_values::domain::runs::types::RuleId;
#[derive(Clone, Copy)]
enum Scenario {
    EngineSingleton,
    EngineOrdinary,
    EngineNamed,
    EngineDuplicate,
    EngineFailure,
    OmitEngineOrdinary,
    OmitEngineNamed,
    Omitted,
    Repeated,
    Foreign,
    Ordinary,
    Named,
    Duplicate,
}
type Input = Scenario;
type Value = ();
type Failure = BatchPreparationError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("production_service_retains_legitimate_singleton"),
        word: Word::Succeeds,
        input: || Scenario::EngineSingleton,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("production_service_submits_complete_ordinary_batch_once"),
        word: Word::Succeeds,
        input: || Scenario::EngineOrdinary,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("production_service_submits_complete_named_batch_once"),
        word: Word::Succeeds,
        input: || Scenario::EngineNamed,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("production_service_preserves_duplicate_occurrences"),
        word: Word::Succeeds,
        input: || Scenario::EngineDuplicate,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("production_service_preserves_preparation_failure_and_healthy_outcome"),
        word: Word::Succeeds,
        input: || Scenario::EngineFailure,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("ordinary_omission_refused_before_real_stage_submission"),
        word: Word::Refuses,
        input: || Scenario::OmitEngineOrdinary,
        expect: Expect::Error(BatchPreparationError::Missing(Occurrence(1))),
    },
    Case {
        name: CaseName("named_omission_refused_before_real_stage_submission"),
        word: Word::Refuses,
        input: || Scenario::OmitEngineNamed,
        expect: Expect::Error(BatchPreparationError::Missing(Occurrence(1))),
    },
    Case {
        name: CaseName("ordinary_all_declared_failures_are_explicit"),
        word: Word::Succeeds,
        input: || Scenario::Ordinary,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("named_selected_inventory_preserves_batch_order"),
        word: Word::Succeeds,
        input: || Scenario::Named,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("duplicate_occurrences_each_need_accounting"),
        word: Word::Succeeds,
        input: || Scenario::Duplicate,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("omitted_occurrence_refuses_sealing"),
        word: Word::Refuses,
        input: || Scenario::Omitted,
        expect: Expect::Error(BatchPreparationError::Missing(Occurrence(1))),
    },
    Case {
        name: CaseName("repeated_slot_cannot_hide_an_omission"),
        word: Word::Refuses,
        input: || Scenario::Repeated,
        expect: Expect::Error(BatchPreparationError::Repeated(Occurrence(0))),
    },
    Case {
        name: CaseName("another_batch_cannot_supply_a_slot"),
        word: Word::Refuses,
        input: || Scenario::Foreign,
        expect: Expect::Error(BatchPreparationError::Foreign),
    },
];
fn todos() -> [Todo; 2] {
    [
        Todo {
            rule: RuleIndex(0),
            id: RuleId("first".into()),
        },
        Todo {
            rule: RuleIndex(1),
            id: RuleId("second".into()),
        },
    ]
}
fn failure() -> CrossFileError {
    OperatingError::NotRead.into()
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    let todo = todos();
    match scenario {
        Scenario::EngineSingleton
        | Scenario::EngineOrdinary
        | Scenario::EngineNamed
        | Scenario::EngineDuplicate
        | Scenario::EngineFailure
        | Scenario::OmitEngineOrdinary
        | Scenario::OmitEngineNamed => {
            use super::fixtures::{BatchControl, sealed_batch_control};
            sealed_batch_control(match scenario {
                Scenario::EngineSingleton => BatchControl::Singleton,
                Scenario::EngineOrdinary => BatchControl::Ordinary,
                Scenario::EngineNamed => BatchControl::Named,
                Scenario::EngineDuplicate => BatchControl::Duplicate,
                Scenario::EngineFailure => BatchControl::PreparationFailure,
                Scenario::OmitEngineOrdinary => BatchControl::OmitOrdinary,
                _ => BatchControl::OmitNamed,
            })
        }
        Scenario::Omitted => {
            let mut pending = Preparation::ordinary(&todo);
            let slots = pending.slots();
            pending.account(slots[0].clone(), Err(failure()))?;
            {
                let sealed = pending.seal();
                assert!(matches!(
                    sealed,
                    Err(BatchPreparationError::Missing(Occurrence(1)))
                ));
                sealed.map(|_| ())
            }
        }
        Scenario::Repeated => {
            let mut pending = Preparation::ordinary(&todo);
            let slots = pending.slots();
            pending.account(slots[0].clone(), Err(failure()))?;
            {
                let result = pending.account(slots[0].clone(), Err(failure()));
                assert!(matches!(
                    result,
                    Err(BatchPreparationError::Repeated(Occurrence(0)))
                ));
                result
            }
        }
        Scenario::Foreign => {
            let mut pending = Preparation::ordinary(&todo);
            let foreign = Preparation::ordinary(&todo).slots().remove(0);
            {
                let result = pending.account(foreign, Err(failure()));
                assert!(matches!(result, Err(BatchPreparationError::Foreign)));
                result
            }
        }
        Scenario::Ordinary | Scenario::Named | Scenario::Duplicate => {
            let pending = match scenario {
                Scenario::Named => Preparation::named(&[RuleIndex(1), RuleIndex(0)], &todo),
                Scenario::Duplicate => Preparation::ordinary(&[todo[0].clone(), todo[0].clone()]),
                _ => Preparation::ordinary(&todo),
            };
            let mut seen = Vec::new();
            let sealed = pending
                .prepare_all(|occurrence, todo| {
                    seen.push((occurrence, todo.rule));
                    Err(failure())
                })
                .seal()?;
            assert_eq!(seen.len(), 2);
            assert_eq!(seen[0].0, Occurrence(0));
            assert_eq!(seen[1].0, Occurrence(1));
            assert_eq!(
                seen[0].1,
                if matches!(scenario, Scenario::Named) {
                    RuleIndex(1)
                } else {
                    RuleIndex(0)
                }
            );
            assert_eq!(
                seen[1].1,
                if matches!(scenario, Scenario::Ordinary) {
                    RuleIndex(1)
                } else {
                    RuleIndex(0)
                }
            );
            let entries = sealed.into_entries();
            assert_eq!(entries.len(), 2);
            assert!(entries.iter().all(|entry| matches!(
                &entry.result,
                Err(CrossFileError::Operating(OperatingError::NotRead))
            )));
            Ok(())
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
