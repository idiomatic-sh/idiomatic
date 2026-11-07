//! The library of the `oxc_parser` adapter, as the conformance reads it: the walk of its own tree, and its own reading of the file.

use ryo::languages::contract::{Oracle, ReferenceRefusal, Walked};

/// How the library walks its own tree of a file: what the conformance holds our tree to the shape of.
#[must_use]
pub(crate) fn walk() -> fn(&str) -> Result<Walked, ReferenceRefusal> {
    crate::outbound::syntax::adapters::oxc_parser::fixtures::walk::reference
}

/// How the library reads the document of a file: what the conformance holds the rows to.
#[must_use]
pub(crate) fn oracle() -> Oracle {
    crate::outbound::syntax::adapters::oxc_parser::fixtures::oracle::oracle
}
