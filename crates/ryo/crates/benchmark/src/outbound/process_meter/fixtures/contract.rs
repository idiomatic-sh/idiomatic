//! What every `process_meter` implementation must pass.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::process_meter::interface::ProcessMeterPort;

type Input = ();
type Value = bool;
type Failure = String;

/// The contract's rows.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the CPU time of this process never decreases"),
        word: Word::Succeeds,
        input: || (),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "after a reset the peak is no greater than before it plus what was allocated after it",
        ),
        word: Word::Succeeds,
        input: || (),
        expect: Expect::Value(true),
    },
];

/// Run the rows against one implementation.
///
/// # Panics
///
/// Panics when an implementation violates a contract row.
pub fn check(port: &impl ProcessMeterPort) {
    let row = std::cell::Cell::new(0_usize);
    run(CASES, Word::Succeeds, |()| {
        row.set(row.get() + 1);
        if row.get() == 1 {
            let first = port.cpu().map_err(|error| error.to_string())?;
            let second = port.cpu().map_err(|error| error.to_string())?;
            Ok(second >= first)
        } else {
            let before = port.peak().map_err(|error| error.to_string())?;
            port.reset_peak().map_err(|error| error.to_string())?;
            // A block of memory allocated and touched after the reset, so a peak read after it may grow by its size and no more.
            let touched = std::hint::black_box(vec![1_u8; 1 << 20]);
            let after = port.peak().map_err(|error| error.to_string())?;
            Ok(after.0
                <= before
                    .0
                    .saturating_add(u64::try_from(touched.len()).unwrap_or(u64::MAX)))
        }
    });
}
