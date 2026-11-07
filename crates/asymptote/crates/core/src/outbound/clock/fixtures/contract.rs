//! What every `clock` adapter must pass: today is a day, `YYYY-MM-DD`.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::clock::interface::ClockPort;

/// Nothing: the clock is asked.
type Input = ();
/// Whether today is spelled as a day.
type Value = bool;
/// Asking the clock cannot fail, so no row has an error to expect.
type Failure = ();

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("today_is_a_day"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl ClockPort) {
    run(CASES, Word::Succeeds, |()| -> Result<Value, Failure> {
        let today = adapter.today();
        let parts: Vec<&str> = today.0.split('-').collect();
        Ok(parts.len() == 3
            && parts.iter().zip([4, 2, 2]).all(|(part, digits)| {
                part.len() == digits && part.bytes().all(|byte| byte.is_ascii_digit())
            }))
    });
}
