//! The types of the `owner_facts` concept: the scope every node of one file is evaluated in, worked out once for the fillers that
//! name an owner.

use ryo::languages::Position;

/// Where a part of a node is evaluated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Evaluated {
    /// In the scope the node opens.
    Inside,
    /// In the scope around the node.
    Around,
}

/// Whether the owners of the file being read are worked out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Readiness {
    /// Not yet: the first filler that asks works them out.
    #[default]
    Pending,
    /// Worked out for this file.
    Ready,
}

/// The scope each syntax node of one file is evaluated in, by the node's id; kept from file to file and refilled.
#[derive(Clone, Debug, Default)]
pub(crate) struct Owners {
    /// The owner of each node: the module for the module itself.
    pub scopes: Vec<Position>,
    /// Whether `scopes` is this file's.
    pub ready: Readiness,
}
