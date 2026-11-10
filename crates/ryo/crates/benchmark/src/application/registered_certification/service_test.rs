//! A registered rule certifies only when it is executable and pins the tested query.
use super::service::pin_rule;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::constants::{QUERY_RYOQL, RULES};
use crate::outbound::inventory::types::{
    Command, FilePin, RegisteredRule, RuleCommand, RuleDefinition, RuleInstance, RuleReason,
};
use crate::outbound::linter::fixtures::data::tool;
use crate::outbound::suite::fixtures::fake::SuiteFake;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    ReasonText, RuleName, Sha256Text,
};
use std::path::PathBuf;

/// Which registered rule the certification is given.
#[derive(Clone, Copy)]
enum Scenario {
    /// A rule its registration states cannot run.
    Stated,
    /// A rule with no command to run.
    Unregistered,
    /// A rule that pins the very query the profile holds.
    Pinned,
}

/// What the certification kept of an admitted rule.
#[derive(Debug, PartialEq, Eq)]
enum Admitted {
    /// The one pinned query is the only artifact the rule contributes.
    TheQueryPin,
}

/// How the certification refused the rule.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    /// The rule is not executable.
    Unexecutable,
}

type Input = Scenario;
type Value = Admitted;
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a rule pinning the measured query contributes its pin"),
        word: Word::Succeeds,
        input: || Scenario::Pinned,
        expect: Expect::Value(Admitted::TheQueryPin),
    },
    Case {
        name: CaseName("a rule whose registration states a reason is unexecutable"),
        word: Word::Refuses,
        input: || Scenario::Stated,
        expect: Expect::Error(Refusal::Unexecutable),
    },
    Case {
        name: CaseName("a rule without a command is unexecutable"),
        word: Word::Refuses,
        input: || Scenario::Unregistered,
        expect: Expect::Error(Refusal::Unexecutable),
    },
];

const DIGEST: &str = "a";

fn query() -> PathBuf {
    PathBuf::from("context")
        .join(RULES)
        .join("A001")
        .join(QUERY_RYOQL)
}

fn rule(scenario: Scenario) -> RegisteredRule {
    let pinned = matches!(scenario, Scenario::Pinned);
    RegisteredRule {
        definition: if pinned {
            RuleDefinition::Pinned(FilePin {
                path: query(),
                sha256: Sha256Text(DIGEST.into()),
            })
        } else {
            RuleDefinition::Unpinned
        },
        modules: Default::default(),
        artifacts: if pinned {
            [(query(), Sha256Text(DIGEST.into()))].into()
        } else {
            Default::default()
        },
        rule_id: RuleName("A001".into()),
        command: if pinned {
            RuleCommand::Registered(Command::CaseVerification {
                context: PathBuf::from("context"),
            })
        } else {
            RuleCommand::Unregistered
        },
        reason: match scenario {
            Scenario::Stated => RuleReason::Stated(ReasonText("not implemented".into())),
            Scenario::Unregistered | Scenario::Pinned => RuleReason::Unstated,
        },
        cases: Vec::new(),
        instance: RuleInstance::Undeclared,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let profile = PathBuf::from("profile");
    let mut suite = SuiteFake::default();
    suite.artifact_digests.insert(
        profile.join(RULES).join("A001").join(QUERY_RYOQL),
        ArtifactDigest(DIGEST.into()),
    );
    let paths = pin_rule(
        &rule(scenario),
        &suite,
        &profile,
        &tool(),
        &profile.join(RULES),
    )
    .map_err(|error| {
        assert!(
            error
                .to_string()
                .contains("unexecutable registered context")
        );
        Refusal::Unexecutable
    })?;
    assert_eq!(paths, [query()]);
    Ok(Admitted::TheQueryPin)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
