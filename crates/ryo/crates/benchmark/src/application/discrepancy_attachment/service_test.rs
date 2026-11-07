//! Behavioral proof of `discrepancy_attachment`: authored reference wires become canonical case expectations.
use super::errors::DiscrepancyAttachmentError;
use super::fixtures::{
    Attachment, Profile, discrepancy, language, language_evidence, prior_worker, public_contract,
    wire, with,
};
use super::service::attach;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::errors::DiscrepancyError;
use crate::domain::discrepancy::fixtures::case;
use crate::domain::registered_inputs::types::RefusalText;
use crate::outbound::suite::fixtures::fake::SuiteFake;
use crate::outbound::suite::types::discrepancy::{DiscrepancyBasisWire, EvidenceWire};
use crate::outbound::suite::types::process::ProcessFailureWire;
use std::path::Path;

/// What an accepted attachment answers.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Outcome {
    Attached,
}

type Input = Attachment;
type Value = Outcome;
type Failure = DiscrepancyAttachmentError;

fn process() -> ProcessFailureWire {
    ProcessFailureWire::ProcessFailure {
        exit: 2,
        stdout: Vec::new(),
        stderr: Vec::new(),
        observations: Vec::new(),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a language defect wire attaches to its case"),
        word: Word::Succeeds,
        input: language,
        expect: Expect::Value(Outcome::Attached),
    },
    Case {
        name: CaseName("a tool contract wire with its two evidence rows attaches"),
        word: Word::Succeeds,
        input: || {
            with(DiscrepancyBasisWire::ToolContract, |item| {
                item.evidence = vec![public_contract(), prior_worker()];
            })
        },
        expect: Expect::Value(Outcome::Attached),
    },
    Case {
        name: CaseName("rules without a profile directory name no case profile"),
        word: Word::Refuses,
        input: || Attachment {
            rules: "",
            ..language()
        },
        expect: Expect::Error(DiscrepancyAttachmentError::MissingCaseProfileBoundary),
    },
    Case {
        name: CaseName("a suite that serves no profile refuses with its own words"),
        word: Word::Refuses,
        input: || Attachment {
            profile: Profile::Missing,
            ..language()
        },
        expect: Expect::Error(DiscrepancyAttachmentError::Refused(RefusalText(
            String::new(),
        ))),
    },
    Case {
        name: CaseName("two expectations for one case are refused"),
        word: Word::Refuses,
        input: || Attachment {
            wires: vec![
                wire(DiscrepancyBasisWire::LanguageDefect, discrepancy()),
                wire(DiscrepancyBasisWire::LanguageDefect, discrepancy()),
            ],
            ..language()
        },
        expect: Expect::Error(DiscrepancyAttachmentError::DuplicateExpectationIdentity),
    },
    Case {
        name: CaseName("an expectation for no executable case is refused"),
        word: Word::Refuses,
        input: || {
            let mut other = wire(DiscrepancyBasisWire::LanguageDefect, discrepancy());
            other.name = "elsewhere".into();
            Attachment {
                wires: vec![other],
                ..language()
            }
        },
        expect: Expect::Error(DiscrepancyAttachmentError::ExpectationOutsideInventory),
    },
    Case {
        name: CaseName("a case that already carries an expectation is refused"),
        word: Word::Refuses,
        input: || Attachment {
            case: case(),
            ..language()
        },
        expect: Expect::Error(DiscrepancyAttachmentError::DuplicateCanonicalExpectation),
    },
    Case {
        name: CaseName("a process expectation cannot also declare diagnostics"),
        word: Word::Refuses,
        input: || {
            with(DiscrepancyBasisWire::LanguageDefect, |item| {
                item.process = Some(process())
            })
        },
        expect: Expect::Error(DiscrepancyAttachmentError::ProcessAlsoDeclaresDiagnostics),
    },
    Case {
        name: CaseName("a diagnostic in a file the case does not request is refused"),
        word: Word::Refuses,
        input: || {
            with(DiscrepancyBasisWire::LanguageDefect, |item| {
                item.expected[0].file = "other.py".into()
            })
        },
        expect: Expect::Error(DiscrepancyAttachmentError::InvalidSourceRange),
    },
    Case {
        name: CaseName("a diagnostic range that ends before it starts is refused"),
        word: Word::Refuses,
        input: || {
            with(DiscrepancyBasisWire::LanguageDefect, |item| {
                item.expected[0].start = 5
            })
        },
        expect: Expect::Error(DiscrepancyAttachmentError::InvalidSourceRange),
    },
    Case {
        name: CaseName("a tool contract difference cannot be a process failure"),
        word: Word::Refuses,
        input: || {
            with(DiscrepancyBasisWire::ToolContract, |item| {
                item.expected.clear();
                item.process = Some(process());
            })
        },
        expect: Expect::Error(DiscrepancyAttachmentError::ToolContractNeedsDiagnostics),
    },
    Case {
        name: CaseName("language evidence without a rationale is refused"),
        word: Word::Refuses,
        input: || {
            with(DiscrepancyBasisWire::LanguageDefect, |item| {
                item.evidence = vec![match language_evidence() {
                    EvidenceWire::LanguageSpec {
                        path,
                        sha256,
                        language_version,
                        target_configuration,
                        ..
                    } => EvidenceWire::LanguageSpec {
                        path,
                        sha256,
                        language_version,
                        target_configuration,
                        version_rationale: " ".into(),
                    },
                    other => other,
                }];
            })
        },
        expect: Expect::Error(DiscrepancyAttachmentError::LanguageEvidenceNeedsContract),
    },
    Case {
        name: CaseName("tool contract evidence without a pinned revision is refused"),
        word: Word::Refuses,
        input: || {
            with(DiscrepancyBasisWire::ToolContract, |item| {
                item.evidence = vec![
                    match public_contract() {
                        EvidenceWire::PublicToolContract {
                            path,
                            sha256,
                            source_path,
                            rationale,
                            ..
                        } => EvidenceWire::PublicToolContract {
                            path,
                            sha256,
                            source_path,
                            rationale,
                            source_revision: "abc".into(),
                        },
                        other => other,
                    },
                    prior_worker(),
                ];
            })
        },
        expect: Expect::Error(DiscrepancyAttachmentError::ToolContractEvidenceNeedsSource),
    },
    Case {
        name: CaseName("prior worker evidence without a named control is refused"),
        word: Word::Refuses,
        input: || {
            with(DiscrepancyBasisWire::ToolContract, |item| {
                item.evidence = vec![
                    public_contract(),
                    match prior_worker() {
                        EvidenceWire::PriorWorkerRun {
                            path,
                            sha256,
                            worker_sha256,
                            control_source_sha256,
                            control_query_sha256,
                            control_profile_sha256,
                            rationale,
                            ..
                        } => EvidenceWire::PriorWorkerRun {
                            path,
                            sha256,
                            worker_sha256,
                            control_source_sha256,
                            control_query_sha256,
                            control_profile_sha256,
                            rationale,
                            witness_case: " ".into(),
                        },
                        other => other,
                    },
                ];
            })
        },
        expect: Expect::Error(DiscrepancyAttachmentError::PriorWorkerEvidenceNeedsControl),
    },
    Case {
        name: CaseName("language evidence cannot support a tool contract difference"),
        word: Word::Refuses,
        input: || with(DiscrepancyBasisWire::ToolContract, |_| {}),
        expect: Expect::Error(DiscrepancyAttachmentError::EvidenceDoesNotMatchAuthority),
    },
    Case {
        name: CaseName("a tool contract difference needs both of its evidence rows"),
        word: Word::Refuses,
        input: || {
            with(DiscrepancyBasisWire::ToolContract, |item| {
                item.evidence = vec![public_contract()]
            })
        },
        expect: Expect::Error(DiscrepancyAttachmentError::ToolContractNeedsTwoEvidenceRows),
    },
    Case {
        name: CaseName("an uppercase certificate identity is refused"),
        word: Word::Refuses,
        input: || {
            with(DiscrepancyBasisWire::LanguageDefect, |item| {
                item.identity.profile_sha256 = "A".repeat(64)
            })
        },
        expect: Expect::Error(DiscrepancyAttachmentError::Identity(
            DiscrepancyError::IdentityNotLowercaseSha256,
        )),
    },
];

/// A refusal in a library's words is one kind of refusal, whatever the words.
fn words_dropped(error: Failure) -> Failure {
    match error {
        DiscrepancyAttachmentError::Refused(_) => {
            DiscrepancyAttachmentError::Refused(RefusalText(String::new()))
        }
        other => other,
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    let suite = SuiteFake {
        profile: match input.profile {
            Profile::Served => Some(crate::domain::discrepancy::fixtures::profile()),
            Profile::Missing => None,
        },
        reference_wires: input.wires,
        ..SuiteFake::default()
    };
    let mut cases = vec![input.case];
    attach(&suite, Path::new(input.rules), &mut cases).map_err(words_dropped)?;
    assert!(cases[0].reference_discrepancy.is_some());
    Ok(Outcome::Attached)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
