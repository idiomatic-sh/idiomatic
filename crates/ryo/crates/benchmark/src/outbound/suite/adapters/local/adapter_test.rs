//! Actual physical suite observations, exact digest bytes and profile decoding.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::fixtures::contract::check;
use crate::outbound::suite::{
    factory::local,
    interface::SuitePort as _,
    types::{ArtifactPath, CaseReceipts},
};
fn physical_contract() {
    check(&local(CaseReceipts::Unrecorded));
}
fn exact_executable_digest_bytes() {
    let directory = tempfile::tempdir().expect("source artifacts");
    let bytes = directory.path().join("executable");
    std::fs::write(&bytes, b"abc").expect("digest fixture");
    assert_eq!(
        local(CaseReceipts::Unrecorded).digest(ArtifactPath(&bytes)).expect("digest").0,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert!(
        matches!(local(CaseReceipts::Unrecorded).digest(ArtifactPath(directory.path())), Err(ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(message)) if message == "executable digest requires a regular file")
    );
}
fn optional_source_language_profile_preserves_default_and_validates_exact_names() {
    let directory = tempfile::tempdir().expect("profile");
    let base = "tool = 'fixture'\nversion = '1'\nsha256 = ['pin']\ndefaults = ['F401']\n";
    for (suffix, expected) in [("", None), ("source-language = 'python'\n", Some("python"))] {
        std::fs::write(
            directory.path().join("benchmark.toml"),
            format!("{base}{suffix}"),
        )
        .expect("profile bytes");
        let profile = local(CaseReceipts::Unrecorded)
            .profile(ArtifactPath(directory.path()))
            .expect("profile");
        assert_eq!(profile.source_language.as_deref(), expected);
    }
    for bad in ["", " python", "python "] {
        std::fs::write(
            directory.path().join("benchmark.toml"),
            format!("{base}source-language = '{bad}'\n"),
        )
        .expect("profile bytes");
        assert!(matches!(
            local(CaseReceipts::Unrecorded).profile(ArtifactPath(directory.path())),
            Err(_refused_profile)
        ));
    }
}

type Scenario = fn();
type Input = Scenario;
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("physical_contract"),
        word: Word::Succeeds,
        input: || physical_contract as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("exact_executable_digest_bytes"),
        word: Word::Succeeds,
        input: || exact_executable_digest_bytes as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "optional_source_language_profile_preserves_default_and_validates_exact_names",
        ),
        word: Word::Succeeds,
        input: || {
            optional_source_language_profile_preserves_default_and_validates_exact_names as fn()
        },
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
