//! The Linux adapter reads `jeprof`'s total, wraps the worker in `perf record`, and keeps a run's files on disk.
use super::adapter::{ProfileToolsLinuxAdapter, total};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::profiles::types::SamplingRate;
use crate::outbound::profile_tools::fixtures::contract::check;
use crate::outbound::profile_tools::interface::ProfileToolsPort as _;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use std::borrow::Cow;
use std::collections::BTreeMap;

/// `jeprof --text --show_bytes` over a dump of a tikv-jemalloc 0.6.1 worker, by Debian's jeprof 5.3.0: its first lines.
const CAPTURED_TOTAL: &str = "Total: 524800 B
  524800 100.0% 100.0%   524800 100.0% prof_backtrace_impl
       0   0.0% 100.0%   524800 100.0% ::call
";

/// What a row reads or builds.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    CapturedTotal,
    NoTotal,
    PerfRecord,
    Contract,
}
type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the total of a captured jeprof report is read"),
        word: Word::Succeeds,
        input: || Scenario::CapturedTotal,
        expect: Expect::Value(Cow::Borrowed("HeapBytes(524800)")),
    },
    Case {
        name: CaseName("the worker runs under perf record at the rate, into the recording"),
        word: Word::Succeeds,
        input: || Scenario::PerfRecord,
        expect: Expect::Value(Cow::Borrowed(
            r#""perf" ["record", "-e", "cpu-clock:u", "-F", "22", "--call-graph", "dwarf,65528", "--user-callchains", "-o", "out/cpu/perf.data", "--", "target/release/deps/worker", "run-ryo", "request.json"]"#,
        )),
    },
    Case {
        name: CaseName("the Linux adapter satisfies the profile tools contract"),
        word: Word::Succeeds,
        input: || Scenario::Contract,
        expect: Expect::Value(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("a jeprof report without a total is refused"),
        word: Word::Refuses,
        input: || Scenario::NoTotal,
        expect: Expect::Error(Cow::Borrowed("jeprof printed no total")),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let answer = match scenario {
        Scenario::CapturedTotal => format!(
            "{:?}",
            total(CAPTURED_TOTAL.as_bytes()).map_err(|error| Cow::Owned(error.to_string()))?
        ),
        Scenario::NoTotal => format!(
            "{:?}",
            total(b"  0 0.0% 0.0% 0 0.0% ::call\n")
                .map_err(|error| Cow::Owned(error.to_string()))?
        ),
        Scenario::PerfRecord => {
            let profiled = ProfileToolsLinuxAdapter.cpu_invocation(
                Invocation {
                    executable: "target/release/deps/worker".into(),
                    arguments: vec!["run-ryo".into(), "request.json".into()],
                    environment: BTreeMap::new(),
                    directory: ".".into(),
                },
                SamplingRate(22),
                std::path::Path::new("out/cpu/perf.data"),
            );
            format!("{:?} {:?}", profiled.executable, profiled.arguments)
        }
        Scenario::Contract => {
            let root = tempfile::tempdir().map_err(|error| Cow::Owned(error.to_string()))?;
            check(&ProfileToolsLinuxAdapter, root.path());
            String::new()
        }
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
