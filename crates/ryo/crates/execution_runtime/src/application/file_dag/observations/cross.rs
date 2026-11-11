//! Named publication leases across declared project ready batches. No source IO or producer evaluation.
use super::constants::PUBLICATION_INCOMPLETE;
use super::types::{
    ArenaCount, ConsumerCount, CrossPublications, CrossRoot, FinishState, ReferenceCount,
};
use crate::application::file_dag::frontiers::types::{
    BoundInput, BoundMember, InputReadiness, PublicationSlot,
};
use crate::application::file_dag::program::observation_stage::types::AdmittedOutput;
use crate::application::file_dag::program::preparation::types::ObservationMetadata;
use crate::application::file_dag::program::preparation::types::OutputRoot;
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use ryo_relations::domain::relations::types::{CROSS_FILE, WorkerNumber};
use ryo_values::domain::{
    records::types::{ObservationIndex, ProducerReadiness},
    stored_facts::types::ArenaBytes,
};
use std::rc::Rc;
use {
    ryo_execution_domain::domain::coverage::types::RuleIndex,
    ryo_execution_domain::domain::file_program::types::ObservationRetirement,
    ryo_execution_domain::domain::operating::errors::OperatingError,
    ryo_values::domain::records::types::CrossPublicationTransfer,
};

impl<'a> CrossPublications<'a> {
    pub(crate) fn new(
        metadata: &'a ObservationMetadata,
        scope: &crate::application::file_dag::program::facade_runtime::service::PublicationScope,
        arenas: &'a mut Vec<ArenaBytes>,
    ) -> Self {
        let inherited = arenas.len();
        Self {
            metadata,
            owner: Rc::clone(scope.owner()),
            inherited: ArenaCount(inherited),
            references: vec![ReferenceCount(0); inherited],
            arenas,
            roots: metadata
                .outputs
                .iter()
                .map(|outputs| {
                    outputs
                        .iter()
                        .map(|output| match output.root {
                            OutputRoot::CrossFile => CrossRoot::Pending,
                            OutputRoot::PerFile | OutputRoot::Unavailable => CrossRoot::Unavailable,
                        })
                        .collect()
                })
                .collect(),
            remaining: metadata
                .outputs
                .iter()
                .map(|outputs| {
                    outputs
                        .iter()
                        .map(|output| ConsumerCount(output.consumers.len()))
                        .collect()
                })
                .collect(),
            finished: vec![FinishState::Pending; metadata.bindings.len()],
        }
    }
    pub(crate) fn arenas(&self) -> &[ArenaBytes] {
        self.arenas
    }
    /// Prepared once for one task. Availability retains the producer's admitted
    /// coverage; a partial publication is usable without becoming complete-empty.
    pub(crate) fn binding_readiness(
        &self,
        rule: RuleIndex,
    ) -> Result<
        std::collections::BTreeMap<
            ryo_values::domain::plan_parts::types::ObservationBindingId,
            InputReadiness,
        >,
        OperatingError,
    > {
        let bindings = self.metadata.bindings.get(rule.0).ok_or_else(incomplete)?;
        Ok(bindings
            .iter()
            .map(|binding| {
                let available = binding.producers.iter().all(|producer| match producer {
                    ryo_execution_domain::domain::observation_schedule::types::ResolvedObservationProducer::NoQuery { .. } => false,
                    ryo_execution_domain::domain::observation_schedule::types::ResolvedObservationProducer::Output { rule, output } => matches!(self.roots.get(rule.0).and_then(|roots| roots.get(output.0 as usize)), Some(CrossRoot::FileFrontier { .. } | CrossRoot::Named { .. })),
                });
                (
                    binding.input,
                    if available {
                        InputReadiness::Ready
                    } else {
                        InputReadiness::Unavailable
                    },
                )
            })
            .collect())
    }
    /// Only phases with demanded cross roots reserve an ordinary arena slot.
    pub(crate) fn reserve(&mut self, rule: RuleIndex) -> Result<PublicationSlot, OperatingError> {
        let outputs = self
            .metadata
            .outputs
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?;
        if !outputs
            .iter()
            .any(|output| matches!(output.root, OutputRoot::CrossFile))
        {
            return Ok(PublicationSlot::Unused);
        }
        let worker =
            WorkerNumber(u32::try_from(self.arenas.len()).map_err(|_| OperatingError::NotRun)?);
        if worker == CROSS_FILE {
            return Err(OperatingError::NotRun);
        }
        self.arenas.push(ArenaBytes::default());
        self.references.push(ReferenceCount(0));
        Ok(PublicationSlot::Reserved(worker))
    }
    fn member(
        &self,
        rule: RuleIndex,
        output: ObservationIndex,
    ) -> Result<BoundMember, OperatingError> {
        match self
            .roots
            .get(rule.0)
            .and_then(|outputs| outputs.get(output.0 as usize))
        {
            Some(CrossRoot::FileFrontier { token, .. }) => Ok(BoundMember::Restored(token.clone())),
            Some(CrossRoot::Named { admitted, .. }) => Ok(BoundMember::Current(admitted.clone())),
            Some(CrossRoot::Pending | CrossRoot::Unavailable) | None => Err(incomplete()),
        }
    }
    /// Freeze this task's actual producer handles before the ready batch executes.
    /// Workers receive immutable words only; the coordinator retains the physical leases.
    pub(crate) fn bindings(&self, rule: RuleIndex) -> Result<Vec<BoundInput>, OperatingError> {
        let bindings = self.metadata.bindings.get(rule.0).ok_or_else(incomplete)?;
        Ok(bindings.iter().filter(|binding| binding.schema.scope == ryo_values::domain::plan_parts::types::StepScope::CrossFile).map(|binding| {
            let members = binding.producers.iter().map(|producer| match producer {
                ryo_execution_domain::domain::observation_schedule::types::ResolvedObservationProducer::Output { rule, output } => self.member(*rule, *output),
                ryo_execution_domain::domain::observation_schedule::types::ResolvedObservationProducer::NoQuery { .. } => Err(incomplete()),
            }).collect();
            BoundInput {
                input: binding.input,
                members,
            }
        }).collect())
    }
    /// Install a closure's owned transfer before retiring its consumed inputs.
    /// Re-exported inherited arena references are pinned before the producer publication can release them.
    pub(crate) fn install(
        &mut self,
        rule: RuleIndex,
        slot: PublicationSlot,
        transfer: CrossPublicationTransfer,
        admitted: Vec<AdmittedOutput>,
    ) -> Result<(), OperatingError> {
        let count = admitted.len();
        let mut admitted = admitted
            .into_iter()
            .map(
                |AdmittedOutput {
                     output,
                     publication,
                 }| (output.0, publication),
            )
            .collect::<std::collections::BTreeMap<_, _>>();
        if admitted.len() != count {
            return Err(incomplete());
        }
        if admitted.len()
            != transfer
                .roots
                .iter()
                .filter(|root| {
                    matches!(
                        root.words,
                        ryo_values::domain::records::types::RootWords::Published(_)
                    )
                })
                .count()
        {
            return Err(incomplete());
        }
        let PublicationSlot::Reserved(expected) = slot else {
            return Err(OperatingError::NotRun);
        };
        if transfer.worker != expected
            || expected == CROSS_FILE
            || (expected.0 as usize) < self.inherited.0
        {
            return Err(OperatingError::NotRun);
        }
        let at = expected.0 as usize;
        if self.arenas.get(at).is_none_or(|arena| !arena.0.is_empty()) {
            return Err(OperatingError::NotRun);
        }
        let outputs = self
            .metadata
            .outputs
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?;
        let mut seen = vec![false; outputs.len()];
        for root in &transfer.roots {
            let output = root.output.0 as usize;
            if outputs
                .get(output)
                .is_none_or(|output| !matches!(output.root, OutputRoot::CrossFile))
                || seen.get(output).copied().unwrap_or(true)
            {
                return Err(OperatingError::NotRun);
            }
            seen[output] = true;
            if let Some(publication) = admitted.get(&root.output.0) {
                publication.validate_transfer_readiness(&root.readiness)?;
            } else if root.readiness == ProducerReadiness::Complete {
                return Err(incomplete());
            }
            if matches!(
                root.words,
                ryo_values::domain::records::types::RootWords::Published(_)
            ) != admitted.contains_key(&root.output.0)
            {
                return Err(incomplete());
            }
            for arena in &root.arenas {
                if *arena == CROSS_FILE || arena.0 as usize >= self.arenas.len() {
                    return Err(incomplete());
                }
            }
        }
        // Prepare every lease and checked reference increment before changing
        // the coordinator's roots. A malformed later output leaves no earlier
        // publication visible, and dropping this local vector releases leases.
        let mut additions = vec![0usize; self.references.len()];
        let mut prepared = Vec::with_capacity(transfer.roots.len());
        for root in transfer.roots {
            let output = root.output.0 as usize;
            if let ryo_values::domain::records::types::RootWords::Published(words) = root.words {
                // The pack boundary listed each referenced arena once. Validate uniqueness
                // rather than silently over-counting a malformed transfer's lease.
                let mut referenced = Vec::with_capacity(root.arenas.len());
                for arena in root.arenas {
                    if referenced.contains(&arena) {
                        return Err(incomplete());
                    }
                    referenced.push(arena);
                    let at = arena.0 as usize;
                    let added = additions.get_mut(at).ok_or_else(incomplete)?;
                    *added = added.checked_add(1).ok_or(OperatingError::NotRun)?;
                    self.references
                        .get(at)
                        .ok_or_else(incomplete)?
                        .0
                        .checked_add(*added)
                        .ok_or(OperatingError::NotRun)?;
                }
                let backings = referenced
                    .iter()
                    .map(|worker| {
                        if *worker == expected {
                            Ok(&transfer.arena)
                        } else {
                            self.arenas.get(worker.0 as usize).ok_or_else(incomplete)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let publication = self.owner.publish(
                    ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Packed {
                        words,
                        arenas: &backings,
                    },
                )?;
                prepared.push((
                    output,
                    CrossRoot::Named {
                        publication,
                        admitted: admitted
                            .remove(&u32::try_from(output).map_err(|_| incomplete())?)
                            .ok_or_else(incomplete)?,
                        arenas: referenced,
                    },
                ));
            } else {
                prepared.push((output, CrossRoot::Unavailable));
            }
        }
        self.arenas[at] = transfer.arena;
        for (count, added) in self.references.iter_mut().zip(additions) {
            count.0 += added; // checked against the old count above
        }
        for (output, root) in prepared {
            self.roots[rule.0][output] = root;
        }
        for (output, declared) in outputs.iter().enumerate() {
            if matches!(declared.root, OutputRoot::CrossFile) && !seen[output] {
                self.roots[rule.0][output] = CrossRoot::Unavailable;
            }
        }
        self.release_unused_arena(expected);
        Ok(())
    }
    pub(crate) fn finish(&mut self, consumer: RuleIndex) -> Result<(), OperatingError> {
        let finished = self
            .finished
            .get_mut(consumer.0)
            .ok_or(OperatingError::NotRun)?;
        if *finished == FinishState::Finished {
            return Err(OperatingError::NotRun);
        }
        *finished = FinishState::Finished;
        for edge in &self.metadata.retire[consumer.0] {
            if !matches!(
                self.metadata.outputs[edge.producer.0][edge.output.0 as usize].root,
                OutputRoot::CrossFile
            ) {
                continue;
            }
            let count = self
                .remaining
                .get_mut(edge.producer.0)
                .and_then(|outputs| outputs.get_mut(edge.output.0 as usize))
                .ok_or(OperatingError::NotRun)?;
            count.0 = count.0.checked_sub(1).ok_or(OperatingError::NotRun)?;
            if count.0 == 0 {
                self.release(*edge)?;
            }
        }
        Ok(())
    }
    fn release(&mut self, edge: ObservationRetirement) -> Result<(), OperatingError> {
        let root = self
            .roots
            .get_mut(edge.producer.0)
            .and_then(|outputs| outputs.get_mut(edge.output.0 as usize))
            .ok_or(OperatingError::NotRun)?;
        let released = std::mem::replace(root, CrossRoot::Unavailable);
        let released = match released {
            CrossRoot::FileFrontier {
                publication,
                arenas,
                ..
            }
            | CrossRoot::Named {
                publication,
                arenas,
                ..
            } => Some((publication, arenas)),
            CrossRoot::Pending | CrossRoot::Unavailable => None,
        };
        if let Some((publication, arenas)) = released {
            drop(publication);
            for arena in arenas {
                let count = self
                    .references
                    .get_mut(arena.0 as usize)
                    .ok_or(OperatingError::NotRun)?;
                count.0 = count.0.checked_sub(1).ok_or(OperatingError::NotRun)?;
                self.release_unused_arena(arena);
            }
        }
        Ok(())
    }
    pub(crate) fn release_unused_arena(&mut self, worker: WorkerNumber) {
        let index = worker.0 as usize;
        if index >= self.inherited.0 && self.references.get(index) == Some(&ReferenceCount(0)) {
            if let Some(arena) = self.arenas.get_mut(index) {
                arena.0.clear();
                arena.0.shrink_to_fit();
            }
        }
    }
}
impl Drop for CrossPublications<'_> {
    fn drop(&mut self) {
        // Every transfer and dependent publication is invocation-local. Unwind drops
        // all owned roots, then returns the inherited vector without renumbering its slots.
        self.roots.clear();
        self.arenas.truncate(self.inherited.0);
    }
}
pub(crate) fn incomplete() -> OperatingError {
    OperatingError::ObservationIncomplete {
        reason: PUBLICATION_INCOMPLETE.into(),
    }
}
