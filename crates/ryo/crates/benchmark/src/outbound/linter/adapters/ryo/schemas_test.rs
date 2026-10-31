//! Wire reports preserve the canonical fields and reject unknown or malformed input.
use super::decode_report;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::diagnostics::types::{Diagnostic, RunReport};
type Input = &'static [u8];
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("all canonical fields survive"),
        word: Word::Succeeds,
        input: || {
            br#"{"diagnostics":[{"rule":"R1","file":"input","start":2,"end":7}],"files":["input","clean"]}"#
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unknown fields are refused"),
        word: Word::Refuses,
        input: || br#"{"diagnostics":[],"files":[],"extra":true}"#,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("invalid json is refused"),
        word: Word::Refuses,
        input: || b"not-json",
        expect: Expect::Error(()),
    },
];
fn subject(bytes: Input) -> Result<Value, Failure> {
    decode_report(bytes)
        .map(|report| {
            report
                == RunReport {
                    diagnostics: vec![Diagnostic {
                        rule: "R1".into(),
                        file: "input".into(),
                        start: 2,
                        end: 7,
                    }],
                    files: vec!["input".into(), "clean".into()],
                }
        })
        .map_err(|_| ())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
