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
    walk_target::<MINOR>
}

/// Walk the reference with precisely the parser's selected target.
fn walk_target<const MINOR: u8>(text: &str) -> ryo::languages::contract::Walked {
    crate::outbound::syntax::adapters::ruff_python_parser::fixtures::walk::walked_with(
        text,
        ruff_python_ast::PythonVersion {
            major: 3,
            minor: MINOR,
        },
    )
}

/// The independent row reference for an already validated fixture target.
pub(crate) fn oracle_for<const MINOR: u8>() -> Oracle {
    oracle_target::<MINOR>
}

/// Read independent row expectations with precisely the parser's selected target.
fn oracle_target<const MINOR: u8>(
    text: &str,
    rows: &ryo::languages::contract::OracleRows,
) -> Vec<String> {
    crate::outbound::syntax::adapters::ruff_python_parser::fixtures::oracle::oracle_with(
        text,
        rows,
        ruff_python_ast::PythonVersion {
            major: 3,
            minor: MINOR,
        },
    )
}
