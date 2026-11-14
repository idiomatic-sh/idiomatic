//! The sole immutable executable form: fields private, made only by the executable-program compiler, read through accessors.
//! There is no retained RequestGraph, PhysicalRegion, PlanLayout or per-plan evaluator here.
use super::super::region::value::Region;
use super::super::{
    FileReaderGraph, InputSource, LogicalInstruction, PartitionRequirement, RegionId, RuleProgram,
};
use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::{plan_parts::types::StepScope, plans::types::Operation};

/// The compiled program of one request.
#[derive(Debug)]
pub struct ExecutableProgram {
    regions: Box<[Region]>,
    file_readers: FileReaderGraph,
    rules: Box<[RuleProgram]>,
    logical_regions: Box<[RegionId]>,
    observation_regions: Box<[Result<RegionId, OperatingError>]>,
    /// The partition requirement of each relation source: the first region declaring it decides, found once per program.
    relation_partitions: rustc_hash::FxHashMap<
        ryo_relations::domain::relations::types::Relation,
        PartitionRequirement,
    >,
    /// What a file must hold for each region to have a row or a row error there.
    presence: crate::domain::executable_program::presence::types::PresenceTable,
    /// The request's literal pattern slots, declared from the program's operations; the presence fold compiled its source
    /// searches' patterns into them.
    matchers: std::sync::Arc<ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers>,
    /// The fusion classification of every instruction, indexed by its logical number.
    #[cfg(any(test, feature = "fixtures"))]
    fusibility: Box<[crate::domain::executable_program::fusion::types::Fusibility]>,
}

impl ExecutableProgram {
    /// The program the compiler made.
    pub(in crate::domain::executable_program) fn new(
        regions: Box<[Region]>,
        file_readers: FileReaderGraph,
        rules: Box<[RuleProgram]>,
        logical_regions: Box<[RegionId]>,
        observation_regions: Box<[Result<RegionId, OperatingError>]>,
    ) -> Self {
        let mut relation_partitions = rustc_hash::FxHashMap::default();
        for declaration in regions
            .iter()
            .flat_map(|region| region.source_partitions().iter())
        {
            if let InputSource::Relation(relation) = declaration.source {
                let _first = relation_partitions
                    .entry(relation)
                    .or_insert(declaration.requirement);
            }
        }
        let region_count = regions.len();
        Self {
            regions,
            relation_partitions,
            file_readers,
            rules,
            logical_regions,
            observation_regions,
            presence: crate::domain::executable_program::presence::types::PresenceTable {
                regions: vec![
                    crate::domain::executable_program::presence::service::ALWAYS;
                    region_count
                ]
                .into_boxed_slice(),
                reads: vec![Box::default(); region_count].into_boxed_slice(),
                ..Default::default()
            },
            matchers: std::sync::Arc::default(),
            #[cfg(any(test, feature = "fixtures"))]
            fusibility: Box::new([]),
        }
    }
    /// The same program with the presence formulas of its regions.
    pub(in crate::domain::executable_program) fn with_presence(
        mut self,
        presence: crate::domain::executable_program::presence::types::PresenceTable,
    ) -> Self {
        self.presence = presence;
        self
    }
    /// The same program with the literal pattern slots its presence fold compiled into.
    pub(in crate::domain::executable_program) fn with_matchers(
        mut self,
        matchers: std::sync::Arc<ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers>,
    ) -> Self {
        self.matchers = matchers;
        self
    }
    /// The request's literal pattern slots: one per authored pattern of the program, each compiled at most once per run.
    ///
    /// # Examples
    ///
    /// ```
    /// // A program is built only by compiling a request, and this crate has no fixture path to one: the call's types, checked.
    /// use ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram;
    /// use ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers;
    /// use std::sync::Arc;
    /// fn slots(program: &ExecutableProgram) -> &Arc<PreparedMatchers> {
    ///     program.matchers()
    /// }
    /// ```
    pub fn matchers(
        &self,
    ) -> &std::sync::Arc<ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers> {
        &self.matchers
    }
    /// What a file must hold for each region to have a row or a row error there.
    pub fn presence(&self) -> &crate::domain::executable_program::presence::types::PresenceTable {
        &self.presence
    }
    /// The skip formula of a region; a program made without a presence table holds every region `Always`.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] for a region the program does not have.
    pub fn presence_of(
        &self,
        region: RegionId,
    ) -> Result<crate::domain::executable_program::presence::types::FormulaId, OperatingError> {
        self.presence
            .regions
            .get(region.0)
            .copied()
            .ok_or(OperatingError::NotRun)
    }
    /// The same program with the fusion classification the compiler made, which only the controls read.
    #[cfg(any(test, feature = "fixtures"))]
    pub(in crate::domain::executable_program) fn with_fusibility(
        mut self,
        fusibility: Box<[crate::domain::executable_program::fusion::types::Fusibility]>,
    ) -> Self {
        self.fusibility = fusibility;
        self
    }
    /// How the compiler classified an instruction against the region of its producer.
    #[cfg(any(test, feature = "fixtures"))]
    pub fn fusibility(
        &self,
        logical: LogicalInstruction,
    ) -> Option<crate::domain::executable_program::fusion::types::Fusibility> {
        self.fusibility.get(logical.0).copied()
    }
    /// The readers of each file root.
    pub fn file_readers(&self) -> &FileReaderGraph {
        &self.file_readers
    }
    /// The prepared physical phase of an original frontier root. No file or
    /// project runtime can change this declaration while admitting its input.
    pub fn frontier_phase(&self, source: InputSource) -> Result<StepScope, OperatingError> {
        match source {
            InputSource::Relation(_) => match self.source_partition(source)? {
                PartitionRequirement::CurrentFile => Ok(StepScope::PerFile),
                PartitionRequirement::Project => Ok(StepScope::CrossFile),
                _ => Err(OperatingError::WrongOperation),
            },
            InputSource::Region(id) => self
                .regions
                .get(id.0)
                .map(Region::phase)
                .ok_or(OperatingError::NotRun),
            InputSource::Observation(slot) => {
                let root = self
                    .observation_regions
                    .get(slot.0)
                    .ok_or(OperatingError::NotRun)?
                    .as_ref()
                    .map_err(Clone::clone)?;
                self.regions
                    .get(root.0)
                    .map(Region::phase)
                    .ok_or(OperatingError::NotRun)
            }
            _ => Err(OperatingError::WrongOperation),
        }
    }
    /// The partition requirement of one input of a region.
    pub fn source_partition(
        &self,
        source: InputSource,
    ) -> Result<PartitionRequirement, OperatingError> {
        let last = |region: &Region| {
            region
                .instructions()
                .last()
                .map(super::super::instruction::value::Instruction::partition)
        };
        match source {
            InputSource::Relation(relation) => self
                .relation_partitions
                .get(&relation)
                .copied()
                .ok_or(OperatingError::NotRead),
            InputSource::Region(id) => self
                .regions
                .get(id.0)
                .and_then(last)
                .ok_or(OperatingError::NotRead),
            InputSource::Hosted { rule, relation } => self
                .regions
                .iter()
                .flat_map(|region| region.instructions().iter())
                .find(|instruction| {
                    instruction.producer() == rule
                        && matches!(instruction.operation(), Operation::Hosted { relation: actual } if *actual == relation)
                })
                .map(super::super::instruction::value::Instruction::partition)
                .ok_or(OperatingError::NotRead),
            InputSource::EmptyObservation(slot) => {
                let root = self
                    .observation_regions
                    .get(slot.0)
                    .ok_or(OperatingError::NotRead)?
                    .as_ref()
                    .map_err(Clone::clone)?;
                self.regions
                    .get(root.0)
                    .and_then(last)
                    .ok_or(OperatingError::NotRead)
            }
            _ => Err(OperatingError::NotRead),
        }
    }
    /// The program's regions.
    pub fn regions(&self) -> &[Region] {
        &self.regions
    }
    /// The program of each rule.
    pub fn rules(&self) -> &[RuleProgram] {
        &self.rules
    }
    /// The region of each logical instruction.
    pub fn logical_regions(&self) -> &[RegionId] {
        &self.logical_regions
    }
    /// The region of each observation, or why it has none.
    pub fn observation_regions(&self) -> &[Result<RegionId, OperatingError>] {
        &self.observation_regions
    }
    /// Where the rows of one input of a region come from: a relation's from one file each, a region's or a named
    /// observation's as the compiler found, anything else from no one file.
    pub fn source_lineage(
        &self,
        source: InputSource,
    ) -> ryo_values::domain::lineage::types::Lineage {
        use ryo_values::domain::lineage::types::Lineage;
        let of_region = |id: RegionId| {
            self.regions
                .get(id.0)
                .map(|region| region.lineage().clone())
                .unwrap_or_default()
        };
        match source {
            InputSource::Relation(relation) => ryo_values::domain::lineage::service::one_file(
                ryo_relations::domain::relations::service::kinds(relation),
            ),
            InputSource::Region(id) => of_region(id),
            InputSource::Observation(place) => match self.observation_regions.get(place.0) {
                Some(Ok(root)) => of_region(*root),
                Some(Err(_)) | None => Lineage::Mixed,
            },
            InputSource::Hosted { .. }
            | InputSource::EmptyObservation(_)
            | InputSource::ObservationMember { .. } => Lineage::Mixed,
        }
    }
}
