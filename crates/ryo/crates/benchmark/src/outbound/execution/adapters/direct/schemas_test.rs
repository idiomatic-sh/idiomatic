//! Independent collector records preserve byte units and refuse duration overflow.
use super::ResourcesSchema;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::measurements::types::Resources;
use std::time::Duration;
type Input = wait4::ResourceUsage;
type Value = Resources;
type Failure = bool;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("both CPU clocks sum without changing resident byte units"),
        word: Word::Succeeds,
        input: || wait4::ResourceUsage {
            utime: Duration::from_millis(125),
            stime: Duration::from_millis(75),
            maxrss: 12345,
        },
        expect: Expect::Value(Resources {
            cpu: Duration::from_millis(200),
            peak_rss_bytes: crate::domain::measurements::types::ResidentBytes(12345),
        }),
    },
    Case {
        name: CaseName("overflow cannot become a small CPU measurement"),
        word: Word::Refuses,
        input: || wait4::ResourceUsage {
            utime: Duration::MAX,
            stime: Duration::from_nanos(1),
            maxrss: 0,
        },
        expect: Expect::Error(true),
    },
];
fn subject(record: Input) -> Result<Value, Failure> {
    ResourcesSchema(record).try_into().map_err(|_| true)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
