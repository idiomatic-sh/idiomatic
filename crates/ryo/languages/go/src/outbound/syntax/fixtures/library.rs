//! The library of the `tree_sitter_go` adapter, as the conformance reads it: the walk of its own tree, its own reading of the file, and
//! the roles it gives the elements.

use ryo::languages::contract::{LibraryWalk, Oracle};

/// How the library walks its own tree of a file: what the conformance holds our tree to the shape of.
#[must_use]
pub(crate) fn walk() -> LibraryWalk {
    crate::outbound::syntax::adapters::tree_sitter_go::fixtures::walk::walked
}

/// How the library reads the document of a file: what the conformance holds the rows to.
#[must_use]
pub(crate) fn oracle() -> Oracle {
    crate::outbound::syntax::adapters::tree_sitter_go::fixtures::oracle::oracle
}

/// The roles `go/ast` gives the elements of a file, each `start end N|T kind field`: what the roles of our tree are held to; none for
/// a file `go/parser` refuses or the fixtures do not pin.
#[must_use]
pub(crate) fn roles(text: &str) -> Option<Vec<&'static str>> {
    crate::outbound::syntax::adapters::tree_sitter_go::fixtures::walk::roles(text)
}

/// The files whose tree and roles the standard pinned: every relation's input, every file of the corpus and of the suite.
#[must_use]
pub(crate) fn pinned(fixtures: &std::path::Path) -> Vec<std::path::PathBuf> {
    crate::outbound::syntax::adapters::tree_sitter_go::fixtures::walk::pinned(fixtures)
}

/// The library's tree of `text` as plain data in the words of Go: what the services of the domain are run on; none when the library
/// gives no tree.
#[must_use]
pub(crate) fn concreted(text: &str) -> Option<crate::domain::grammar::types::Concrete<'_>> {
    crate::outbound::syntax::adapters::tree_sitter_go::fixtures::concreted(
        crate::domain::grammar::types::GoText(text),
    )
}
