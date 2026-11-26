//! The `names` service: free functions named by verb over the domain's types, with no IO and no ports. A name a row holds is made from
//! the tokens of the file, in the file's names; what cannot be resolved statically is the unknown value of its reason.

use ryo::languages::{self, Symbol};












/// A name that no file writes, as a name of the run: the path of a module a manifest names.
pub(crate) fn named(symbols: &mut languages::Symbols, text: languages::NameText<'_>) -> Symbol {
    languages::intern_derived(symbols, languages::ByteSlice(&[]), text)
}
