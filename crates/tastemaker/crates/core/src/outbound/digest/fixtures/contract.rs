//! What every `digest` adapter must pass.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::findings::types::{Identifying, IdentityParts, Ordinal};
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::DigestParts;

/// Two lists of parts, or two findings with the same parts at two ordinals.
#[derive(Clone, Copy, Debug)]
pub enum Pair {
    /// The parts of two digests.
    Parts(&'static [&'static [u8]], &'static [&'static [u8]]),
    /// The ordinals of two findings with the same rule, path and line.
    Ordinals(u64, u64),
}

/// What a row asks.
type Input = Pair;
/// Whether they come to the same digest, or the same fingerprint.
type Value = bool;
/// A row of this contract never fails.
type Failure = ();

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_same_parts_come_to_the_same_digest"),
        word: Word::Succeeds,
        input: || Pair::Parts(&[b"a", b"b"], &[b"a", b"b"]),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("parts_split_differently_come_to_different_digests"),
        word: Word::Succeeds,
        input: || Pair::Parts(&[b"ab"], &[b"a", b"b"]),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_finding_at_the_same_ordinal_has_the_same_fingerprint"),
        word: Word::Succeeds,
        input: || Pair::Ordinals(1, 1),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_later_finding_with_the_same_parts_has_another_fingerprint"),
        word: Word::Succeeds,
        input: || Pair::Ordinals(0, 1),
        expect: Expect::Value(false),
    },
];

/// Whether `adapter` brings the two sides of `pair` to the same digest or fingerprint.
fn answer(adapter: &impl DigestPort, pair: Input) -> Value {
    match pair {
        Pair::Parts(one, other) => {
            adapter.digest(DigestParts(one)) == adapter.digest(DigestParts(other))
        }
        Pair::Ordinals(one, other) => {
            let identifying = |ordinal| Identifying {
                parts: IdentityParts(["rule".into(), "path".into(), "line".into()]),
                ordinal: Ordinal(ordinal),
            };
            adapter.fingerprint(&identifying(one)) == adapter.fingerprint(&identifying(other))
        }
    }
}

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl DigestPort) {
    run(CASES, Word::Succeeds, |input| Ok(answer(adapter, input)));
}
