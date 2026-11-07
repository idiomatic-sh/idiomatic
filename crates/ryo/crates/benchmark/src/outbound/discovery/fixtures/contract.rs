//! What every `discovery` implementation must pass.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::discovery::interface::DiscoveryPort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::path::Path;

/// Which implementation a row asks: one that finds files, or one that finds none.
#[derive(Clone, Copy, Debug)]
pub enum Implementation {
    /// The implementation was given two files.
    Populated,
    /// The implementation was given none.
    Empty,
}
/// The implementation a row exercises.
type Input = Implementation;
/// How many files a row expects.
type Value = usize;
/// What a contract row is refused with.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The implementation refused an empty selection as invalid.
    Invalid,
    /// The implementation failed for another reason.
    Other,
}
type Failure = Refusal;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a discovery lists its files sorted"),
        word: Word::Succeeds,
        input: || Implementation::Populated,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a discovery that finds no file is refused"),
        word: Word::Refuses,
        input: || Implementation::Empty,
        expect: Expect::Error(Refusal::Invalid),
    },
];

/// Run the shared rows against one implementation that finds two files under `root` and one that finds none.
///
/// # Panics
///
/// Panics when an implementation violates a contract row.
pub fn check(populated: &impl DiscoveryPort, empty: &impl DiscoveryPort, root: &Path) {
    let subject = |input: Implementation| -> Result<Value, Failure> {
        let port: &dyn DiscoveryPort = match input {
            Implementation::Populated => populated,
            Implementation::Empty => empty,
        };
        let files = port.files(root).map_err(|error| match error {
            Error::Invalid(_) => Refusal::Invalid,
            Error::Io(_) | Error::Json(_) | Error::Toml(_) => Refusal::Other,
        })?;
        assert!(files.windows(2).all(|pair| pair[0] <= pair[1]));
        Ok(files.len())
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
