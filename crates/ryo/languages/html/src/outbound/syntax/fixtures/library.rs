//! The library of the `swc_html_parser` adapter, as the conformance reads it: the walk of its own tree, and its own reading of the file.

use ryo::languages::contract::{LibraryWalk, Oracle};

/// How the library walks its own tree of a file: what the conformance holds our tree to the shape of.
#[must_use]
pub(crate) fn walk() -> LibraryWalk {
    crate::outbound::syntax::adapters::swc_html_parser::fixtures::walk::walked
}

/// How the library reads the document of a file: what the conformance holds the rows to.
#[must_use]
pub(crate) fn oracle() -> Oracle {
    crate::outbound::syntax::adapters::swc_html_parser::fixtures::oracle::oracle
}
