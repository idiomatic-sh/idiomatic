//! Slot and generation state of one runtime scope, and the execution that owns the scopes.
use super::super::{
    ProgramExecution, ProjectState, RetirementWork, RootReaders, RuntimeState, Slot, State,
};
use crate::application::file_dag::program::admission::Partition;
#[cfg(test)]
use crate::application::file_dag::program::runtime::types::ReaderInitializations;
#[cfg(test)]
use crate::application::file_dag::program::runtime::types::CallCount;
use crate::application::file_dag::program::runtime::types::{Dependencies, ReaderCount, Touch};
use ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram;
use ryo_execution_domain::domain::executable_program::types::RegionId;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::cell::RefCell;
use std::sync::Arc;

impl Slot {
    pub(crate) fn touched<'a>(
        state: &RuntimeState,
        slots: &'a mut rustc_hash::FxHashMap<RegionId, Slot>,
        program: &ExecutableProgram,
        id: RegionId,
    ) -> Result<&'a mut Slot, OperatingError> {
        let region = program.regions().get(id.0).ok_or_else(|| OperatingError::NotRun)?;
        let vacant = match slots.entry(id) {
            std::collections::hash_map::Entry::Occupied(entry) => return Ok(entry.into_mut()),
            std::collections::hash_map::Entry::Vacant(entry) => entry,
        };
        let settled = state.dormant.borrow_mut().remove(&id);
        let remaining = match settled {
            Some(remaining) => remaining,
            None => state.initial_remaining(program, id)?,
        };
        let cancelled = remaining.0 == 0;
        // An absent root owes no input: the walk never counted its edges, so nothing is released when it publishes.
        let absent = state.absent.borrow().presence(id) == ryo_execution_domain::domain::executable_program::presence::types::RegionPresence::Absent;
        // Compiled obligations include roots in other project populations. They
        // remain a conservative upper bound until phase finish; no bank may
        // retire another population's publication or claim exact last use.
        Ok(vacant.insert(Self {
            #[cfg(test)]
            observed: Default::default(),
            #[cfg(test)]
            registered_calls: CallCount(0),
            state: if cancelled {
                State::Retired
            } else {
                State::Dormant
            },
            scratch: Vec::new(),
            admission: Default::default(),
            fed: vec![
                ryo_execution_domain::domain::operator::types::RowsFed::default();
                region.instructions().len()
            ]
            .into_boxed_slice(),
            remaining,
            dependencies_consumed: if cancelled || absent {
                Dependencies::Consumed
            } else {
                Dependencies::Pending
            },
            waiting_on: None,
            completion: None,
        }))
    }
}
impl RuntimeState {
    pub(crate) fn new(
        scope: ryo_execution_store::application::file_dag::relations::publication::types::ProducerScope,
        generation: Arc<ryo_execution_store::application::file_dag::relations::generation::service::GenerationLease>,
    ) -> Self {
        Self {
            generation,
            scope,
            roots: RefCell::new(RootReaders::Declared),
            #[cfg(test)]
            reader_initializations: std::cell::Cell::new(ReaderInitializations(0)),
            slots: RefCell::new(rustc_hash::FxHashMap::default()),
            dormant: RefCell::new(rustc_hash::FxHashMap::default()),
            retirement: RefCell::new(RetirementWork {
                simulated: rustc_hash::FxHashMap::default(),
                touched: Vec::new(),
                pending: Vec::new(),
                waiters: rustc_hash::FxHashMap::default(),
                retired_states: Vec::new(),
                retired_scratch: Vec::new(),
            }),
            absent: RefCell::new(Default::default()),
            interior: RefCell::new(Default::default()),
            refused: RefCell::new(Vec::new()),
        }
    }
    /// The refusal the file's presence read met for this source, which admission meets in its place.
    pub(crate) fn refusal(
        &self,
        relation: ryo_relations::domain::relations::types::Relation,
    ) -> Option<OperatingError> {
        self.refused
            .borrow()
            .iter()
            .find(|refusal| refusal.relation == relation)
            .map(|refusal| refusal.error.clone())
    }
    /// Whether a region the file's admitted roots reach has a skip formula that fails on the file.
    pub(crate) fn presence(
        &self,
        id: RegionId,
    ) -> ryo_execution_domain::domain::executable_program::presence::types::RegionPresence {
        self.interior.borrow().presence(id)
    }
    pub(crate) fn initial_remaining(
        &self,
        program: &ExecutableProgram,
        id: RegionId,
    ) -> Result<ReaderCount, OperatingError> {
        #[cfg(test)]
        self.reader_initializations.set(ReaderInitializations(
            self.reader_initializations.get().0 + 1,
        ));
        match &*self.roots.borrow() {
            RootReaders::Declared => program
                .regions()
                .get(id.0)
                .map(|region| ReaderCount(region.consumers().0))
                .ok_or_else(|| OperatingError::NotRun),
            RootReaders::Selected(selected) => {
                ryo_execution_domain::domain::executable_program::file_demand::service::reader_count(selected, id)
                    .map(|count| ReaderCount(count.0))
            }
        }
    }
    pub(crate) fn population_scope(
        &self,
    ) -> ryo_execution_store::application::file_dag::relations::publication::types::ProducerScope {
        self.scope
    }
    pub(crate) fn bind(
        &self,
        owner: &ryo_execution_store::application::file_dag::relations::RelationOwner,
        root: crate::application::file_dag::program::admission::ProducerRoot,
        publication: &ryo_execution_store::application::file_dag::relations::handle::service::PublishedInput,
    ) -> Result<(), OperatingError> {
        if let crate::application::file_dag::program::admission::ProducerRoot::Region(id) = root {
            owner.bind_producer(self.scope, id, publication.as_ref())?;
        }
        Ok(())
    }
}
/// Constructors are private to the outer execution owner; no primitive can open
/// another scope or choose a scheduler.
impl ProgramExecution {
    pub(crate) fn open(
        program: Arc<ExecutableProgram>,
        partition: Partition,
        owner: std::rc::Rc<ryo_execution_store::application::file_dag::relations::RelationOwner>,
    ) -> Self {
        let generation = owner.current_generation();
        let file_state = RuntimeState::new(
            ryo_execution_store::application::file_dag::relations::publication::types::ProducerScope::File,
            Arc::clone(&generation),
        );
        Self {
            program,
            owner,
            generation,
            partition,
            file_state,
            project_states: RefCell::new(Vec::new()),
            cleanup_failure: RefCell::new(None),
            readiness_counts: RefCell::new(Default::default()),
            #[cfg(test)]
            producer_lease_probe: RefCell::new(None),
        }
    }
    /// Bind the immutable declared file intents before any region is touched: the reader counts of one walk from the
    /// admitted roots that stops at the absent ones.
    pub(crate) fn activate_file_roots(
        &self,
        readers: &[RegionId],
        absent: ryo_execution_domain::domain::executable_program::file_demand::types::AbsentRoots,
    ) -> Result<(), OperatingError> {
        if !matches!(self.partition, Partition::File(_)) || self.touched() == Touch::Touched {
            return Err(OperatingError::NotRun);
        }
        let mut counted = rustc_hash::FxHashMap::<RegionId, usize>::default();
        for &root in readers {
            if !self.program.file_readers().roots.contains(&root) {
                return Err(OperatingError::NotRun);
            }
            *counted.entry(root).or_default() += 1;
        }
        let selected: Vec<_> = counted
            .into_iter()
            .map(|(region, count)| {
                ryo_execution_domain::domain::executable_program::file_demand::types::SelectedRoot {
                    region,
                    readers: ryo_execution_domain::domain::executable_program::types::ReaderMultiplicity(count),
                }
            })
            .collect();
        let table = ryo_execution_domain::domain::executable_program::file_demand::service::table(
            self.program.regions(),
            &selected,
            &absent,
        )?;
        *self.file_state.roots.borrow_mut() = RootReaders::Selected(Arc::new(table));
        *self.file_state.absent.borrow_mut() = absent;
        Ok(())
    }
    /// Whether the file scope has touched a region: before it has, its reader counts may be bound again.
    pub(crate) fn touched(&self) -> Touch {
        if self.file_state.slots.borrow().is_empty() && self.file_state.dormant.borrow().is_empty() {
            Touch::Untouched
        } else {
            Touch::Touched
        }
    }
    /// The regions the file's walk counted a reader for: what its admitted roots can reach, and its absent roots.
    pub(crate) fn reached(&self) -> Vec<RegionId> {
        match &*self.file_state.roots.borrow() {
            RootReaders::Selected(table) => {
                let mut reached = Vec::with_capacity(table.counts.len());
                reached.extend(
                    table
                        .counts
                        .iter()
                        .enumerate()
                        .filter(|(_, count)| count.0 > 0)
                        .map(|(at, _)| RegionId(at)),
                );
                reached
            }
            RootReaders::Declared => Vec::new(),
        }
    }
    /// The regions found absent at admission: interior regions of admitted rules whose skip formula fails on the file.
    pub(crate) fn mark_interior(
        &self,
        interior: ryo_execution_domain::domain::executable_program::file_demand::types::AbsentRegions,
    ) {
        *self.file_state.interior.borrow_mut() = interior;
    }
    /// The file's presence read met this source refused: its refusal is the file's refusal for the source.
    pub(crate) fn refuse_source(&self, refusal: crate::application::file_dag::program::runtime::types::SourceRefusal) {
        self.file_state.refused.borrow_mut().push(refusal);
    }
    pub(crate) fn validate_state(
        &self,
        state: &RuntimeState,
    ) -> Result<(), OperatingError> {
        if ryo_execution_store::application::file_dag::relations::generation::service::GenerationLease::same(&self.generation, &state.generation) == ryo_execution_store::application::file_dag::relations::types::Sameness::Same && self.generation.is_live() == ryo_execution_store::application::file_dag::relations::types::Liveness::Live {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(crate) fn file_state(&self) -> &RuntimeState {
        &self.file_state
    }
    pub(crate) fn project_state(
        &self,
        id: ryo_execution_store::application::file_dag::derived::population::types::PopulationId,
    ) -> std::rc::Rc<RuntimeState> {
        let mut states = self.project_states.borrow_mut();
        if let Some(project) = states.iter().find(|project| project.population == id) {
            return std::rc::Rc::clone(&project.state);
        }
        let state = std::rc::Rc::new(RuntimeState::new(
            ryo_execution_store::application::file_dag::relations::publication::types::ProducerScope::Project(id),
            Arc::clone(&self.generation),
        ));
        states.push(ProjectState {
            population: id,
            state: std::rc::Rc::clone(&state),
        });
        state
    }
    pub(crate) fn partition(&self) -> &Partition {
        &self.partition
    }
    /// Producer truth comes from the compiled declaration and this run's actual
    /// source scope. The consumer's requested partition is never an input.
    pub(crate) fn source_partition(
        &self,
        root: crate::application::file_dag::program::admission::ProducerRoot,
    ) -> Result<Partition, OperatingError> {
        use ryo_execution_domain::domain::executable_program::types::{InputSource, PartitionRequirement};
        if self.owner.matches_generation(&self.generation) == ryo_execution_store::application::file_dag::relations::types::Sameness::Different || self.generation.is_live() != ryo_execution_store::application::file_dag::relations::types::Liveness::Live {
            return Err(OperatingError::NotRead);
        }
        let source = match root {
            crate::application::file_dag::program::admission::ProducerRoot::Input(source) => source,
            crate::application::file_dag::program::admission::ProducerRoot::Region(id) => {
                InputSource::Region(id)
            }
        };
        let declaration = self.program.source_partition(source)?;
        match (declaration, &self.partition) {
            (PartitionRequirement::RequestInvariant, _) => Ok(Partition::RequestInvariant),
            (PartitionRequirement::CurrentFile, Partition::File(file)) => {
                Ok(Partition::File(file.clone()))
            }
            (PartitionRequirement::Project, Partition::Project(project)) => {
                Ok(Partition::Project(project.clone()))
            }
            (PartitionRequirement::InputLineage, actual) => Ok(actual.clone()),
            _ => Err(OperatingError::NotRead),
        }
    }
    pub(crate) fn validate_project(
        &self,
        program: &Arc<ExecutableProgram>,
        required: &crate::application::file_dag::program::admission::RequiredPartitions,
    ) -> Result<(), OperatingError> {
        if Arc::ptr_eq(&self.program, program)
            && matches!(&self.partition, Partition::Project(actual) if actual.same_request(required) == crate::application::file_dag::program::admission::Agreement::Same)
        {
            Ok(())
        } else {
            Err(OperatingError::WrongOperation)
        }
    }
    pub(crate) fn finish(self) -> Result<(), OperatingError> {
        if self
            .file_state
            .slots
            .borrow()
            .values()
            .any(|slot| matches!(slot.state, State::Running))
            || self.project_states.borrow().iter().any(|project| {
                project
                    .state
                    .slots
                    .borrow()
                    .values()
                    .any(|slot| matches!(slot.state, State::Running))
            })
        {
            return Err(OperatingError::NotRun);
        }
        let failure = self.cleanup_failure.into_inner();
        drop(self.file_state);
        drop(self.project_states);
        self.owner.release_population();
        failure.map_or(Ok(()), Err)
    }
}
