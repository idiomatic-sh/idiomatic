//! The types of the `elements` concept: where an element of a file comes from, the expressions embedded in it and the macro expansions that produced it.

use crate::domain::relations::types::{Offset, Position};

/// The macro expansion that produced an element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExpansionId(pub u32);

/// Where an element comes from: the file itself, or the expansion of a macro call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ElementOrigin {
    /// The file wrote it.
    #[default]
    File,
    /// The parser implied this node; no source location was reported.
    Implied,
    /// A separately parsed expression carried by a source element, not a macro expansion.
    Embedded {
        /// The native element containing the expression's logical source.
        host: Position,
    },
    /// A macro expansion produced it: it has the span of its call site, and the identity of the call site beside the expansion.
    Expansion {
        /// The element of the file that is the call.
        call_site: Position,
        /// The expansion.
        expansion: ExpansionId,
    },
}

/// How logical offsets in an embedded expression map to the containing source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmbeddedMapping {
    /// Logical offsets are shifted by this physical content start.
    Exact(Offset),
    /// Decoding or concatenation prevents precise subexpression locations.
    WholeHost,
}

/// One contiguous, separately parsed expression in a file's element table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmbeddedSource {
    /// The native string or other carrier of the logical source.
    pub host: Position,
    /// The first appended element, normally the expression root.
    pub root: Position,
    /// The exclusive end of the appended elements.
    pub end: Position,
    /// The physical source mapping; token text remains logical text.
    pub mapping: EmbeddedMapping,
}

/// An element that a macro expansion produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Expanded {
    /// The element.
    pub element: Position,
    /// The element of the file that is the call.
    pub call_site: Position,
    /// The expansion.
    pub expansion: ExpansionId,
}
