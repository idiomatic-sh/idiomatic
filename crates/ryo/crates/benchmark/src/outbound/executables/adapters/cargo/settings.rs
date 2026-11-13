//! What the cargo artifacts adapter is built with.
use std::ffi::OsString;

/// The build tool and the reference tool's search path, as the edge read them.
#[derive(Clone, Debug)]
pub(crate) struct ExecutablesCargoSettings {
    /// The cargo executable the edge was started by.
    pub(crate) cargo: OsString,
    /// The search path of the reference tool.
    pub(crate) search_path: OsString,
}
