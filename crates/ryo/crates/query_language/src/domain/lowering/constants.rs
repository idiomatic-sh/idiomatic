//! The names of the `lowering` concept: the headings a plan's own columns carry, text that leaves the engine.

/// A column the plan itself adds or expects of a hierarchy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HierarchyColumn {
    /// The entity column of a table of nodes.
    Id,
    /// The parent column of an edge table.
    Parent,
    /// The child column of an edge table.
    Child,
    /// The column an axis appends.
    Related,
}

/// The text of a heading.
#[must_use]
pub(crate) fn heading(heading: HierarchyColumn) -> &'static str {
    match heading {
        HierarchyColumn::Id => "id",
        HierarchyColumn::Parent => ryo_language::domain::relations::constants::PARENT,
        HierarchyColumn::Child => "child",
        HierarchyColumn::Related => "related",
    }
}

/// A column the same-sets idiom adds to the pairs of functions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Added {
    /// The digest of one function's texts.
    Digest,
    /// How many texts the digest covers.
    Size,
    /// The digest of the other function's texts.
    OtherDigest,
    /// How many texts the other digest covers.
    OtherSize,
}

/// The text of an added column.
#[must_use]
pub(crate) fn added(added: Added) -> &'static str {
    match added {
        Added::Digest => "digest",
        Added::Size => "size",
        Added::OtherDigest => "other-digest",
        Added::OtherSize => "other-size",
    }
}
