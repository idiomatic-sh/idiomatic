//! Immutable request-wide file root activation metadata.
use super::super::types::{InputSource, RegionId};
use crate::domain::executable_program::presence::types::RegionPresence;
use crate::domain::executable_program::types::RegionCount;
use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::{
    plan_parts::types::StepScope,
    records::types::ObservationIndex,
};

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
impl FileDemand {
    /// Nothing is demanded.
    pub const NONE: Self = Self {
        diagnostic: RootSelection::Undemanded,
        observations: RootSelection::Undemanded,
        capture: FrontierSelection::Disabled,
    };
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
/// A rule's file demand in every demand mode.
#[derive(Debug)]
pub struct FileRuleDemand {
    /// How many named outputs the rule has.
    pub outputs: NamedOutputCount,
    pub(super) modes: Box<[FileMode]>,
}
impl FileRuleDemand {
    /// The file mode the demand selects.
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

/// A root a file selects as an external reader, and how many readers it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedRoot {
    /// The root.
    pub region: RegionId,
    /// Its readers.
    pub readers: crate::domain::executable_program::types::ReaderMultiplicity,
}
/// The file roots whose skip formula fails on one file: each publishes the empty table there without demanding an input,
/// and the file's reader counts never follow its edges. Decided once per file, when the file binds its intents.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbsentRoots(AbsentRegions);

impl AbsentRoots {
    /// The roots a file found absent.
    pub fn decided(absent: AbsentRegions) -> Self {
        Self(absent)
    }
    /// Whether a region is one of them.
    pub fn presence(&self, region: RegionId) -> RegionPresence {
        self.0.presence(region)
    }
    /// How many there are.
    pub fn count(&self) -> RegionCount {
        self.0.count()
    }
}

/// Regions whose skip formula fails on one file, one bit each.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbsentRegions(Box<[u64]>);

impl AbsentRegions {
    /// No region, over a program of `regions` regions.
    pub fn none(regions: RegionCount) -> Self {
        Self(vec![0; regions.0.div_ceil(64)].into_boxed_slice())
    }
    /// Adds a region; one past the program is ignored.
    pub fn insert(&mut self, region: RegionId) {
        if let Some(word) = self.0.get_mut(region.0 / 64) {
            *word |= 1 << (region.0 % 64);
        }
    }
    /// Whether the set holds a region.
    pub fn presence(&self, region: RegionId) -> RegionPresence {
        if self
            .0
            .get(region.0 / 64)
            .is_some_and(|word| word & (1 << (region.0 % 64)) != 0)
        {
            RegionPresence::Absent
        } else {
            RegionPresence::Present
        }
    }
    /// How many regions it holds.
    pub fn count(&self) -> RegionCount {
        RegionCount(self.0.iter().map(|word| word.count_ones() as usize).sum())
    }
}
