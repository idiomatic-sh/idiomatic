//! The types of the `visit` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. What a visit wants of a file is the `language` port's `WantedRelation`.

/// The bytes of a file, borrowed.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BorrowedBytes<'a>(pub &'a [u8]);

/// How many lines a file's bytes hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LineCount(pub i64);

/// Whether a step reads the `field` column of the edges of the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RoleRead {
    /// The step reads the role, so the language is asked for the roles of the file.
    Reads,
    /// The step reads no role.
    Skips,
}

/// Whether the columns a step keeps name the role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RoleNamed {
    /// A kept column is the role.
    Named,
    /// No kept column is the role.
    Unnamed,
}

/// Whether the comparisons of a step read the role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RoleCompared {
    /// A comparison reads the role.
    Compared,
    /// No comparison reads the role.
    Uncompared,
}

/// Whether an input of a step is the edges of the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EdgeInput {
    /// The input is the edges of the tree.
    Edges,
    /// The input is another relation or step.
    Elsewhere,
}

/// Whether the run's store keeps anything: a run whose store keeps nothing makes nothing only a store reads (the digests of the files,
/// the words they hand moved for another run, the digests of what they show one another).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Keeping {
    /// It keeps what the run found for the next.
    Keeps,
    /// It keeps nothing.
    Discards,
}

/// Whether the languages write their notes of the files, for the project to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Noting {
    /// They do.
    Wanted,
    /// They do not.
    Unwanted,
}

/// What the plans of a run read of the files: the relations the languages are asked for, and what else they are asked to write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Wants {
    /// The relations the plans read.
    pub wanted: Vec<ryo_relations::domain::relations::types::Relation>,
    /// The relations the languages are asked to walk the files for.
    pub walked: Vec<ryo_relations::domain::relations::types::Relation>,
    /// Whether the languages write their notes of the files.
    pub noted: Noting,
    /// Whether the languages give the elements their roles.
    pub roled: RoleRead,
}
