//! Category totals, metadata and overflow stay bounded across repeated leases.
use super::fixtures::{Observed, Overflow, Repeated, Turn, overflow, repeated};
use crate::application::file_dag::accounting::ledger::types::ByteCount;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::borrow::Cow;

/// The leases a case takes.
#[derive(Clone, Copy, Debug)]
enum Leased {
    /// One optional lease of 20 and 30 bytes, taken and released sixty-four times.
    Repeated,
    /// A required lease whose bytes overflow, and position units that overflow as optional.
    Overflowing,
}

type Input = Leased;
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("category_totals_and_metadata_remain_bounded_across_repeated_leases"),
        word: Word::Succeeds,
        input: || Leased::Repeated,
        expect: Expect::Value(Observed::Repeated(Repeated {
            turns: 64,
            distinct: Cow::Borrowed(&[Turn {
                live: (ByteCount(50), ByteCount(50)),
                metadata_charged: true,
                released: (ByteCount(0), ByteCount(0)),
                peak_metadata_as_first: true,
            }]),
        })),
    },
    Case {
        name: CaseName("overflow_refuses_without_mutating_live_accounting"),
        word: Word::Succeeds,
        input: || Leased::Overflowing,
        expect: Expect::Value(Observed::Overflow(Overflow {
            bytes: Some(OperatingError::ShapeMismatch),
            incremental_units: Some(OperatingError::ShapeMismatch),
            units: Some(OperatingError::ShapeMismatch),
            unchanged: true,
        })),
    },
];

/// Takes the input's leases and answers what the ledger showed.
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(match input {
        Leased::Repeated => Observed::Repeated(repeated()?),
        Leased::Overflowing => Observed::Overflow(overflow()),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
