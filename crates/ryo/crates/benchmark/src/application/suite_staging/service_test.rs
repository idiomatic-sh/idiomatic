//! A refused source observation stops staging before any snapshot writes, and a selected rule stages alone.
use super::service;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::constants::RULES;
use ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST;
use crate::outbound::suite::{factory::local, fixtures::fake::SuiteFake};
use ryo_benchmark_contracts::domain::workloads::types::StagePurpose;

/// Which source the staging is given.
#[derive(Clone, Copy)]
enum Scenario {
    /// A profile no suite can read.
    MissingProfile,
    /// A real profile with two authored rules, one selected.
    SelectedRule,
}

/// What a staged snapshot holds.
#[derive(Debug, PartialEq, Eq)]
enum Staged {
    /// Exactly the selected rule.
    SelectedRuleOnly,
}

type Input = Scenario;
type Value = Staged;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a selected rule is staged without its unselected sibling"),
        word: Word::Succeeds,
        input: || Scenario::SelectedRule,
        expect: Expect::Value(Staged::SelectedRuleOnly),
    },
    Case {
        name: CaseName("missing profile stops staging"),
        word: Word::Refuses,
        input: || Scenario::MissingProfile,
        expect: Expect::Error(()),
    },
];

fn missing_profile() -> Result<Value, Failure> {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("snapshot");
    let result = service::stage(
        &SuiteFake::default(),
        root.path(),
        &destination,
        &[],
        StagePurpose::Certification,
    );
    assert_eq!(
        std::fs::symlink_metadata(&destination).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    assert!(matches!(
        result,
        Err(ryo_benchmark_contracts::domain::reports::errors::ReportsError::Io(ref error))
            if error.kind() == std::io::ErrorKind::NotFound
    ));
    Err(())
}

fn selected_rule() -> Result<Value, Failure> {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("profile");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join(PROFILE_MANIFEST),
        "tool='fixture'\nversion='1'\nsha256=['pin']\ndefaults=['E722','F704']\n",
    )
    .unwrap();
    for rule in ["E722", "F704"] {
        let rule_root = directory.join(RULES).join(rule);
        std::fs::create_dir_all(&rule_root).unwrap();
        std::fs::write(rule_root.join("query.ryoql"), rule).unwrap();
        std::fs::write(rule_root.join("cases.toml"), "cases = []\n").unwrap();
    }
    let destination = root.path().join("snapshot");
    let selected = [ryo::linter::RuleId::try_from("E722").unwrap()];
    service::stage(
        &local(),
        &directory,
        &destination,
        &selected,
        StagePurpose::Certification,
    )
    .map_err(|_error| ())?;
    let children: Vec<_> = std::fs::read_dir(destination.join(RULES))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(children, [std::ffi::OsString::from("E722")]);
    Ok(Staged::SelectedRuleOnly)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::MissingProfile => missing_profile(),
        Scenario::SelectedRule => selected_rule(),
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
