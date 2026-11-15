//! A delivered occurrence's install into the arena its slot reserved, and each output released once its last consumer is done.

use crate::application::file_dag::program::observation_stage::types::AdmittedOutput;
use crate::application::file_dag::program::preparation::types::OutputRoot;
use crate::application::file_dag::publications::service::{incomplete, output_place, worker_place};
use crate::application::file_dag::publications::types::{
    CrossRoot, FinishState, PublicationSlot, Publications, ReferenceCount, RestoredOutput,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::observation_schedule::types::ObservationRetirement;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{CROSS_FILE, WorkerNumber};
use ryo_values::domain::records::types::{CrossPublicationTransfer, ProducerReadiness, RootWords};
use std::collections::BTreeMap;

impl Publications<'_> {
    /// Installs what one delivered occurrence of `rule` transfers into the arena its slot reserved, before its consumed inputs
    /// retire.
    ///
    /// # Errors
    ///
    /// The publication's refusal of a malformed transfer: a slot it did not reserve, an output it does not publish after the
    /// files, published words without their admitted rows or the other way round, or a readiness the rows disagree with.
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
            .collect::<BTreeMap<_, _>>();
        if admitted.len() != count {
            return Err(incomplete());
        }
        if admitted.len()
            != transfer
                .roots
                .iter()
                .filter(|root| matches!(root.words, RootWords::Published(_)))
                .count()
        {
            return Err(incomplete());
        }
        let PublicationSlot::Reserved(expected) = slot else {
            return Err(OperatingError::NotRun);
        };
        if transfer.worker != expected
            || expected == CROSS_FILE
            || worker_place(expected).0 < self.inherited.0
        {
            return Err(OperatingError::NotRun);
        }
        let at = worker_place(expected).0;
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
            let output = output_place(root.output).0;
            if outputs
                .get(output)
                .is_none_or(|output| !matches!(output.root, OutputRoot::CrossFile))
                || seen.get(output).copied().unwrap_or(true)
            {
                return Err(OperatingError::NotRun);
            }
            if let Some(mark) = seen.get_mut(output) {
                *mark = true;
            }
            if let Some(publication) = admitted.get(&root.output.0) {
                publication.validate_transfer_readiness(&root.readiness, &self.stage)?;
            } else if root.readiness == ProducerReadiness::Complete {
                return Err(incomplete());
            }
            if matches!(root.words, RootWords::Published(_))
                != admitted.contains_key(&root.output.0)
            {
                return Err(incomplete());
            }
            for arena in &root.arenas {
                if *arena == CROSS_FILE || worker_place(*arena).0 >= self.arenas.len() {
                    return Err(incomplete());
                }
            }
        }
        // Every reference is counted and checked before the roots change: a malformed later output leaves no earlier one visible.
        let mut additions = vec![0usize; self.references.len()];
        let mut prepared = Vec::with_capacity(transfer.roots.len());
        for root in transfer.roots {
            let output = output_place(root.output).0;
            if matches!(root.words, RootWords::Published(_)) {
                // The pack boundary lists each referenced arena once: a repeat is a malformed transfer, never counted twice.
                let mut referenced = Vec::with_capacity(root.arenas.len());
                for arena in root.arenas {
                    if referenced.contains(&arena) {
                        return Err(incomplete());
                    }
                    referenced.push(arena);
                    let added = additions
                        .get_mut(worker_place(arena).0)
                        .ok_or_else(incomplete)?;
                    *added = added.checked_add(1).ok_or(OperatingError::NotRun)?;
                    self.references
                        .get(worker_place(arena).0)
                        .ok_or_else(incomplete)?
                        .0
                        .checked_add(*added)
                        .ok_or(OperatingError::NotRun)?;
                }
                prepared.push((
                    output,
                    CrossRoot::Named {
                        publication: admitted.remove(&root.output.0).ok_or_else(incomplete)?,
                        arenas: referenced,
                    },
                ));
            } else {
                prepared.push((output, CrossRoot::Unavailable));
            }
        }
        if let Some(arena) = self.arenas.get_mut(at) {
            *arena = transfer.arena;
        }
        for (count, added) in self.references.iter_mut().zip(additions) {
            count.0 = count.0.saturating_add(added);
        }
        for (output, root) in prepared {
            if let Some(place) = self
                .roots
                .get_mut(rule.0)
                .and_then(|roots| roots.get_mut(output))
            {
                *place = root;
            }
        }
        for (output, declared) in outputs.iter().enumerate() {
            if matches!(declared.root, OutputRoot::CrossFile)
                && !seen.get(output).copied().unwrap_or(true)
                && let Some(place) = self
                    .roots
                    .get_mut(rule.0)
                    .and_then(|roots| roots.get_mut(output))
            {
                *place = CrossRoot::Unavailable;
            }
        }
        self.release_unused_arena(expected);
        Ok(())
    }

    /// `consumer` is done with its inputs: each output it was the last consumer of is released.
    ///
    /// # Errors
    ///
    /// `NotRun` for a rule outside the program or one already finished.
    pub(crate) fn finish(&mut self, consumer: RuleIndex) -> Result<(), OperatingError> {
        let finished = self
            .finished
            .get_mut(consumer.0)
            .ok_or(OperatingError::NotRun)?;
        if *finished == FinishState::Finished {
            return Err(OperatingError::NotRun);
        }
        *finished = FinishState::Finished;
        let edges = self
            .metadata
            .retire
            .get(consumer.0)
            .ok_or(OperatingError::NotRun)?;
        for edge in edges {
            if !matches!(
                self.metadata
                    .outputs
                    .get(edge.producer.0)
                    .and_then(|outputs| outputs.get(output_place(edge.output).0))
                    .map(|output| output.root),
                Some(OutputRoot::CrossFile)
            ) {
                continue;
            }
            let count = self
                .remaining
                .get_mut(edge.producer.0)
                .and_then(|outputs| outputs.get_mut(output_place(edge.output).0))
                .ok_or(OperatingError::NotRun)?;
            count.0 = count.0.checked_sub(1).ok_or(OperatingError::NotRun)?;
            if count.0 == 0 {
                self.release(*edge)?;
            }
        }
        Ok(())
    }

    /// Releases one output and the arenas only it referenced.
    fn release(&mut self, edge: ObservationRetirement) -> Result<(), OperatingError> {
        let root = self
            .roots
            .get_mut(edge.producer.0)
            .and_then(|outputs| outputs.get_mut(output_place(edge.output).0))
            .ok_or(OperatingError::NotRun)?;
        let arenas = match std::mem::replace(root, CrossRoot::Unavailable) {
            CrossRoot::Restored(RestoredOutput { arenas, .. })
            | CrossRoot::Named { arenas, .. } => arenas,
            CrossRoot::Pending | CrossRoot::Unavailable => Vec::new(),
        };
        for arena in arenas {
            let count = self
                .references
                .get_mut(worker_place(arena).0)
                .ok_or(OperatingError::NotRun)?;
            count.0 = count.0.checked_sub(1).ok_or(OperatingError::NotRun)?;
            self.release_unused_arena(arena);
        }
        Ok(())
    }

    /// Empties an arena of this invocation no retained publication references.
    pub(crate) fn release_unused_arena(&mut self, worker: WorkerNumber) {
        let index = worker_place(worker).0;
        if index >= self.inherited.0
            && self.references.get(index) == Some(&ReferenceCount(0))
            && let Some(arena) = self.arenas.get_mut(index)
        {
            arena.0.clear();
            arena.0.shrink_to_fit();
        }
    }
}
