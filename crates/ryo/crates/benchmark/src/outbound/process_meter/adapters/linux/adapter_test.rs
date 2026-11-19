//! The Linux meter answers the port's contract, and its reset lets go of what is no longer resident.
#![cfg(target_os = "linux")]
use super::adapter::ProcessMeterLinuxAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::process_meter::fixtures::contract::check;
use crate::outbound::process_meter::fixtures::fake::FREED;
use crate::outbound::process_meter::interface::ProcessMeterPort as _;

/// What a row asks of the meter.
#[derive(Clone, Copy, Debug)]
enum Probe {
    /// The port's contract.
    Contract,
    /// A block touched and freed before a reset: whether the peak read after the reset is below the one before it by at least
    /// half the block.
    FreedBeforeReset,
}

type Input = Probe;
type Value = bool;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the Linux meter answers the process meter contract"),
        word: Word::Succeeds,
        input: || Probe::Contract,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a reset drops a block freed before it from the peak"),
        word: Word::Succeeds,
        input: || Probe::FreedBeforeReset,
        expect: Expect::Value(true),
    },
];

/// The name this test's own run is told apart by: a second exact filter it is re-run with, which names no test.
const ALONE: &str = "alone";

/// This test's name in the test binary, and the filter that tells its own run apart.
fn names() -> (String, String) {
    let module = module_path!()
        .split_once("::")
        .map_or(module_path!(), |(_, path)| path);
    let test = format!("{module}::succeeds");
    let alone = format!("{test}::{ALONE}");
    (test, alone)
}

/// Each row reads the peak, which is the whole process's: in the test binary's shared process another test's thread may grow
/// it between a row's reset and its read. So a row is measured in a process the meter owns: this test binary re-run on this
/// test alone, on one thread, with the [`ALONE`] filter, where each row reads the peak in place.
fn subject(input: Input) -> Result<Value, Failure> {
    let (test, alone) = names();
    if std::env::args().any(|argument| argument == alone) {
        in_place(input)
    } else {
        own_process(&test, &alone)
    }
}

/// This test, re-run alone in a process of its own, where each row reads the peak in place: true when it passed, having run
/// exactly this one test.
fn own_process(test: &str, alone: &str) -> Result<Value, Failure> {
    let output =
        std::process::Command::new(std::env::current_exe().map_err(|error| error.to_string())?)
            .args(["--exact", test, alone, "--test-threads=1"])
            .output()
            .map_err(|error| error.to_string())?;
    let printed = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        return Err(format!(
            "the meter's own process ended with {}: {printed}{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(printed.contains("test result: ok. 1 passed;"))
}

/// The row read in this process, which is the meter's own.
fn in_place(input: Input) -> Result<Value, Failure> {
    let meter = ProcessMeterLinuxAdapter;
    match input {
        Probe::Contract => {
            check(&meter);
            Ok(true)
        }
        Probe::FreedBeforeReset => {
            drop(std::hint::black_box(vec![1_u8; FREED]));
            let before = meter.peak().map_err(|error| error.to_string())?;
            meter.reset_peak().map_err(|error| error.to_string())?;
            let after = meter.peak().map_err(|error| error.to_string())?;
            let half = u64::try_from(FREED / 2).map_err(|error| error.to_string())?;
            Ok(after.0.saturating_add(half) <= before.0)
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
