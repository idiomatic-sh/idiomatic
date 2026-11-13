//! Behavioral proof of `discrepancy`: a pinned reference may differ from the intended contract only with complete, known evidence.
use super::errors::DiscrepancyError;
use super::fixtures::{accounted, case, differing, evidence, identity, profile};
use super::service::{
    check_evidence, complete, identity_digest, matches_intended, output_digest, reference_digest,
    reference_expected, require_distinct, validate_pins,
};
use super::types::{Contestant, IntentAgreement};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode;
use ryo_benchmark_contracts::domain::process::types::{ProcessFailure, ReferenceExpectation};
use ryo_benchmark_contracts::domain::reports::types::{
    CoverageAccounting, ExecutionMetadata, PairCount, UnavailableReason,
};
use ryo_benchmark_contracts::domain::workloads::types::{
    CaseComparison, DiscrepancyBasis, PreparedReferenceBehavior,
    discrepancy_evidence::value::{DiscrepancyEvidence, ParserTarget},
};

/// What a row asks of the rules.
#[derive(Clone, Debug)]
enum Operation {
    ReferenceAgainstIntendedDigest,
    AddedFileAgainstIntendedDigest,
    ReferenceExpectedAgainstIntended,
    IntendedAgainstItself,
    Complete(Contestant),
    Pins {
        actual: ArtifactDigest,
        profile: ArtifactDigest,
    },
    Spelling(&'static str),
    ReferenceExpected,
    Distinct,
    Evidence,
    UnicodeMultiset,
}

struct Request {
    case: ryo_benchmark_contracts::domain::workloads::types::PreparedCase,
    operation: Operation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Outcome {
    Accepted,
    Equal,
    Different,
}

type Input = Request;
type Value = Outcome;
type Failure = DiscrepancyError;

fn asking(operation: Operation) -> Request {
    Request {
        case: case(),
        operation,
    }
}

fn edited(
    operation: Operation,
    edit: impl FnOnce(&mut ryo_benchmark_contracts::domain::workloads::types::PreparedCase),
) -> Request {
    let mut request = asking(operation);
    edit(&mut request.case);
    request
}

fn digest(letter: &str) -> ArtifactDigest {
    ArtifactDigest(letter.repeat(64))
}

fn agreement(answer: IntentAgreement) -> Outcome {
    match answer {
        IntentAgreement::Matches => Outcome::Equal,
        IntentAgreement::Differs => Outcome::Different,
    }
}

fn unavailable(metadata: ExecutionMetadata) -> Request {
    edited(Operation::Complete(Contestant::Candidate), |case| {
        case.expected.execution_metadata = metadata;
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the wrong oracle has a distinct digest from the intended contract"),
        word: Word::Succeeds,
        input: || asking(Operation::ReferenceAgainstIntendedDigest),
        expect: Expect::Value(Outcome::Different),
    },
    Case {
        name: CaseName("a coverage identity change invalidates the evidence digest"),
        word: Word::Succeeds,
        input: || asking(Operation::AddedFileAgainstIntendedDigest),
        expect: Expect::Value(Outcome::Different),
    },
    Case {
        name: CaseName("copying the wrong oracle fails the intended behavior"),
        word: Word::Succeeds,
        input: || asking(Operation::ReferenceExpectedAgainstIntended),
        expect: Expect::Value(Outcome::Different),
    },
    Case {
        name: CaseName("the intended contract matches itself"),
        word: Word::Succeeds,
        input: || asking(Operation::IntendedAgainstItself),
        expect: Expect::Value(Outcome::Equal),
    },
    Case {
        name: CaseName("a complete candidate is admissible"),
        word: Word::Succeeds,
        input: || asking(Operation::Complete(Contestant::Candidate)),
        expect: Expect::Value(Outcome::Accepted),
    },
    Case {
        name: CaseName("an approved other platform binary is portable"),
        word: Word::Succeeds,
        input: || {
            asking(Operation::Pins {
                actual: digest("b"),
                profile: digest("c"),
            })
        },
        expect: Expect::Value(Outcome::Accepted),
    },
    Case {
        name: CaseName("a lowercase sha256 identity is accepted"),
        word: Word::Succeeds,
        input: || {
            asking(Operation::Spelling(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            ))
        },
        expect: Expect::Value(Outcome::Accepted),
    },
    Case {
        name: CaseName("a language evidence row supports the configured contract"),
        word: Word::Succeeds,
        input: || {
            edited(Operation::Evidence, |case| {
                differing(case).evidence.push(evidence());
            })
        },
        expect: Expect::Value(Outcome::Accepted),
    },
    Case {
        name: CaseName("canonical unicode multisets digest as the python certificate encoding"),
        word: Word::Succeeds,
        input: || asking(Operation::UnicodeMultiset),
        expect: Expect::Value(Outcome::Equal),
    },
    Case {
        name: CaseName("a candidate without accounting cannot admit a discrepancy"),
        word: Word::Refuses,
        input: || {
            edited(Operation::Complete(Contestant::Candidate), |case| {
                case.expected.coverage = CoverageAccounting::Unaccounted
            })
        },
        expect: Expect::Error(DiscrepancyError::CoverageNotCompleteAndKnown),
    },
    Case {
        name: CaseName("unknown execution capability is not a discrepancy"),
        word: Word::Refuses,
        input: || unavailable(ExecutionMetadata::Unknown),
        expect: Expect::Error(DiscrepancyError::CoverageNotCompleteAndKnown),
    },
    Case {
        name: CaseName("unavailable execution capability is not a discrepancy"),
        word: Word::Refuses,
        input: || {
            unavailable(ExecutionMetadata::Unavailable {
                reason: UnavailableReason("not observed".into()),
            })
        },
        expect: Expect::Error(DiscrepancyError::CoverageNotCompleteAndKnown),
    },
    Case {
        name: CaseName("an unread rule file pair is not a discrepancy"),
        word: Word::Refuses,
        input: || {
            edited(Operation::Complete(Contestant::Candidate), |case| {
                accounted(&mut case.expected).unknown_or_unread = PairCount(1);
            })
        },
        expect: Expect::Error(DiscrepancyError::CoverageNotCompleteAndKnown),
    },
    Case {
        name: CaseName("an unapproved tool cannot attest evidence"),
        word: Word::Refuses,
        input: || {
            asking(Operation::Pins {
                actual: digest("9"),
                profile: digest("c"),
            })
        },
        expect: Expect::Error(DiscrepancyError::StaleToolProfileOrExecutable),
    },
    Case {
        name: CaseName("a changed profile invalidates the evidence"),
        word: Word::Refuses,
        input: || {
            asking(Operation::Pins {
                actual: digest("a"),
                profile: digest("8"),
            })
        },
        expect: Expect::Error(DiscrepancyError::StaleToolProfileOrExecutable),
    },
    Case {
        name: CaseName("an uppercase sha256 identity is refused"),
        word: Word::Refuses,
        input: || {
            asking(Operation::Spelling(
                "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789",
            ))
        },
        expect: Expect::Error(DiscrepancyError::IdentityNotLowercaseSha256),
    },
    Case {
        name: CaseName("a short identity is refused"),
        word: Word::Refuses,
        input: || asking(Operation::Spelling("abc")),
        expect: Expect::Error(DiscrepancyError::IdentityNotLowercaseSha256),
    },
    Case {
        name: CaseName("a process expectation is not a diagnostic report"),
        word: Word::Refuses,
        input: || {
            edited(Operation::ReferenceExpected, |case| {
                differing(case).expected = ReferenceExpectation::ProcessFailure(ProcessFailure {
                    exit: ProcessExitCode(2),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                    observations: Vec::new(),
                });
            })
        },
        expect: Expect::Error(DiscrepancyError::ProcessExpectationNotDiagnostic),
    },
    Case {
        name: CaseName("missing evidence fails before execution"),
        word: Word::Refuses,
        input: || asking(Operation::Distinct),
        expect: Expect::Error(DiscrepancyError::ExpectationsNotDistinctFullEvidenced),
    },
    Case {
        name: CaseName("a rule projection cannot hide differences"),
        word: Word::Refuses,
        input: || {
            edited(Operation::Distinct, |case| {
                differing(case).evidence.push(evidence());
                case.comparison = CaseComparison::Rule;
            })
        },
        expect: Expect::Error(DiscrepancyError::ExpectationsNotDistinctFullEvidenced),
    },
    Case {
        name: CaseName("identical outputs are not a discrepancy"),
        word: Word::Refuses,
        input: || {
            edited(Operation::Distinct, |case| {
                let expected = case.expected.clone();
                let item = differing(case);
                item.evidence.push(evidence());
                item.expected = ReferenceExpectation::Diagnostics(expected);
            })
        },
        expect: Expect::Error(DiscrepancyError::ExpectationsNotDistinctFullEvidenced),
    },
    Case {
        name: CaseName("a tool contract basis cannot be supported by language evidence"),
        word: Word::Refuses,
        input: || {
            edited(Operation::Evidence, |case| {
                let item = differing(case);
                item.evidence.push(evidence());
                item.basis = DiscrepancyBasis::ToolContract;
            })
        },
        expect: Expect::Error(DiscrepancyError::AuthorityAndEvidenceDiffer),
    },
    Case {
        name: CaseName("language evidence for another parser target is refused"),
        word: Word::Refuses,
        input: || {
            edited(Operation::Evidence, |case| {
                differing(case).evidence.push(match evidence() {
                    DiscrepancyEvidence::Language {
                        path,
                        sha256,
                        kind,
                        language_version,
                        version_rationale,
                        ..
                    } => DiscrepancyEvidence::Language {
                        path,
                        sha256,
                        kind,
                        language_version,
                        version_rationale,
                        target_configuration: ParserTarget("python3.8".into()),
                    },
                    other @ (DiscrepancyEvidence::PublicToolContract { .. }
                    | DiscrepancyEvidence::PriorWorkerRun { .. }) => other,
                });
            })
        },
        expect: Expect::Error(DiscrepancyError::EvidenceDoesNotSupportConfiguration),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    let case = &input.case;
    let PreparedReferenceBehavior::Differs(item) = &case.reference_discrepancy else {
        panic!("the fixture case documents a difference")
    };
    match input.operation {
        Operation::ReferenceAgainstIntendedDigest => Ok(equal(
            &reference_digest(&item.expected),
            &output_digest(&case.expected),
        )),
        Operation::AddedFileAgainstIntendedDigest => {
            let mut changed = case.expected.clone();
            changed
                .files
                .push(ryo::linter::RelativePath("other.py".into()));
            Ok(equal(
                &output_digest(&changed),
                &output_digest(&case.expected),
            ))
        }
        Operation::ReferenceExpectedAgainstIntended => {
            Ok(agreement(matches_intended(case, reference_expected(case)?)))
        }
        Operation::IntendedAgainstItself => Ok(agreement(matches_intended(case, &case.expected))),
        Operation::Complete(contestant) => {
            complete(case, &case.expected.clone(), contestant).map(|()| Outcome::Accepted)
        }
        Operation::Pins {
            actual,
            profile: pinned,
        } => validate_pins(&identity(), &profile(), &actual, &pinned).map(|()| Outcome::Accepted),
        Operation::Spelling(text) => identity_digest(text.to_owned()).map(|_| Outcome::Accepted),
        Operation::ReferenceExpected => reference_expected(case).map(|_| Outcome::Accepted),
        Operation::Distinct => require_distinct(case, item).map(|()| Outcome::Accepted),
        Operation::Evidence => check_evidence(item, &profile()).map(|()| Outcome::Accepted),
        Operation::UnicodeMultiset => {
            let pinned: serde_json::Value = serde_json::from_str(include_str!(
                "../../../tests/fixtures/reference-discrepancy-canonical.json"
            ))
            .unwrap();
            let mut report = case.expected.clone();
            report.files = vec![
                ryo::linter::RelativePath("雪/é.py".into()),
                ryo::linter::RelativePath("z.py".into()),
            ];
            let finding = ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding {
                rule: ryo::linter::RuleId::try_from("F821").unwrap(),
                file: report.files[0].clone(),
                start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(2),
                end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(7),
            };
            report.diagnostics = vec![finding.clone(), finding];
            Ok(equal(
                &output_digest(&report),
                &ArtifactDigest(pinned["sha256"].as_str().unwrap().to_owned()),
            ))
        }
    }
}

fn equal(left: &ArtifactDigest, right: &ArtifactDigest) -> Outcome {
    if left == right {
        Outcome::Equal
    } else {
        Outcome::Different
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
