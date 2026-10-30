//! The types of the `check_store` port: plain data, public fields and derives, with no constructor and no inherent
//! impl.

use crate::domain::reports::types::{Recorded, Report};

/// The build that runs: a store another build wrote reads as empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Build(pub Box<str>);

/// Where a product's check store is kept and the build that keeps it, as the edge works them out: plain values,
/// nothing opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Store {
    /// The product's cache directory.
    pub directory: std::path::PathBuf,
    /// The build that runs.
    pub build: Build,
}

/// Whether the store holds a check's report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum KeptReport {
    /// None was kept.
    #[default]
    Nothing,
    /// This one.
    Kept(std::sync::Arc<Report>),
}

/// What the store keeps of a product's last check.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stored {
    /// The last check's report.
    pub report: KeptReport,
    /// The record of every file the last check stamped.
    pub recorded: Recorded,
}
