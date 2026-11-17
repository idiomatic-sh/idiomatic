//! The Linux meter answers the port's contract, and its reset lets go of what is no longer resident.
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

fn subject(input: Input) -> Result<Value, Failure> {
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
