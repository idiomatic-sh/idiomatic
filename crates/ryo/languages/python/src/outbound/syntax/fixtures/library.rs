//! The library of the `ruff_python_parser` adapter, as the conformance reads it: the walk of its own tree, and its own nodes compared
//! with the rows.

use ryo::languages::contract::{LibraryWalk, Oracle};

/// How the library walks its own tree of a file: what the conformance holds our tree to the shape of.
#[must_use]
pub(crate) fn walk() -> LibraryWalk {
    crate::outbound::syntax::adapters::ruff_python_parser::fixtures::walk::walked
}

/// How the library reads the document of a file: what the conformance holds the rows to.
#[must_use]
pub(crate) fn oracle() -> Oracle {
    crate::outbound::syntax::adapters::ruff_python_parser::fixtures::oracle::oracle
}

/// The independent shape reference for an already validated fixture target.
pub(crate) fn walk_for<const MINOR: u8>() -> LibraryWalk {
    crate::outbound::syntax::adapters::ruff_python_parser::fixtures::walk::walked_target::<MINOR>
}

/// The independent row reference for an already validated fixture target.
pub(crate) fn oracle_for<const MINOR: u8>() -> Oracle {
    crate::outbound::syntax::adapters::ruff_python_parser::fixtures::oracle::oracle_target::<MINOR>
}
