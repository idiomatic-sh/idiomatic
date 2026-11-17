//! The store's tables carry a checked product back whole, each text once, and a store whose rows name a text it does
//! not hold is refused.
use super::super::errors::CheckStorePostcardError;
use super::{KeptSchema, StoreSchema};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::outbound::check_store::fixtures::fake::checked;
use crate::outbound::check_store::types::{Build, KeptReport, Stored};
use asymptote_values::domain::judgement::types::{Code, Failure as RunFailure, Problem, Tail};
use asymptote_values::domain::reports::service::failed;
use asymptote_values::domain::reports::types::{FailureOutput, Keeping, Stamped};
use asymptote_values::domain::violations::types::ToolId;

#[derive(Clone, Copy, Debug)]
enum Question {
    /// Whether a checked product, its violations, inputs and file records among it, reads back as written.
    RoundTrip,
    /// Complete failure details and streams survive the extension.
    Diagnostics,
    /// An old record is explicitly unavailable, never captured empty.
    LegacyDiagnostics,
    /// Empty observed streams remain captured empty.
    EmptyDiagnostics,
    /// Whether every text of the store is held once.
    Once,
    /// Whether a store whose texts were dropped is refused.
    Unnamed,
    /// Whether the store a failing check left, a failed run beside the completed run it replaced, reads back as written.
    AfterFailure,
    /// Bytes that are no store.
    NoStore,
    /// A store another build wrote, read as this build's.
    OtherBuild,
    /// A store whose rows name the texts it dropped.
    Dropped,
    /// A store whose diagnostic extension lost its last byte.
    Truncated,
    /// A store with a byte after its diagnostic extension.
    Extra,
}
type Input = Question;
type Value = bool;
/// `read`'s refusal of the bytes a row damaged.
type Failure = CheckStorePostcardError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("failure_streams_and_exact_identity_round_trip"),
        word: Word::Succeeds,
        input: || Question::Diagnostics,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("legacy_failure_output_is_explicitly_unavailable"),
        word: Word::Succeeds,
        input: || Question::LegacyDiagnostics,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("observed_empty_streams_are_not_unavailable"),
        word: Word::Succeeds,
        input: || Question::EmptyDiagnostics,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_checked_product_reads_back_as_written"),
        word: Word::Succeeds,
        input: || Question::RoundTrip,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("every_text_is_held_once"),
        word: Word::Succeeds,
        input: || Question::Once,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_store_whose_rows_name_an_absent_text_is_refused"),
        word: Word::Succeeds,
        input: || Question::Unnamed,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_store_kept_after_a_failing_run_reads_back_as_written"),
        word: Word::Succeeds,
        input: || Question::AfterFailure,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("bytes_that_are_no_store_are_undecodable"),
        word: Word::Refuses,
        input: || Question::NoStore,
        expect: Expect::Refusal(|error| {
            selected(matches!(error, CheckStorePostcardError::Undecodable(_)))
        }),
    },
    Case {
        name: CaseName("a_store_another_build_wrote_is_that_builds"),
        word: Word::Refuses,
        input: || Question::OtherBuild,
        expect: Expect::Refusal(|error| {
            selected(matches!(error, CheckStorePostcardError::OtherBuild))
        }),
    },
    Case {
        name: CaseName("a_store_whose_rows_name_dropped_texts_is_unnamed"),
        word: Word::Refuses,
        input: || Question::Dropped,
        expect: Expect::Refusal(|error| {
            selected(matches!(error, CheckStorePostcardError::Unnamed))
        }),
    },
    Case {
        name: CaseName("a_truncated_diagnostic_extension_does_not_decode"),
        word: Word::Refuses,
        input: || Question::Truncated,
        expect: Expect::Refusal(|error| {
            selected(matches!(error, CheckStorePostcardError::Extension(_)))
        }),
    },
    Case {
        name: CaseName("a_byte_after_the_diagnostic_extension_is_damage"),
        word: Word::Refuses,
        input: || Question::Extra,
        expect: Expect::Refusal(|error| {
            selected(matches!(error, CheckStorePostcardError::Damaged))
        }),
    },
];

/// The row's selection from whether the error is the variant it names.
const fn selected(matched: bool) -> Selection {
    if matched {
        Selection::Matches
    } else {
        Selection::Differs
    }
}

fn subject(question: Input) -> Result<Value, Failure> {
    if matches!(
        question,
        Question::Diagnostics | Question::LegacyDiagnostics | Question::EmptyDiagnostics
    ) {
        return diagnostics(question);
    }
    if matches!(
        question,
        Question::NoStore
            | Question::OtherBuild
            | Question::Dropped
            | Question::Truncated
            | Question::Extra
    ) {
        return damaged(question).map(|_read| true);
    }
    let stored = if matches!(question, Question::AfterFailure) {
        // The failing run is current, and the completed run it replaced is retained beside it.
        let mut stored = checked();
        let KeptReport::Kept(report) = &mut stored.report else {
            panic!("the checked product keeps a report");
        };
        let report = std::sync::Arc::make_mut(report);
        for scan in &mut report.scanned {
            scan.keeping = Keeping::Retained;
        }
        report.scanned.push(failed(
            &ToolId("lint".into()),
            RunFailure::Exited {
                code: Code(1),
                stderr: Tail("the tool failed".into()),
            },
        ));
        stored
    } else {
        checked()
    };
    let build = Build("build-1".into());
    let bytes = postcard::to_allocvec(&StoreSchema::from(KeptSchema(&stored, &build)))
        .expect("the fixture is written and read back");
    let read: StoreSchema =
        postcard::from_bytes(&bytes).expect("the fixture is written and read back");
    match question {
        Question::RoundTrip | Question::AfterFailure => {
            // Read back and written again, the store is the same bytes: nothing changes shape on the way.
            let again = Stored::try_from(read).expect("the store reads back");
            let rewritten = postcard::to_allocvec(&StoreSchema::from(KeptSchema(&again, &build)))
                .expect("the fixture is written and read back");
            // A failed run is kept only to say that it failed, not what it said: after a failing run the bytes are the same,
            // the failure's text is not.
            let same = matches!(question, Question::AfterFailure) || again == stored;
            Ok(same && rewritten == bytes)
        }
        Question::Diagnostics | Question::LegacyDiagnostics | Question::EmptyDiagnostics => {
            diagnostics(question)
        }
        Question::NoStore
        | Question::OtherBuild
        | Question::Dropped
        | Question::Truncated
        | Question::Extra => damaged(question).map(|_read| true),
        Question::Once => {
            let texts = read.strings.len();
            let distinct: std::collections::BTreeSet<&String> = read.strings.iter().collect();
            Ok(texts > 0 && distinct.len() == texts)
        }
        Question::Unnamed => Ok(Stored::try_from(StoreSchema {
            strings: Vec::new(),
            ..read
        }) == Err(())),
    }
}

/// Exercise the production codec, including legacy bytes made with the unchanged original schema.
fn diagnostics(question: Question) -> Result<Value, Failure> {
    use asymptote_values::domain::renames::types::Head;
    use asymptote_values::domain::reports::types::{DiagnosticText, Digest, Report};
    let mut scan = failed(
        &ToolId("stdout-only".into()),
        RunFailure::Unreadable {
            problem: Problem("exact decoder refusal".into()),
            stderr: Tail("last line".into()),
        },
    );
    let empty = matches!(question, Question::EmptyDiagnostics);
    scan.failure_output = FailureOutput::Captured {
        stdout: DiagnosticText(
            if empty {
                String::new()
            } else {
                "first diagnostic\n".repeat(400)
            }
            .into(),
        ),
        stderr: DiagnosticText(
            if empty {
                String::new()
            } else {
                "early error\n".repeat(400)
            }
            .into(),
        ),
        inputs: Stamped::Digested(Digest("command-and-input-fingerprint".into())),
    };
    let stored = Stored {
        report: KeptReport::Kept(std::sync::Arc::new(Report {
            head: Head::Detached,
            scanned: vec![scan],
        })),
        ..Stored::default()
    };
    let build = Build("capture-build".into());
    let bytes = super::write(&stored, &build).expect("the fixture is written and read back");
    match question {
        Question::Diagnostics | Question::EmptyDiagnostics => {
            Ok(super::read(&bytes, &build).is_ok_and(|read| read == stored))
        }
        Question::LegacyDiagnostics => {
            let old = postcard::to_allocvec(&StoreSchema::from(KeptSchema(&stored, &build)))
                .expect("the fixture is written and read back");
            let loaded = super::read(&old, &build)?;
            let KeptReport::Kept(report) = loaded.report else {
                panic!("the legacy record keeps its report");
            };
            Ok(report.scanned.iter().all(|loaded_scan| {
                matches!(loaded_scan.failure_output, FailureOutput::Unavailable)
                    && matches!(
                        loaded_scan.run.status,
                        asymptote_values::domain::judgement::types::Status::Failed(_)
                    )
            }))
        }
        Question::RoundTrip
        | Question::AfterFailure
        | Question::Once
        | Question::Unnamed
        | Question::NoStore
        | Question::OtherBuild
        | Question::Dropped
        | Question::Truncated
        | Question::Extra => panic!("a diagnostic case is required"),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// What `read` answers for the bytes of the checked product beside a failed run, whose failure the diagnostic extension
/// carries, damaged as `question` says.
fn damaged(question: Question) -> Result<Stored, Failure> {
    let build = Build("build-1".into());
    let mut stored = checked();
    if let KeptReport::Kept(report) = &mut stored.report {
        std::sync::Arc::make_mut(report).scanned.push(failed(
            &ToolId("broken".into()),
            RunFailure::Exited {
                code: Code(2),
                stderr: Tail("the tool failed".into()),
            },
        ));
    }
    let mut bytes = super::write(&stored, &build).expect("the fixture is written and read back");
    let mut reader = build.clone();
    match question {
        Question::NoStore => bytes = vec![0xff; 4],
        Question::OtherBuild => reader = Build("build-2".into()),
        Question::Dropped => {
            bytes = postcard::to_allocvec(&StoreSchema {
                strings: Vec::new(),
                ..StoreSchema::from(KeptSchema(&stored, &build))
            })
            .expect("the fixture is written and read back");
        }
        Question::Truncated => drop(bytes.pop()),
        Question::Extra => bytes.push(1),
        Question::RoundTrip
        | Question::Diagnostics
        | Question::LegacyDiagnostics
        | Question::EmptyDiagnostics
        | Question::Once
        | Question::Unnamed
        | Question::AfterFailure => panic!("a damaged store is required"),
    }
    super::read(&bytes, &reader)
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
