//! Registered certification retains independent artifact namespaces and refuses changed authority.
use super::fixtures::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::requests::types::{BenchmarkCoverage, CertificationCases};
use std::borrow::Cow;

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
    Drift {
        change: &'static str,
        reason: &'static str,
    },
    /// A registered rule is missing from the document.
    MissingRule,
    /// The registered module inventory omits or adds a module.
    Modules(&'static str),
    /// A diagnostic run selects rule A from a profile narrowed to it.
    Selected,
    /// An accounted run selects rule A.
    SelectedAccounted,
    /// A diagnostic run selects rule A but measures both registered rules.
    SelectedUnnarrowed,
    /// A diagnostic run selects rule A from a copy of the registered profile.
    SelectedElsewhere,
    /// A diagnostic run selects rules A and C, and C is not registered.
    SelectedUnregistered,
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
    Case {
        name: CaseName("diagnostic certifies only the selected registered rules"),
        word: Word::Succeeds,
        input: || Scenario::Selected,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("accounted refuses a selected registration"),
        word: Word::Refuses,
        input: || Scenario::SelectedAccounted,
        expect: Expect::Error(Cow::Borrowed(
            crate::inbound::use_cases::certify::constants::SELECTED_COVERAGE_REQUIRES_INTERIM,
        )),
    },
    Case {
        name: CaseName("selected refuses rules other than the measured profile's"),
        word: Word::Refuses,
        input: || Scenario::SelectedUnnarrowed,
        expect: Expect::Error(Cow::Borrowed(
            crate::application::registered_certification::constants::SELECTED_RULES_DIFFER_FROM_MEASURED,
        )),
    },
    Case {
        name: CaseName("selected refuses a source other than the registered authority"),
        word: Word::Refuses,
        input: || Scenario::SelectedElsewhere,
        expect: Expect::Error(Cow::Borrowed(
            crate::application::registered_certification::constants::SELECTED_SOURCE_DIFFERS_FROM_AUTHORITY,
        )),
    },
    Case {
        name: CaseName("selected refuses a rule the registration does not hold"),
        word: Word::Refuses,
        input: || Scenario::SelectedUnregistered,
        expect: Expect::Error(Cow::Borrowed(
            crate::application::registered_certification::constants::SELECTED_RULES_DIFFER_FROM_MEASURED,
        )),
    },
];

/// The number of cases a certificate of rule A alone holds.
fn selected_subset() -> Result<Value, Failure> {
    let f = Fixture::new();
    let measured = narrowed(&f, &["A"]);
    let certificate = selected(
        &f,
        &measured,
        BenchmarkCoverage::Interim,
        &f.profile,
        &["A"],
    )
    .map_err(|error| failed(&error))?;
    Ok(certificate.cases().0)
}

/// An accounted run never certifies a selected coverage, and refuses before any case runs.
fn selected_accounted() -> Result<Value, Failure> {
    let f = Fixture::new();
    let measured = narrowed(&f, &["A"]);
    let reason = crate::inbound::use_cases::certify::constants::SELECTED_COVERAGE_REQUIRES_INTERIM;
    selected(
        &f,
        &measured,
        BenchmarkCoverage::Accounted,
        &f.profile,
        &["A"],
    )
    .map(|_| 0)
    .map_err(|error| refusal(&f, &error, reason, 0))
}

/// Selecting rule A while the measured profile holds A and B refuses before any case runs.
fn selected_unnarrowed() -> Result<Value, Failure> {
    let f = Fixture::new();
    let reason = crate::application::registered_certification::constants::SELECTED_RULES_DIFFER_FROM_MEASURED;
    selected(
        &f,
        &f.profile,
        BenchmarkCoverage::Interim,
        &f.profile,
        &["A"],
    )
    .map(|_| 0)
    .map_err(|error| refusal(&f, &error, reason, 0))
}

/// A source with the registered bytes at another path is not the registered authority.
fn selected_elsewhere() -> Result<Value, Failure> {
    let f = Fixture::new();
    let measured = narrowed(&f, &["A"]);
    let elsewhere = f._root.path().join("elsewhere");
    copy_measured_profile(&f, &elsewhere);
    let reason = crate::application::registered_certification::constants::SELECTED_SOURCE_DIFFERS_FROM_AUTHORITY;
    selected(
        &f,
        &measured,
        BenchmarkCoverage::Interim,
        &elsewhere,
        &["A"],
    )
    .map(|_| 0)
    .map_err(|error| refusal(&f, &error, reason, 0))
}

/// Selecting a rule the measured profile declares but the registration lacks refuses before any case runs.
fn selected_unregistered() -> Result<Value, Failure> {
    let f = Fixture::new();
    let measured = narrowed(&f, &["A", "C"]);
    let reason = crate::application::registered_certification::constants::SELECTED_RULES_DIFFER_FROM_MEASURED;
    selected(
        &f,
        &measured,
        BenchmarkCoverage::Interim,
        &f.profile,
        &["A", "C"],
    )
    .map(|_| 0)
    .map_err(|error| refusal(&f, &error, reason, 0))
}

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
        Scenario::Selected => selected_subset(),
        Scenario::SelectedAccounted => selected_accounted(),
        Scenario::SelectedUnnarrowed => selected_unnarrowed(),
        Scenario::SelectedElsewhere => selected_elsewhere(),
        Scenario::SelectedUnregistered => selected_unregistered(),
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
