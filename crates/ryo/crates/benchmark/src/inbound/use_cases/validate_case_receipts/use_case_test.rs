//! The producer's shared fixture crosses the real injected filesystem boundary.
use super::{interface::ValidateCaseReceiptsUseCase, use_case::ValidateCaseReceiptsUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::sync::Arc;

#[derive(Clone, Copy)]
enum RegistrationField {
    Complete,
    Authority,
    Modules,
    Artifacts,
}

type Input = RegistrationField;
type Value = usize;
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("shared receipt-only registration verifies both actual sides"),
        word: Word::Succeeds,
        input: || RegistrationField::Complete,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("omitted authority refuses"),
        word: Word::Refuses,
        input: || RegistrationField::Authority,
        expect: Expect::Error("authority"),
    },
    Case {
        name: CaseName("omitted module inventory refuses"),
        word: Word::Refuses,
        input: || RegistrationField::Modules,
        expect: Expect::Error("modules"),
    },
    Case {
        name: CaseName("omitted artifact inventory refuses"),
        word: Word::Refuses,
        input: || RegistrationField::Artifacts,
        expect: Expect::Error("artifacts"),
    },
];

fn subject(field: Input) -> Result<Value, Failure> {
    let mut registration: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/registrations/receipt_only/registration.json"
    ))
    .expect("shared registration fixture");
    let missing = match field {
        RegistrationField::Complete => None,
        RegistrationField::Authority => Some("authority"),
        RegistrationField::Modules => Some("modules"),
        RegistrationField::Artifacts => Some("artifacts"),
    };
    if let Some(key) = missing {
        let object = if key == "authority" {
            &mut registration
        } else {
            &mut registration["rules"][0]
        };
        object
            .as_object_mut()
            .expect("registration record")
            .remove(key);
    }
    let temporary = tempfile::tempdir().expect("receipt fixture directory");
    let registration_path = temporary.path().join("registration.json");
    let receipts = temporary.path().join("observations.jsonl");
    std::fs::write(
        &registration_path,
        serde_json::to_vec(&registration).expect("wire bytes"),
    )
    .expect("write registration");
    std::fs::write(
        &receipts,
        include_str!("../../../../tests/fixtures/registrations/receipt_only/observations.jsonl"),
    )
    .expect("write actual side observations");
    let use_case = ValidateCaseReceiptsUseCaseImpl::new(Arc::new(
        crate::outbound::inventory::factory::local(),
    ));
    match use_case.validate_case_receipts(&registration_path, &receipts) {
        Ok(audit) => {
            assert!(audit.errors.is_empty(), "{:?}", audit.errors);
            assert_eq!(audit.verified, vec![("A001".into(), "mode".into())]);
            assert_eq!(audit.raw_exact, 1);
            assert_eq!(audit.metadata_verified.len(), 1);
            Ok(audit.case_count)
        }
        Err(ReportsError::Invalid(message)) => {
            let key = missing.expect("complete fixture must decode and verify");
            assert!(
                message.contains(&format!("missing field `{key}`")),
                "{message}"
            );
            Err(key)
        }
        Err(error) => panic!("unexpected receipt refusal: {error}"),
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
