//! The `names` service: free functions named by verb over the domain's types, with no IO and no ports. A name a row holds is made
//! from the tokens of the file, in the file's names; what cannot be resolved statically is the unknown value of its reason.

use ryo::languages::{self, Symbol};


use super::types::{Unresolved};









/// The unknown value of `reason`: the same word in every file.
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "a reason is a field-less enum with a u8 representation: its number is the cast"
)]
pub(crate) const fn unknown(reason: Unresolved) -> Symbol {
    languages::unknown_symbol(languages::UnknownReason(reason as u8))
}
