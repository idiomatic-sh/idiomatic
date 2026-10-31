//! Independently specified resource records reject missing and malformed observations.
use super as schemas;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::measurements::types::Resources;
use std::time::Duration;

type Input = &'static str;
type Value = Resources;
type Failure = bool;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("CPU sums both clocks and RSS converts KiB"),
        word: Word::Succeeds,
        input: || "1.25\t0.75\t2048\n",
        expect: Expect::Value(Resources {
            cpu: Duration::from_secs(2),
            peak_rss_bytes: 2_097_152,
        }),
    },
    Case {
        name: CaseName("missing metrics fail"),
        word: Word::Refuses,
        input: || "",
        expect: Expect::Error(true),
    },
    Case {
        name: CaseName("non finite CPU fails"),
        word: Word::Refuses,
        input: || "NaN\t0.0\t2048",
        expect: Expect::Error(true),
    },
    Case {
        name: CaseName("negative CPU fails"),
        word: Word::Refuses,
        input: || "-1.0\t0.0\t2048",
        expect: Expect::Error(true),
    },
    Case {
        name: CaseName("RSS byte overflow fails"),
        word: Word::Refuses,
        input: || "0.0\t0.0\t18446744073709551615",
        expect: Expect::Error(true),
    },
    Case {
        name: CaseName("extra records fail"),
        word: Word::Refuses,
        input: || "0.1\t0.1\t1\n0.2\t0.2\t2",
        expect: Expect::Error(true),
    },
];

fn subject(record: &str) -> Result<Resources, bool> {
    schemas::decode(record).map_err(|_| true)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
