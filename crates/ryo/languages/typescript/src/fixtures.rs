//! What the language's conformance is held to besides the files of its fixtures: how each kind of its trees is shown and the typed rows
//! each fixture must give. Project fixtures hold npm dependencies, workspace membership, and file ownership.

pub(crate) mod projects;
#[cfg(test)]
pub(crate) mod workspaces;
