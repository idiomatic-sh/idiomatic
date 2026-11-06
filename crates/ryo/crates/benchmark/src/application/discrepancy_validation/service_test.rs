//! Behavioral proof of `discrepancy_validation`: stale or unevidenced reference differences fail before either contestant executes.
use super::errors::DiscrepancyValidationError;
use super::fixtures::{
    Record, Served, Step, Validation, archived, certified, edited, evidenced, suite,
};
use super::service::{validate, validate_archives};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::errors::DiscrepancyError;
use crate::domain::discrepancy::fixtures::profile;
use crate::domain::registered_inputs::types::RefusalText;
use crate::outbound::linter::fixtures::fake::LinterFake;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::process::types::ReferenceExpectation;
use ryo_benchmark_contracts::domain::workloads::types::CaseComparison;
use std::path::Path;

/// What an accepted case answers.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Outcome {
    NoProof,
    Proved,
}

type Input = Validation;
type Value = Outcome;
type Failure = DiscrepancyValidationError;

fn unregistered() -> Validation {
    edited(evidenced, |case| case.reference_discrepancy = None)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a case without a registered discrepancy has no proof"),
        word: Word::Succeeds,
        input: unregistered,
        expect: Expect::Value(Outcome::NoProof),
    },
    Case {
        name: CaseName("a certificate binding this case, profile and invocation yields a proof"),
        word: Word::Succeeds,
        input: certified,
        expect: Expect::Value(Outcome::Proved),
    },
    Case {
        name: CaseName("missing evidence fails before execution"),
        word: Word::Refuses,
        input: || {
            edited(evidenced, |case| {
                case.reference_discrepancy
                    .as_mut()
                    .unwrap()
                    .evidence
                    .clear()
            })
        },
        expect: Expect::Error(DiscrepancyValidationError::Discrepancy(
            DiscrepancyError::ExpectationsNotDistinctFullEvidenced,
        )),
    },
    Case {
        name: CaseName("a rule projection cannot hide differences"),
        word: Word::Refuses,
        input: || edited(evidenced, |case| case.comparison = CaseComparison::Rule),
        expect: Expect::Error(DiscrepancyValidationError::Discrepancy(
            DiscrepancyError::ExpectationsNotDistinctFullEvidenced,
        )),
    },
    Case {
        name: CaseName("identical outputs are not a discrepancy"),
        word: Word::Refuses,
        input: || {
            edited(evidenced, |case| {
                let expected = case.expected.clone();
                case.reference_discrepancy.as_mut().unwrap().expected =
                    ReferenceExpectation::Diagnostics(expected);
            })
        },
        expect: Expect::Error(DiscrepancyValidationError::Discrepancy(
            DiscrepancyError::ExpectationsNotDistinctFullEvidenced,
        )),
    },
    Case {
        name: CaseName("rules without a profile directory name no profile root"),
        word: Word::Refuses,
        input: || Validation {
            rules: "",
            ..evidenced()
        },
        expect: Expect::Error(DiscrepancyValidationError::MissingProfileRoot),
    },
    Case {
        name: CaseName("an unapproved actual executable cannot attest the evidence"),
        word: Word::Refuses,
        input: || Validation {
            executable: ArtifactDigest("9".repeat(64)),
            ..evidenced()
        },
        expect: Expect::Error(DiscrepancyValidationError::Discrepancy(
            DiscrepancyError::StaleToolProfileOrExecutable,
        )),
    },
    Case {
        name: CaseName("language evidence for another parser target is refused"),
        word: Word::Refuses,
        input: || {
            edited(evidenced, |case| {
                let item = case.reference_discrepancy.as_mut().unwrap();
                item.evidence.clear();
                item.evidence.push(
                ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::DiscrepancyEvidence::Language {
                    path: "semantic-authority.txt".into(),
                    sha256: ArtifactDigest("f".repeat(64)),
                    kind: ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::DiscrepancyEvidenceKind::LanguageSpec,
                    language_version: "toy-v1".into(),
                    target_configuration: "python3.8".into(),
                    version_rationale: "toy fixture contract".into(),
                },
            );
            })
        },
        expect: Expect::Error(DiscrepancyValidationError::Discrepancy(
            DiscrepancyError::EvidenceDoesNotSupportConfiguration,
        )),
    },
    Case {
        name: CaseName("a suite that serves no profile refuses with its own words"),
        word: Word::Refuses,
        input: || Validation {
            served: Served::Nothing,
            ..evidenced()
        },
        expect: Expect::Error(DiscrepancyValidationError::Refused(RefusalText(
            String::new(),
        ))),
    },
    Case {
        name: CaseName("an archived capture that recorded no process identity is refused"),
        word: Word::Refuses,
        input: || archived(Record::Failed),
        expect: Expect::Error(DiscrepancyValidationError::ArchivedBytesDiffer),
    },
    Case {
        name: CaseName("archived settings that their arguments do not establish are refused"),
        word: Word::Refuses,
        input: || archived(Record::Unestablished),
        expect: Expect::Error(DiscrepancyValidationError::ArchivedSettingsNotEstablished),
    },
    Case {
        name: CaseName("a certificate of other settings, source or expectations is stale"),
        word: Word::Refuses,
        input: evidenced,
        expect: Expect::Error(DiscrepancyValidationError::StaleIdentity),
    },
];

/// A refusal in a library's words is one kind of refusal, whatever the words.
fn words_dropped(error: Failure) -> Failure {
    match error {
        DiscrepancyValidationError::Refused(_) => {
            DiscrepancyValidationError::Refused(RefusalText(String::new()))
        }
        other => other,
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    let served = suite(&input);
    match &input.step {
        Step::Validate => validate(
            &served,
            &crate::outbound::inventory::factory::local(),
            &LinterFake::default(),
            Path::new(input.rules),
            &input.case,
        )
        .map(|proof| proof.map_or(Outcome::NoProof, |_| Outcome::Proved)),
        Step::Archives(archive) => validate_archives(
            &served,
            &crate::outbound::inventory::factory::local(),
            archive.root.path(),
            &profile(),
            &input.case,
            &archive.process,
            &input.executable,
        )
        .map(|()| Outcome::Proved),
    }
    .map_err(words_dropped)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
