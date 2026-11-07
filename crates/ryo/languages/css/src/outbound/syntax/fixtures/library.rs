//! The library of the `cssparser` adapter, as the conformance reads it: the walk of its own tree, and its parsers' reading of the
//! rules of a stylesheet.

use ryo::languages::contract::{LibraryWalk, Oracle};

/// How the library walks its own tree of a file: what the conformance holds our tree to the shape of.
#[must_use]
pub(crate) fn walk() -> LibraryWalk {
    crate::outbound::syntax::adapters::cssparser::fixtures::walk::walked
}

/// How the library reads the rules of a file: what the conformance holds the rows to.
#[must_use]
pub(crate) fn oracle() -> Oracle {
    crate::outbound::syntax::adapters::cssparser::fixtures::oracle::oracle
}
