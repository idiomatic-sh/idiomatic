//! Immutable request-wide file root activation metadata.
use super::super::types::{InputSource, RegionId};
use crate::domain::executable_program::presence::types::RegionPresence;
use crate::domain::executable_program::types::RegionCount;
use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::{plan_parts::types::StepScope, records::types::ObservationIndex};

/// Whether a kind of root is demanded of a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootSelection {
    /// It is not demanded.
    Undemanded,
    /// It is demanded.
    Required,
}
/// Which frontiers are captured from a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontierSelection {
    /// None.
    Disabled,
    /// Those of the observations.
    Observations,
    /// Those of the diagnostics and the observations.
    All,
}
/// What a file is asked for: its diagnostic roots, its observation roots and the frontiers captured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileDemand {
    /// Whether the diagnostic roots are demanded.
    pub diagnostic: RootSelection,
    /// Whether the observation roots are demanded.
    pub observations: RootSelection,
    /// Which frontiers are captured.
    pub capture: FrontierSelection,
}
/// Where a demanded observation's rows come from, and its phase.
#[derive(Debug)]
pub struct ObservationDemandSource {
    /// The input the observation reads, or why it could not be resolved.
    pub source: Result<InputSource, OperatingError>,
    /// Whether the observation is per file or per project.
    pub phase: StepScope,
}
/// A root a file demands.
#[derive(Debug)]
pub enum FileRootDemand {
    /// An ordinary root, by its region.
    Ordinary(RegionId),
    /// A named observation output.
    Observation {
        /// The region producing the output.
        region: RegionId,
        /// The output, by its index.
        output: ObservationIndex,
    },
    /// Compilation retains the fault; only actual admission may surface it.
    Deferred(OperatingError),
}
/// What a file reads in one demand mode: its readers, its roots and its observations.
#[derive(Debug)]
pub struct FileMode {
    /// The regions that read the file.
    pub readers: Box<[RegionId]>,
    /// The roots demanded.
    pub roots: Box<[FileRootDemand]>,
    /// The observations demanded.
    pub observations: Box<[ObservationIndex]>,
}
/// How many named outputs a rule has.
#[derive(Debug, Clone, Copy)]
pub struct NamedOutputCount(pub usize);

/// A root a file selects as an external reader, and how many readers it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedRoot {
    /// The root.
    pub region: RegionId,
    /// Its readers.
    pub readers: crate::domain::operator::types::ReaderMultiplicity,
}
