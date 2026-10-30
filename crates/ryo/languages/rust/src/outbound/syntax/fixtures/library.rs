//! The library of the `ra_ap_syntax` adapter, as the conformance reads it: the walk of its own tree, and its reading of the items of a file.

use ryo::languages::contract::{LibraryWalk, Oracle};

/// How the library walks its own tree of a file: what the conformance holds our tree to the shape of.
#[must_use]
pub(crate) fn walk() -> LibraryWalk {
    crate::outbound::syntax::adapters::ra_ap_syntax::fixtures::walk::walked
}

/// How the library reads the items of a file: what the conformance holds the rows to.
#[must_use]
pub(crate) fn oracle() -> Oracle {
    crate::outbound::syntax::adapters::ra_ap_syntax::fixtures::oracle::oracle
}
