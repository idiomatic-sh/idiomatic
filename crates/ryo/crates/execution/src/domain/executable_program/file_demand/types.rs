//! Immutable request-wide file root activation metadata.
use super::super::types::{InputSource, RegionId};
use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::{
    plan_parts::types::StepScope,
    records::types::ObservationIndex,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RootSelection {
    Undemanded,
    Required,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FrontierSelection {
    Disabled,
    Observations,
    All,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileDemand {
    pub diagnostic: RootSelection,
    pub observations: RootSelection,
    pub capture: FrontierSelection,
}
impl FileDemand {
    pub const NONE: Self = Self {
        diagnostic: RootSelection::Undemanded,
        observations: RootSelection::Undemanded,
        capture: FrontierSelection::Disabled,
    };
}
pub(crate) struct ObservationDemandSource {
    pub source: Result<InputSource, OperatingError>,
    pub phase: StepScope,
}
#[derive(Debug)]
pub(crate) enum FileRootDemand {
    Ordinary(RegionId),
    Observation {
        region: RegionId,
        output: ObservationIndex,
    },
    /// Compilation retains the fault; only actual admission may surface it.
    Deferred(OperatingError),
}
#[derive(Debug)]
pub(crate) struct FileMode {
    pub readers: Box<[RegionId]>,
    pub roots: Box<[FileRootDemand]>,
    pub observations: Box<[ObservationIndex]>,
}
#[derive(Debug)]
pub(crate) struct NamedOutputCount(pub usize);
#[derive(Debug)]
pub(crate) struct FileRuleDemand {
    pub outputs: NamedOutputCount,
    pub(super) modes: Box<[FileMode]>,
}
impl FileRuleDemand {
    pub fn selected(&self, demand: FileDemand) -> &FileMode {
        let diagnostic = match demand.diagnostic {
            RootSelection::Undemanded => 0,
            RootSelection::Required => 6,
        };
        let observations = match demand.observations {
            RootSelection::Undemanded => 0,
            RootSelection::Required => 3,
        };
        let capture = match demand.capture {
            FrontierSelection::Disabled => 0,
            FrontierSelection::Observations => 1,
            FrontierSelection::All => 2,
        };
        &self.modes[diagnostic + observations + capture]
    }
}
