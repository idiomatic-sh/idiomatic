//! How each kind of the language's trees is shown: the source whose tree holds it, or the reason no relation reads it.

use ryo::languages::contract::KindSample;
use ryo::languages::{KindName, Sample};

use crate::domain::syntax::service::{kind_bits, kinds};
use crate::domain::syntax::types::Kind;

mod extra;
mod node;
mod token;

/// Every kind, in the order of their numbers, with how the language shows it.
pub(super) fn shown() -> Vec<KindSample> {
    kinds()
        .map(|kind| KindSample {
            kind: KindName(format!("{kind:?}").into()),
            bits: kind_bits(kind),
            sample: sample(kind),
        })
        .collect()
}

/// How the language shows a kind of its trees: the source whose tree holds it, for a kind a relation reads, and for every other the
/// reason no relation models it. Each arm hands the kind to its own module's match, which has no wildcard arm, so a kind the grammar
/// gains is a compile error until it is shown.
#[must_use]
pub(crate) const fn sample(kind: Kind) -> Sample {
    match kind {
        Kind::Node(node) => node::sample(node),
        Kind::Token(token) => token::sample(token),
        Kind::Extra(extra) => extra::sample(extra),
    }
}
