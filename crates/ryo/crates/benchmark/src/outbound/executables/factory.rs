//! Builds the executables adapters, one function each, by constructor injection: a factory takes
//! the settings the edge read and returns the adapter built with them. It is called only by an edge.
use super::adapters::cargo::adapter::ExecutablesCargoAdapter;
use super::adapters::cargo::settings::ExecutablesCargoSettings;
use std::ffi::OsString;

/// The cargo implementation: `cargo` is the build tool the edge was started by, `search_path` the search path of the reference tool.
#[must_use]
pub fn cargo(cargo: OsString, search_path: OsString) -> impl super::interface::ExecutablesPort {
    ExecutablesCargoAdapter::new(ExecutablesCargoSettings { cargo, search_path })
}
