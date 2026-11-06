//! Raw artifact data cannot issue an audit for a different or incomplete inventory partition.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::constants;
use crate::domain::suite::service::audited;
use ryo_benchmark_contracts::domain::reports::types::AuthoredInventory;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
use std::borrow::Cow;

type Input = AuthoredInventory;
type Value = (Cow<'static, str>, Cow<'static, str>);
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an exact disjoint inventory is bound before an audit is issued"),
        word: Word::Succeeds,
        input: || inventory(&["A001"], &["B001"]),
        expect: Expect::Value((Cow::Borrowed("A001"), Cow::Borrowed("B001"))),
    },
    Case {
        name: CaseName("a duplicated implemented default is refused"),
        word: Word::Refuses,
        input: || inventory(&["A001", "A001"], &["B001"]),
        expect: Expect::Error(Cow::Borrowed(
            constants::ARTIFACT_OBSERVATIONS_DO_NOT_PARTITION_THE,
        )),
    },
    Case {
        name: CaseName("a default both implemented and missing is refused"),
        word: Word::Refuses,
        input: || inventory(&["A001"], &["A001", "B001"]),
        expect: Expect::Error(Cow::Borrowed(
            constants::ARTIFACT_OBSERVATIONS_DO_NOT_PARTITION_THE,
        )),
    },
    Case {
        name: CaseName("an unreported default is refused"),
        word: Word::Refuses,
        input: || inventory(&["A001"], &[]),
        expect: Expect::Error(Cow::Borrowed(
            constants::ARTIFACT_OBSERVATIONS_DO_NOT_PARTITION_THE,
        )),
    },
    Case {
        name: CaseName("an undeclared default is refused"),
        word: Word::Refuses,
        input: || inventory(&["A001", "B001"], &["X001"]),
        expect: Expect::Error(Cow::Borrowed(
            constants::ARTIFACT_OBSERVATIONS_DO_NOT_PARTITION_THE,
        )),
    },
];

fn inventory(implemented: &[&str], missing: &[&str]) -> Input {
    let rules = |names: &[&str]| {
        names
            .iter()
            .map(|name| ryo::linter::RuleId::try_from(*name).unwrap())
            .collect()
    };
    AuthoredInventory {
        implemented: rules(implemented),
        missing: rules(missing),
    }
}

fn tool() -> Tool {
    Tool {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::workloads::types::ObservationSelectionPolicy::Declared,
        supporting_rules: vec![],
        rule_observations: Default::default(),
        rule_context: None,
        file_observation: None,
        project_context_mode: Default::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec!["pin".into()],
        deferred: Default::default(),
        pending: Default::default(),
        query_modules: Default::default(),
        rule_applicability: Default::default(),
        parser_configuration: None,
        source_language: None,
        project_search_roots: None,
        defaults: vec!["A001".try_into().unwrap(), "B001".try_into().unwrap()],
    }
}
fn subject(observed: Input) -> Result<Value, Failure> {
    let issued = audited(&tool(), observed).map_err(|error| error.to_string())?;
    let names = |rules: &[ryo::linter::RuleId]| {
        Cow::Owned(
            rules
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
        )
    };
    Ok((names(issued.authored()), names(&issued.missing)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
