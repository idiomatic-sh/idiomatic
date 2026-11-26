//! The typed fixtures of the language: what its conformance holds its answers to besides the files of its fixtures: how each kind of
//! its trees is shown and the typed rows each fixture must give. It reads the manifests of a Python project: the rows they give are the fixture's.

pub mod contracts;
pub mod grown;
pub(crate) mod kinds;
#[cfg(test)]
pub(crate) mod projects;
#[cfg(test)]
pub(crate) mod syntax_files;
pub mod table_laws;
pub mod walking;
