//! A registered rule certifies only when it is executable and pins the tested query.
use super::service::{pin_rule, require_defaults};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::constants::{QUERY_RYOQL, RULES};
use crate::outbound::inventory::types::{
    Command, FilePin, ProviderName, RegisteredRule, Registration, RegistrationAuthority,
    RuleCommand, RuleDefinition, RuleInstance, RuleReason,
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
    /// A full registration of the measured default A001 and a deferred default, which staging leaves out of the profile.
    DeferredDefault,
    /// A full registration of A001 and another default registered as executable, which the profile does not hold.
    MissingDefault,
}

/// What the certification kept of an admitted rule.
#[derive(Debug, PartialEq, Eq)]
enum Admitted {
    /// The one pinned query is the only artifact the rule contributes.
    TheQueryPin,
    /// The registered defaults are the measured profile's.
    TheMeasuredDefaults,
}

/// How the certification refused the rule.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    /// The rule is not executable.
    Unexecutable,
    /// The registered defaults differ from the measured profile's.
    InventoryDiffers,
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
    Case {
        name: CaseName(
            "a full registration is the measured profile's defaults but its deferred ones",
        ),
        word: Word::Succeeds,
        input: || Scenario::DeferredDefault,
        expect: Expect::Value(Admitted::TheMeasuredDefaults),
    },
    Case {
        name: CaseName("a registered executable default the measured profile lacks is refused"),
        word: Word::Refuses,
        input: || Scenario::MissingDefault,
        expect: Expect::Error(Refusal::InventoryDiffers),
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
            Scenario::Unregistered
            | Scenario::Pinned
            | Scenario::DeferredDefault
            | Scenario::MissingDefault => RuleReason::Unstated,
        },
        cases: Vec::new(),
        instance: RuleInstance::Undeclared,
    }
}

/// A full registration of the pinned A001 and `other`, deferred when `deferred`, over a profile whose one default is A001.
fn defaults(deferred: bool) -> Result<Value, Failure> {
    let other = RegisteredRule {
        rule_id: RuleName("B002".into()),
        command: if deferred {
            RuleCommand::Deferred(
                ryo_benchmark_contracts::domain::workloads::types::capability_name::value::CapabilityName(
                    "types".into(),
                ),
            )
        } else {
            RuleCommand::Unregistered
        },
        ..rule(Scenario::Pinned)
    };
    let registration = Registration {
        authority: RegistrationAuthority::ReceiptOnly,
        provider: ProviderName("fixture".into()),
        defaults: vec![RuleName("A001".into()), RuleName("B002".into())],
        rules: vec![rule(Scenario::Pinned), other],
        identity: Default::default(),
    };
    let mut measured = tool();
    measured.defaults =
        vec![ryo::linter::RuleId::try_from("A001").map_err(|_| Refusal::Unexecutable)?];
    require_defaults(
        &registration,
        &measured,
        &ryo_benchmark_contracts::domain::requests::types::RegisteredCoverage::Full,
    )
    .map(|()| Admitted::TheMeasuredDefaults)
    .map_err(|_differs| Refusal::InventoryDiffers)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::DeferredDefault => return defaults(true),
        Scenario::MissingDefault => return defaults(false),
        Scenario::Stated | Scenario::Unregistered | Scenario::Pinned => {}
    }
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
