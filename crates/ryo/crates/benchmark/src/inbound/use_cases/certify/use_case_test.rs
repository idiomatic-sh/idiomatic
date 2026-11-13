//! The public operation seals evidence and refuses changed inputs.
use super::fixtures::{Change, exercise};
use super::use_case::CertifyUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = Change;
type Value = ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("narrowed benchmark launches neither contestant"),
        word: Word::Refuses,
        input: || Change::InventoryNarrow,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("complete benchmark executes both contestants"),
        word: Word::Succeeds,
        input: || Change::InventoryFull,
        expect: Expect::Value(
            ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount(2),
        ),
    },
    Case {
        name: CaseName("execution mode mismatch refuses before launch"),
        word: Word::Refuses,
        input: || Change::Instrumentation,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("collector pin mismatch refuses before launch"),
        word: Word::Refuses,
        input: || Change::CollectorPin,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("collector path mismatch refuses before launch"),
        word: Word::Refuses,
        input: || Change::CollectorPath,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("unchanged certified workload measures"),
        word: Word::Succeeds,
        input: || Change::None,
        expect: Expect::Value(
            ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount(2),
        ),
    },
    Case {
        name: CaseName("patch mutation refuses measurement"),
        word: Word::Refuses,
        input: || Change::Patch,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("project pin mutation refuses measurement"),
        word: Word::Refuses,
        input: || Change::Manifest,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("changed collector refuses before execution"),
        word: Word::Refuses,
        input: || Change::Collector,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("query mutation refuses measurement"),
        word: Word::Refuses,
        input: || Change::Query,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("executable mutation refuses measurement"),
        word: Word::Refuses,
        input: || Change::Executable,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("invocation change refuses measurement"),
        word: Word::Refuses,
        input: || Change::Command,
        expect: Expect::Error(()),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input, CertifyUseCaseImpl::new)
}

mod registered;
mod substituted;
