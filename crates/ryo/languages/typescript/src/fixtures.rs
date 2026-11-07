//! What the language's conformance is held to besides the files of its fixtures: how each kind of its trees is shown and the typed rows
//! each fixture must give. Project fixtures hold npm dependencies, workspace membership, and file ownership.

pub mod contracts;
mod invalid_manifest;
pub(crate) mod kinds;
#[cfg(test)]
pub(crate) mod reading;
#[cfg(not(bless))]
mod relations;

use crate::outbound::syntax::fixtures::library;
use ryo::languages::contract::{Fixtures, Shape};

/// The fixtures of the language.
#[must_use]
pub fn expected() -> Fixtures {
    Fixtures {
        kinds: kinds::shown(),
        relations: stated_relations(),
        manifests: vec![invalid_manifest::expected(), workspace::expected()],
        semantic: Vec::new(),
        shape: Shape::Reference(library::walk()),
        oracle: library::oracle(),
        unfilled: stated_unfilled(),
    }
}

/// The rows each relation read from trees is held to.
#[cfg(not(bless))]
fn stated_relations() -> Vec<ryo::languages::contract::Expected> {
    relations::expected()
}

/// The rows each relation read from trees is held to: none while they are written (`--bless`).
#[cfg(bless)]
fn stated_relations() -> Vec<ryo::languages::contract::Expected> {
    Vec::new()
}

pub(crate) mod projects;
mod workspace;
#[cfg(test)]
pub(crate) mod workspaces;

/// The columns of the relations the language reads that its fixtures cannot show filled, each with why it never applies.
#[cfg(not(bless))]
fn stated_unfilled() -> Vec<ryo::languages::contract::Unfilled> {
    relations::unfilled()
}

/// The columns its fixtures cannot show filled: none while the rows are written (`--bless`), which holds no column.
#[cfg(bless)]
fn stated_unfilled() -> Vec<ryo::languages::contract::Unfilled> {
    Vec::new()
}
