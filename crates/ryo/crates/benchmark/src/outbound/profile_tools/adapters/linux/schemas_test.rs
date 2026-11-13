//! The receipt is written as one JSON document with the peak and the rate.
use super::schemas::document;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::profiles::types::{DumpTotal, HeapBytes, ProfileReceipt, SamplingRate};
use std::borrow::Cow;

type Input = ProfileReceipt;
type Value = Cow<'static, str>;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the receipt names the directory, the peak dump, its bytes and the rate"),
    word: Word::Succeeds,
    input: || ProfileReceipt {
        directory: "out".into(),
        peak: DumpTotal {
            dump: "out/heap/dumps/jeprof.7.3.f.heap".into(),
            bytes: HeapBytes(524_800),
        },
        rate: SamplingRate(22),
    },
    expect: Expect::Value(Cow::Borrowed(
        r#"{
  "directory": "out",
  "peak_dump": "out/heap/dumps/jeprof.7.3.f.heap",
  "peak_bytes": 524800,
  "sampling_rate": 22
}"#,
    )),
}];

fn subject(receipt: Input) -> Result<Value, Failure> {
    let written = document(&receipt).map_err(|error| Cow::Owned(error.to_string()))?;
    Ok(Cow::Owned(String::from_utf8_lossy(&written.0).into_owned()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
