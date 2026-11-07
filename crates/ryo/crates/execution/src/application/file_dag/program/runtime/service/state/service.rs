//! Slot and generation state of one runtime scope, and the execution that owns the scopes.
use super::super::{
    ProgramExecution, ProjectState, RetirementWork, RootReaders, RuntimeState, Slot, State,
};
use crate::application::file_dag::program::admission::Partition;
#[cfg(test)]
use crate::application::file_dag::program::runtime::types::{CallCount, ReaderInitializations};
use crate::application::file_dag::program::runtime::types::{Dependencies, ReaderCount};
use crate::domain::executable_program::types::ExecutableProgram;
use crate::domain::executable_program::types::RegionId;
use crate::domain::operating::errors::OperatingError;
use std::cell::RefCell;
use std::sync::Arc;

impl Slot {
    pub(in crate::application::file_dag::program::runtime) fn touched<'a>(
        state: &RuntimeState,
        slots: &'a mut std::collections::HashMap<RegionId, Slot>,
        program: &ExecutableProgram,
        id: RegionId,
    ) -> Result<&'a mut Slot, OperatingError> {
        let region = program.regions().get(id.0).ok_or(OperatingError::NotRun)?;
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
        // Compiled obligations include roots in other project populations. They
        // remain a conservative upper bound until phase finish; no bank may
        // retire another population's publication or claim exact last use.
        Ok(vacant.insert(Self {
            #[cfg(test)]
            observed: Default::default(),
            #[cfg(test)]
            native_positions: CallCount(0),
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
                crate::domain::operator::types::RowsFed::default();
                region.instructions.len()
            ]
            .into_boxed_slice(),
            remaining,
            dependencies_consumed: if cancelled {
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
    pub(in crate::application::file_dag::program::runtime::service) fn new(
        scope: crate::application::file_dag::relations::publication::types::ProducerScope,
        generation: Arc<()>,
    ) -> Self {
        Self {
            generation,
            scope,
            roots: RefCell::new(RootReaders::Declared),
            #[cfg(test)]
            reader_initializations: std::cell::Cell::new(ReaderInitializations(0)),
            slots: RefCell::new(std::collections::HashMap::new()),
            dormant: RefCell::new(std::collections::HashMap::new()),
            retirement: RefCell::new(RetirementWork {
                simulated: std::collections::HashMap::new(),
                touched: Vec::new(),
                pending: Vec::new(),
                waiters: std::collections::HashMap::new(),
                retired_states: Vec::new(),
                retired_scratch: Vec::new(),
            }),
        }
    }
    pub(in crate::application::file_dag::program::runtime::service) fn initial_remaining(
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
                .map(|region| ReaderCount(region.consumers))
                .ok_or(OperatingError::NotRun),
            RootReaders::Selected(selected) => {
                Ok(ReaderCount(selected.of(id).0))
            }
        }
    }
    pub(in crate::application::file_dag::program) fn population_scope(
        &self,
    ) -> crate::application::file_dag::relations::publication::types::ProducerScope {
        self.scope
    }
    pub(in crate::application::file_dag::program) fn bind(
        &self,
        owner: &crate::application::file_dag::relations::RelationOwner,
        root: crate::application::file_dag::program::admission::ProducerRoot,
        publication: &crate::application::file_dag::relations::handle::service::PublishedInput,
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
    pub(in crate::application::file_dag::program) fn open(
        program: Arc<ExecutableProgram>,
        partition: Partition,
        owner: std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
    ) -> Self {
        let generation = owner.generation_guard();
        let generation_live = owner.generation_liveness();
        let file_state = RuntimeState::new(
            crate::application::file_dag::relations::publication::types::ProducerScope::File,
            Arc::clone(&generation),
        );
        Self {
            program,
            owner,
            generation,
            generation_live,
            partition,
            file_state,
            project_states: RefCell::new(Vec::new()),
            cleanup_failure: RefCell::new(None),
            readiness_counts: RefCell::new(Default::default()),
            #[cfg(test)]
            producer_lease_probe: RefCell::new(None),
        }
    }
    /// Bind the immutable declared file intents before any region is touched.
    pub(in crate::application::file_dag::program) fn activate_file_roots(
        &self,
        readers: &[RegionId],
    ) -> Result<(), OperatingError> {
        if !matches!(self.partition, Partition::File(_))
            || !self.file_state.slots.borrow().is_empty()
            || !self.file_state.dormant.borrow().is_empty()
            || !matches!(*self.file_state.roots.borrow(), RootReaders::Declared)
        {
            return Err(OperatingError::NotRun);
        }
        let mut selected = std::collections::HashMap::new();
        for &root in readers {
            if !self.program.file_readers().consumers.contains_key(&root) {
                return Err(OperatingError::NotRun);
            }
            selected
                .entry(root)
                .or_insert(crate::domain::executable_program::types::ReaderMultiplicity(0))
                .0 += 1;
        }
        let remaining = crate::domain::executable_program::file_demand::service::remaining(
            self.program.file_readers(),
            crate::domain::readiness::types::RegionCount(self.program.regions().len()),
            &selected,
        )?;
        *self.file_state.roots.borrow_mut() = RootReaders::Selected(remaining.table);
        Ok(())
    }
    pub(in crate::application::file_dag::program::runtime::service) fn validate_state(
        &self,
        state: &RuntimeState,
    ) -> Result<(), OperatingError> {
        if Arc::ptr_eq(&self.generation, &state.generation) && self.generation_live.is_live() {
            Ok(())
        } else {
            Err(OperatingError::NotRead)
        }
    }
    pub(in crate::application::file_dag::program) fn file_state(&self) -> &RuntimeState {
        &self.file_state
    }
    pub(in crate::application::file_dag::program) fn project_state(
        &self,
        id: crate::application::file_dag::derived::population::types::PopulationId,
    ) -> std::rc::Rc<RuntimeState> {
        let mut states = self.project_states.borrow_mut();
        if let Some(project) = states.iter().find(|project| project.population == id) {
            return std::rc::Rc::clone(&project.state);
        }
        let state = std::rc::Rc::new(RuntimeState::new(
            crate::application::file_dag::relations::publication::types::ProducerScope::Project(id),
            Arc::clone(&self.generation),
        ));
        states.push(ProjectState {
            population: id,
            state: std::rc::Rc::clone(&state),
        });
        state
    }
    pub(in crate::application::file_dag::program) fn partition(&self) -> &Partition {
        &self.partition
    }
    /// Producer truth comes from the compiled declaration and this run's actual
    /// source scope. The consumer's requested partition is never an input.
    pub(in crate::application::file_dag::program) fn source_partition(
        &self,
        root: crate::application::file_dag::program::admission::ProducerRoot,
    ) -> Result<Partition, OperatingError> {
        use crate::domain::executable_program::types::{InputSource, PartitionRequirement};
        if !self.owner.matches_generation(&self.generation) || !self.generation_live.is_live() {
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
    pub(in crate::application::file_dag::program) fn validate_project(
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
    pub(in crate::application::file_dag::program) fn finish(self) -> Result<(), OperatingError> {
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
