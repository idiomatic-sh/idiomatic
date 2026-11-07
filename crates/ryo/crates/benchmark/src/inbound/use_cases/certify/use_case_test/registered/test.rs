//! Registered certification retains independent artifact namespaces and refuses changed authority.
use super::fixtures::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;
use ryo_benchmark_contracts::domain::requests::types::CertificationCases;

const ARTIFACT: &str = "registered case artifact changed";
const QUERY: &str = "measured query differs";
const MODULE: &str = "measured helper differs";
const RECEIPT_ONLY: &str = "receipt-only inventory cannot authorize execution";
const IDENTITIES: &str = "case inventory executable identities differ from measured contestants";
const MODULE_INVENTORY: &str = "registered module inventory differs from pinned context profile";

/// One scenario of the registered contexts.
#[derive(Clone, Copy)]
enum Scenario {
    /// Both registered contexts execute natively.
    Executes,
    /// The measured contestants are swapped for each other.
    Swapped,
    /// One registered artifact, query, module or authority drifted.
    Drift { change: &'static str, reason: &'static str },
    /// A registered rule is missing from the document.
    MissingRule,
    /// The registered module inventory omits or adds a module.
    Modules(&'static str),
}

type Input = Scenario;
type Value = usize;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("registered contexts execute and keep colliding relative evidence names separate"),
        word: Word::Succeeds,
        input: || Scenario::Executes,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("swapping distinct measured contestants refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Swapped,
        expect: Expect::Error(Cow::Borrowed(IDENTITIES)),
    },
    Case {
        name: CaseName("a changed registered artifact refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Drift { change: "artifact", reason: ARTIFACT },
        expect: Expect::Error(Cow::Borrowed(ARTIFACT)),
    },
    Case {
        name: CaseName("a changed measured query refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Drift { change: "query", reason: QUERY },
        expect: Expect::Error(Cow::Borrowed(QUERY)),
    },
    Case {
        name: CaseName("a changed measured helper refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Drift { change: "module", reason: MODULE },
        expect: Expect::Error(Cow::Borrowed(MODULE)),
    },
    Case {
        name: CaseName("receipt-only authority refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Drift { change: "receipt-only", reason: RECEIPT_ONLY },
        expect: Expect::Error(Cow::Borrowed(RECEIPT_ONLY)),
    },
    Case {
        name: CaseName("a missing registered rule never issues a certificate"),
        word: Word::Refuses,
        input: || Scenario::MissingRule,
        expect: Expect::Error(Cow::Borrowed(
            crate::application::inventory_verification::constants::REGISTERED_INDEPENDENT_CASE_CERTIFICATION_FA,
        )),
    },
    Case {
        name: CaseName("omitting every registered module refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Modules("omit-all"),
        expect: Expect::Error(Cow::Borrowed(MODULE_INVENTORY)),
    },
    Case {
        name: CaseName("omitting one registered module refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Modules("omit-one"),
        expect: Expect::Error(Cow::Borrowed(MODULE_INVENTORY)),
    },
    Case {
        name: CaseName("adding an unexpected module refuses before native launch"),
        word: Word::Refuses,
        input: || Scenario::Modules("unexpected"),
        expect: Expect::Error(Cow::Borrowed(MODULE_INVENTORY)),
    },
];

fn executes() -> Result<Value, Failure> {
    let f = Fixture::new();
    assert_ne!(f.reference_executable, f.worker_executable);
    assert_ne!(
        digest(&f.reference_executable),
        digest(&f.worker_executable)
    );
    let certificate = f.certify().map_err(|error| failed(&error))?;
    assert!(matches!(
        certificate.case_contexts(),
        CertificationCases::Registered { .. }
    ));
    for rule in ["A", "B"] {
        assert!(
            f._root
                .path()
                .join(format!("receipts/profiles/{rule}/cases.jsonl"))
                .is_file()
        );
    }
    let launches = *f.execution.launches.lock().unwrap();
    Ok(launches)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Executes => executes(),
        Scenario::Swapped => swapped(IDENTITIES),
        Scenario::Drift { change, reason } => drift(change, reason),
        Scenario::MissingRule => missing_rule(),
        Scenario::Modules(change) => modules(change, MODULE_INVENTORY),
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
