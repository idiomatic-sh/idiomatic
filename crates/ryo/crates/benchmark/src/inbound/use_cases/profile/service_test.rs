//! The dumps are ordered as written, the peak is the largest, the rate is bounded, and a `profile-ryo` command line is split.
use super::service::{dump_order, heap_configuration, peak, sampling_rate, split_profile};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::command_line::types::BenchmarkArgument;
use crate::outbound::profile_tools::types::{DumpTotal, HeapBytes};
use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// What each row orders, picks, computes or splits.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    TenthIntervalAfterNinth,
    FinalDumpLast,
    LargestIsPeak,
    FirstOnTie,
    NoDumpNoPeak,
    ShortRunHighestRate,
    LongRunRate,
    VeryLongRunLowestRate,
    HeapConfiguration,
    SplitRequestAndDirectory,
    EmptyArguments,
    MissingDirectory,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the tenth interval dump comes after the ninth"),
        word: Word::Succeeds,
        input: || Scenario::TenthIntervalAfterNinth,
        expect: Expect::Value(Cow::Borrowed(
            r#"["d/jeprof.7.9.i9.heap", "d/jeprof.7.10.i10.heap"]"#,
        )),
    },
    Case {
        name: CaseName("the dump written at exit comes last"),
        word: Word::Succeeds,
        input: || Scenario::FinalDumpLast,
        expect: Expect::Value(Cow::Borrowed(
            r#"["d/jeprof.7.0.i0.heap", "d/jeprof.7.11.i11.heap", "d/jeprof.7.2.f.heap"]"#,
        )),
    },
    Case {
        name: CaseName("the peak is the largest dump"),
        word: Word::Succeeds,
        input: || Scenario::LargestIsPeak,
        expect: Expect::Value(Cow::Borrowed(
            r#"Some(DumpTotal { dump: "b", bytes: HeapBytes(9) })"#,
        )),
    },
    Case {
        name: CaseName("of two largest dumps the first in order is the peak"),
        word: Word::Succeeds,
        input: || Scenario::FirstOnTie,
        expect: Expect::Value(Cow::Borrowed(
            r#"Some(DumpTotal { dump: "b", bytes: HeapBytes(9) })"#,
        )),
    },
    Case {
        name: CaseName("no dump has no peak"),
        word: Word::Succeeds,
        input: || Scenario::NoDumpNoPeak,
        expect: Expect::Value(Cow::Borrowed("None")),
    },
    Case {
        name: CaseName("half a second on eight threads samples at the highest rate"),
        word: Word::Succeeds,
        input: || Scenario::ShortRunHighestRate,
        expect: Expect::Value(Cow::Borrowed("SamplingRate(999)")),
    },
    Case {
        name: CaseName("330 seconds on eight threads samples at 22 a second"),
        word: Word::Succeeds,
        input: || Scenario::LongRunRate,
        expect: Expect::Value(Cow::Borrowed("SamplingRate(22)")),
    },
    Case {
        name: CaseName("ten hours on 64 threads samples at the lowest rate"),
        word: Word::Succeeds,
        input: || Scenario::VeryLongRunLowestRate,
        expect: Expect::Value(Cow::Borrowed("SamplingRate(1)")),
    },
    Case {
        name: CaseName("the heap profiler is configured to dump under the prefix"),
        word: Word::Succeeds,
        input: || Scenario::HeapConfiguration,
        expect: Expect::Value(Cow::Borrowed(
            r#""prof:true,prof_active:true,lg_prof_sample:19,lg_prof_interval:30,prof_final:true,prof_prefix:out/heap/dumps/jeprof""#,
        )),
    },
    Case {
        name: CaseName(
            "a profile command line splits into the worker's arguments and the directory",
        ),
        word: Word::Succeeds,
        input: || Scenario::SplitRequestAndDirectory,
        expect: Expect::Value(Cow::Borrowed(
            r#"ProfileArguments { worker: [BenchmarkArgument("request.json"), BenchmarkArgument("python-target=3.12")], directory: "out" }"#,
        )),
    },
    Case {
        name: CaseName("a profile command line without arguments is refused"),
        word: Word::Refuses,
        input: || Scenario::EmptyArguments,
        expect: Expect::Error(Cow::Borrowed(
            "expected profile-ryo (<json>|--request-file <path>) [<config>] <dir>",
        )),
    },
    Case {
        name: CaseName("a profile command line without its directory is refused"),
        word: Word::Refuses,
        input: || Scenario::MissingDirectory,
        expect: Expect::Error(Cow::Borrowed(
            "expected the profile directory after the worker request",
        )),
    },
];

/// A dump of `bytes` named `name`.
fn total(name: &str, bytes: u64) -> DumpTotal {
    DumpTotal {
        dump: PathBuf::from(name),
        bytes: HeapBytes(bytes),
    }
}

/// The arguments `words` name.
fn arguments(words: &[&str]) -> Vec<BenchmarkArgument> {
    words
        .iter()
        .map(|word| BenchmarkArgument((*word).into()))
        .collect()
}

/// The rate of `seconds` (in thousandths) of wall time on `threads` threads.
fn rate(milliseconds: u64, threads: usize) -> Value {
    let threads = NonZeroUsize::new(threads).unwrap_or(NonZeroUsize::MIN);
    Cow::Owned(format!(
        "{:?}",
        sampling_rate(Duration::from_millis(milliseconds), threads)
    ))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let answer: String = match scenario {
        Scenario::TenthIntervalAfterNinth => format!(
            "{:?}",
            dump_order(vec![
                PathBuf::from("d/jeprof.7.10.i10.heap"),
                PathBuf::from("d/jeprof.7.9.i9.heap"),
            ])
        ),
        Scenario::FinalDumpLast => format!(
            "{:?}",
            dump_order(vec![
                PathBuf::from("d/jeprof.7.2.f.heap"),
                PathBuf::from("d/jeprof.7.11.i11.heap"),
                PathBuf::from("d/jeprof.7.0.i0.heap"),
            ])
        ),
        Scenario::LargestIsPeak => {
            format!("{:?}", peak(&[total("a", 3), total("b", 9), total("c", 5)]))
        }
        Scenario::FirstOnTie => {
            format!("{:?}", peak(&[total("a", 3), total("b", 9), total("c", 9)]))
        }
        Scenario::NoDumpNoPeak => format!("{:?}", peak(&[])),
        Scenario::ShortRunHighestRate => return Ok(rate(500, 8)),
        Scenario::LongRunRate => return Ok(rate(330_000, 8)),
        Scenario::VeryLongRunLowestRate => return Ok(rate(36_000_000, 64)),
        Scenario::HeapConfiguration => format!(
            "{:?}",
            heap_configuration(Path::new("out/heap/dumps/jeprof"))
        ),
        Scenario::SplitRequestAndDirectory => format!(
            "{:?}",
            split_profile(arguments(&["request.json", "python-target=3.12", "out"]))
                .map_err(|error| Cow::Owned(error.to_string()))?
        ),
        Scenario::EmptyArguments => format!(
            "{:?}",
            split_profile(Vec::new()).map_err(|error| Cow::Owned(error.to_string()))?
        ),
        Scenario::MissingDirectory => format!(
            "{:?}",
            split_profile(arguments(&["request.json"]))
                .map_err(|error| Cow::Owned(error.to_string()))?
        ),
    };
    Ok(Cow::Owned(answer))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
