//! A measured run executes only a request without declared observations and covers exactly its files.
use super::fixtures::{Empty, Pinned, history_len, last_environment, request, succeeding};
use super::service::{run as measure, run_with};
use super::types::Pinning;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::linter::fixtures::fake::LinterFake;
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration;

/// Which measured request a row runs.
#[derive(Clone, Copy)]
enum Scenario {
    Plain,
    ObservationBearing,
    OmittedInput,
    Pinned(Pinning),
}

/// What a refused run states.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    VerificationRequired,
    InputCoverage,
    Unexpected,
}

type Input = Scenario;
type Value = usize;
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the measurement path runs a request without observations once"),
        word: Word::Succeeds,
        input: || Scenario::Plain,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the measurement path refuses observation-bearing requests unexecuted"),
        word: Word::Refuses,
        input: || Scenario::ObservationBearing,
        expect: Expect::Error(Refusal::VerificationRequired),
    },
    Case {
        name: CaseName("reported input omissions refuse observations"),
        word: Word::Refuses,
        input: || Scenario::OmittedInput,
        expect: Expect::Error(Refusal::InputCoverage),
    },
    Case {
        name: CaseName("a timed run keeps the linter's one-worker pin and its own variable"),
        word: Word::Succeeds,
        input: || Scenario::Pinned(Pinning::Timed),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("an untimed run drops only the thread pin; the linter's own variable survives"),
        word: Word::Succeeds,
        input: || Scenario::Pinned(Pinning::Untimed),
        expect: Expect::Value(1),
    },
];

fn category(error: &Error) -> Refusal {
    let message = error.to_string();
    if message.contains("require verification preparation") {
        Refusal::VerificationRequired
    } else if message == crate::domain::diagnostics::constants::INPUT_COVERAGE {
        Refusal::InputCoverage
    } else {
        Refusal::Unexpected
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Plain => plain(),
        Scenario::ObservationBearing => observation_bearing(),
        Scenario::OmittedInput => omitted_input(),
        Scenario::Pinned(pinning) => pinned(pinning),
    }
    .map_err(|error| category(&error))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

fn plain() -> Result<Value, Error> {
    let execution = succeeding();
    let mut request = request();
    request.files = vec![ryo::linter::RelativePath("case.py".into())];
    measure(
        &crate::outbound::context::factory::local(),
        &LinterFake::default(),
        &execution,
        &request,
    )?;
    Ok(history_len(&execution).unwrap_or_default())
}

fn observation_bearing() -> Result<Value, Error> {
    let mut request = request();
    request.file_metadata = FileMetadataDeclaration::Declared(std::collections::BTreeMap::from([(
        request.files[0].clone(),
        FileExecutable::Unset,
    )]));
    let execution = succeeding();
    let result = measure(
        &crate::outbound::context::factory::local(),
        &LinterFake::default(),
        &execution,
        &request,
    );
    assert_eq!(
        history_len(&execution),
        Some(0),
        "unsupported observation claims must not prepare or execute a measured contestant"
    );
    result.map(|_| 0)
}

fn omitted_input() -> Result<Value, Error> {
    measure(
        &crate::outbound::context::factory::local(),
        &Empty,
        &Empty,
        &request(),
    )
    .map(|_| 0)
}

fn pinned(pinning: Pinning) -> Result<Value, Error> {
    let execution = succeeding();
    let mut request = request();
    request.files = vec![ryo::linter::RelativePath("case.py".into())];
    run_with(
        &crate::outbound::context::factory::local(),
        &Pinned,
        &execution,
        &request,
        pinning,
    )?;
    let names = last_environment(&execution).unwrap_or_default();
    let thread_pin = names.contains(ryo_benchmark_contracts::domain::measurements::constants::THREAD_PIN);
    assert_eq!(thread_pin, pinning == Pinning::Timed, "the thread pin stands only in a timed run");
    assert!(names.contains("LINTER_HOME"), "a variable that is not the thread pin survives: {names}");
    Ok(names.split(',').filter(|name| !name.is_empty()).count())
}
