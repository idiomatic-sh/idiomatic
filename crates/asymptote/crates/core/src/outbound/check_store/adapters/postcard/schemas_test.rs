//! The store's tables carry a checked product back whole, each text once, and a store whose rows name a text it does
//! not hold is refused.
use super::{KeptSchema, StoreSchema};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::check::{fixtures, interface::CheckUseCase as _};
use crate::outbound::check_store::types::{Build, Stored};

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
    /// Truncated or extra extension bytes cannot fabricate a valid report.
    DamagedDiagnostics,
    /// Whether every text of the store is held once.
    Once,
    /// Whether a store whose texts were dropped is refused.
    Unnamed,
    /// Whether the store a failing check left, a failed run beside the completed run it replaced, reads back as written.
    AfterFailure,
}
type Input = Question;
type Value = bool;
/// Why the store could not be written.
type Failure = String;
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
        name: CaseName("malformed_diagnostic_extensions_are_refused"),
        word: Word::Succeeds,
        input: || Question::DamagedDiagnostics,
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
];

fn subject(question: Input) -> Result<Value, Failure> {
    if matches!(
        question,
        Question::Diagnostics
            | Question::LegacyDiagnostics
            | Question::EmptyDiagnostics
            | Question::DamagedDiagnostics
    ) {
        return diagnostics(question);
    }
    let scenario = fixtures::scenario();
    drop(
        fixtures::use_case(&scenario)
            .check(fixtures::request())
            .map_err(|error| error.to_string())?,
    );
    let scenario = if matches!(question, Question::AfterFailure) {
        scenario.sources.write("src/a.rs", "fn a() {}\n");
        let failing = fixtures::with_sarif(
            scenario,
            crate::outbound::sarif::fixtures::fake::SarifFake::default(),
        );
        drop(
            fixtures::use_case(&failing)
                .check(fixtures::request())
                .map_err(|error| error.to_string())?,
        );
        failing
    } else {
        scenario
    };
    let stored = scenario.store.kept();
    let build = Build("build-1".into());
    let bytes = postcard::to_allocvec(&StoreSchema::from(KeptSchema(&stored, &build)))
        .map_err(|error| error.to_string())?;
    let read: StoreSchema = postcard::from_bytes(&bytes).map_err(|error| error.to_string())?;
    match question {
        Question::RoundTrip | Question::AfterFailure => {
            // Read back and written again, the store is the same bytes: nothing changes shape on the way.
            let again = Stored::try_from(read).map_err(|()| "the store reads back".to_owned())?;
            let rewritten = postcard::to_allocvec(&StoreSchema::from(KeptSchema(&again, &build)))
                .map_err(|error| error.to_string())?;
            // A failed run is kept only to say that it failed, not what it said: after a failing run the bytes are the same,
            // the failure's text is not.
            let same = matches!(question, Question::AfterFailure) || again == stored;
            Ok(same && rewritten == bytes)
        }
        Question::Diagnostics
        | Question::LegacyDiagnostics
        | Question::EmptyDiagnostics
        | Question::DamagedDiagnostics => diagnostics(question),
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
    use crate::domain::judgement::types::{Failure as RunFailure, Problem, Tail};
    use crate::domain::renames::types::Head;
    use crate::domain::reports::types::{DiagnosticText, Digest, FailureOutput, Report, Stamped};
    use crate::domain::violations::types::ToolId;
    use crate::outbound::check_store::types::KeptReport;
    let mut scan = crate::domain::reports::service::failed(
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
    let bytes = super::write(&stored, &build).map_err(|error| error.to_string())?;
    match question {
        Question::Diagnostics | Question::EmptyDiagnostics => {
            Ok(super::read(&bytes, &build) == Some(stored))
        }
        Question::LegacyDiagnostics => {
            let old = postcard::to_allocvec(&StoreSchema::from(KeptSchema(&stored, &build)))
                .map_err(|error| error.to_string())?;
            let loaded = super::read(&old, &build).ok_or("legacy record")?;
            let KeptReport::Kept(report) = loaded.report else {
                return Err("legacy report".into());
            };
            Ok(report.scanned.iter().all(|loaded_scan| {
                matches!(loaded_scan.failure_output, FailureOutput::Unavailable)
                    && matches!(
                        loaded_scan.run.status,
                        crate::domain::judgement::types::Status::Failed(_)
                    )
            }))
        }
        Question::DamagedDiagnostics => {
            let mut extra = bytes.clone();
            extra.push(1);
            let mut truncated = bytes;
            let _removed = truncated.pop();
            Ok(super::read(&extra, &build).is_none() && super::read(&truncated, &build).is_none())
        }
        Question::RoundTrip | Question::AfterFailure | Question::Once | Question::Unnamed => {
            Err("a diagnostic case is required".into())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
