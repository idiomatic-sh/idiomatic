//! The library of the `pulldown_cmark` adapter, as the conformance reads it: the walk of its own tree, and its own events of the
//! headings, links, code and emphasis of a file.

use ryo::languages::contract::{LibraryWalk, Oracle};

/// How the library walks its own tree of a file: what the conformance holds our tree to the shape of.
#[must_use]
pub(crate) fn walk() -> LibraryWalk {
    crate::outbound::syntax::adapters::pulldown_cmark::fixtures::walk::walked
}

/// How the library reads the headings, links, code and emphasis of a file: what the conformance holds the rows to.
#[must_use]
pub(crate) fn oracle() -> Oracle {
    crate::outbound::syntax::adapters::pulldown_cmark::fixtures::oracle::oracle
}
