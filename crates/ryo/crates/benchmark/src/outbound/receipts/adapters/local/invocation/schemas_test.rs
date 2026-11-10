//! What the invocation document writes, one row of `CASES` for each input.
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::{
    FailedInvocation, InvocationDocument,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::ExitStatus;
use ryo_benchmark_contracts::domain::documents::types::text::value::{PathText, ReasonText, YesNo};
use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Signal};
use std::collections::BTreeMap;

type Input = Signal;
type Value = bool;
type Failure = ();

/// A capture that could not record its process identity is written with its exit and signal.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an unsignalled failed capture writes a null signal"),
    word: Word::Succeeds,
    input: || Signal::Absent,
    expect: Expect::Value(true),
}];

/// Encodes one failed capture and answers whether it wrote a null signal.
fn subject(signal: Input) -> Result<Value, Failure> {
    let document = InvocationDocument::Failed(FailedInvocation {
        actual_request_root: PathText("/root".into()),
        files: Vec::new(),
        arguments: Vec::new(),
        environment: BTreeMap::new(),
        directory: PathText("/root".into()),
        exit: ExitStatus(2),
        signal,
        observer_no_deadline: YesNo::Yes,
        metadata_error: ReasonText("failed".into()),
        source_sha256: Pin::Absent,
        file_metadata_sha256: Pin::Absent,
    });
    let bytes = super::bytes(&document).map_err(|_refused| ())?;
    let row: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_refused| ())?;
    Ok(row["signal"].is_null())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
