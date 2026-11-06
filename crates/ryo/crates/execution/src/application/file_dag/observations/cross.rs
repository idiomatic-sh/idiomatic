//! Named publication leases across declared project ready batches. No source IO or producer evaluation.
use crate::application::file_dag::frontiers::types::{BoundInput, BoundMember, PublicationSlot};
use crate::application::file_dag::program::preparation::types::ObservationMetadata;
use crate::application::file_dag::program::preparation::types::OutputRoot;
use crate::domain::completion::types::CoverageState;
use ryo_relations::domain::relations::types::{CROSS_FILE, WorkerNumber};
use ryo_values::domain::{
    records::types::{ObservationIndex, ProducerReadiness},
    stored_facts::types::ArenaBytes,
};
use std::rc::Rc;
use {
    crate::domain::coverage::types::RuleIndex,
    crate::domain::file_program::types::ObservationRetirement,
    crate::domain::operating::errors::OperatingError,
    ryo_values::domain::records::types::CrossPublicationTransfer,
};

enum CrossRoot {
    Pending,
    FileFrontier {
        publication: crate::application::file_dag::relations::packed::service::PackedPublication,
        token: crate::application::file_dag::program::restored::types::ValidatedFrontier,
        arenas: Vec<WorkerNumber>,
    },
    Named {
        publication: crate::application::file_dag::relations::packed::service::PackedPublication,
        admitted: crate::application::file_dag::program::protocol::types::ObservedPublication,
        arenas: Vec<WorkerNumber>,
    },
    Unavailable,
}
/// Every expected file has one validated frame; each logical state retains its file identity.
pub(crate) struct FrontierPopulation<'a> {
    frames: Vec<(
        ryo_relations::domain::relations::types::FileId,
        ryo_values::domain::observations::types::ValidatedFrontiers<'a>,
    )>,
}
/// A checked physical frontier; aliases share this exact owned payload and common lease.
pub(crate) struct PreparedFrontier {
    publication: crate::application::file_dag::relations::packed::service::PackedPublication,
    token: crate::application::file_dag::program::restored::types::ValidatedFrontier,
    arenas: Vec<WorkerNumber>,
}
/// One invocation extends the existing arena vector; stable slot indices never move during execution.
pub(crate) struct CrossPublications<'a> {
    metadata: &'a ObservationMetadata,
    owner: Rc<crate::application::file_dag::relations::RelationOwner>,
    arenas: &'a mut Vec<ArenaBytes>,
    inherited: usize,
    references: Vec<usize>,
    roots: Vec<Vec<CrossRoot>>,
    remaining: Vec<Vec<usize>>,
    finished: Vec<bool>,
}
impl<'a> CrossPublications<'a> {
    pub(crate) fn new(
        metadata: &'a ObservationMetadata,
        scope: &crate::application::file_dag::program::facade_runtime::types::PublicationScope,
        arenas: &'a mut Vec<ArenaBytes>,
    ) -> Self {
        let inherited = arenas.len();
        Self {
            metadata,
            owner: Rc::clone(scope.owner()),
            inherited,
            references: vec![0; inherited],
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
                        .map(|output| output.consumers.len())
                        .collect()
                })
                .collect(),
            finished: vec![false; metadata.bindings.len()],
        }
    }
    pub(crate) fn arenas(&self) -> &[ArenaBytes] {
        self.arenas
    }
    /// Decode and retain one actual physical bucket through the common publication owner.
    pub(crate) fn prepare_frontier(
        &self,
        token: crate::application::file_dag::program::restored::types::ValidatedFrontier,
    ) -> Result<PreparedFrontier, OperatingError> {
        let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
        let backing = std::sync::Arc::new(token.words().clone());
        let ryo_values::domain::table_words::types::Admitted {
            table: metadata, ..
        } = ryo_values::domain::compact_table::service::admit_compact(
            ryo_values::domain::table_words::types::CompactBacking(std::sync::Arc::clone(&backing)),
            ryo_language::domain::symbols::service::places_mut(&mut symbols),
            ryo_values::domain::table_words::types::DecodeMode::Complete,
        )
        .map_err(|error| OperatingError::ObservationIncomplete {
            reason: error.to_string(),
        })?;
        drop(metadata);
        let words =
            std::sync::Arc::try_unwrap(backing).map_err(|_| OperatingError::WrongOperation)?;
        let mut arenas = Vec::new();
        for place in ryo_language::domain::names::service::places_of(&symbols) {
            if let ryo_relations::domain::relations::types::Origin::Derived(worker) = place.origin {
                if worker == CROSS_FILE || worker.0 as usize >= self.inherited {
                    return Err(incomplete());
                }
                if !arenas.contains(&worker) {
                    arenas.push(worker);
                }
            }
        }
        let backings = arenas
            .iter()
            .map(|worker| self.arenas.get(worker.0 as usize).ok_or_else(incomplete))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PreparedFrontier {
            publication: self.owner.packed_publication(words, &backings)?,
            token,
            arenas,
        })
    }
    /// Each logical output retains its own readiness even when names share one physical bucket.
    pub(crate) fn install_frontier(
        &mut self,
        slot: usize,
        prepared: &PreparedFrontier,
        readiness: &FrontierPopulation<'_>,
    ) -> Result<(), OperatingError> {
        if !frontier_compatible(slot, readiness, prepared.token.coverage())? {
            return Err(incomplete());
        }
        let entry = self.metadata.frontiers.get(slot).ok_or_else(incomplete)?;
        if !matches!(
            self.metadata
                .outputs
                .get(entry.producer.0)
                .and_then(|outputs| outputs.get(entry.output.0 as usize))
                .map(|output| &output.root),
            Some(OutputRoot::PerFile)
        ) {
            return Err(incomplete());
        }
        let root = self
            .roots
            .get_mut(entry.producer.0)
            .and_then(|outputs| outputs.get_mut(entry.output.0 as usize))
            .ok_or_else(incomplete)?;
        if !matches!(root, CrossRoot::Unavailable) {
            return Err(incomplete());
        }
        for arena in &prepared.arenas {
            let count = self
                .references
                .get_mut(arena.0 as usize)
                .ok_or_else(incomplete)?;
            *count = count.checked_add(1).ok_or_else(incomplete)?;
        }
        *root = CrossRoot::FileFrontier {
            publication: prepared.publication.clone(),
            token: prepared.token.clone(),
            arenas: prepared.arenas.clone(),
        };
        Ok(())
    }
    /// Prepared once for one task. Availability retains the producer's admitted
    /// coverage; a partial publication is usable without becoming complete-empty.
    pub(crate) fn binding_readiness(
        &self,
        rule: RuleIndex,
    ) -> Result<
        std::collections::BTreeMap<
            ryo_values::domain::plan_parts::types::ObservationBindingId,
            bool,
        >,
        OperatingError,
    > {
        let bindings = self.metadata.bindings.get(rule.0).ok_or_else(incomplete)?;
        Ok(bindings.iter().map(|binding| (binding.input, binding.producers.iter().all(|producer| match producer {
            crate::domain::file_program::types::ResolvedObservationProducer::NoQuery { .. } => false,
            crate::domain::file_program::types::ResolvedObservationProducer::Output { rule, output } => matches!(self.roots.get(rule.0).and_then(|roots| roots.get(output.0 as usize)), Some(CrossRoot::FileFrontier { .. } | CrossRoot::Named { .. })),
        }))).collect())
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
        self.references.push(0);
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
                crate::domain::file_program::types::ResolvedObservationProducer::Output { rule, output } => self.member(*rule, *output),
                crate::domain::file_program::types::ResolvedObservationProducer::NoQuery { .. } => Err(incomplete()),
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
        admitted: Vec<(
            ObservationIndex,
            crate::application::file_dag::program::protocol::types::ObservedPublication,
        )>,
    ) -> Result<(), OperatingError> {
        let count = admitted.len();
        let mut admitted = admitted
            .into_iter()
            .map(|(output, publication)| (output.0, publication))
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
            || (expected.0 as usize) < self.inherited
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
                publication.validate_transfer_readiness(&self.owner, &root.readiness)?;
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
                let publication = self.owner.packed_publication(words, &backings)?;
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
            *count += added; // checked against the old count above
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
        if *finished {
            return Err(OperatingError::NotRun);
        }
        *finished = true;
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
            *count = count.checked_sub(1).ok_or(OperatingError::NotRun)?;
            if *count == 0 {
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
                *count = count.checked_sub(1).ok_or(OperatingError::NotRun)?;
                self.release_unused_arena(arena);
            }
        }
        Ok(())
    }
    fn release_unused_arena(&mut self, worker: WorkerNumber) {
        let index = worker.0 as usize;
        if index >= self.inherited && self.references.get(index) == Some(&0) {
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
        self.arenas.truncate(self.inherited);
    }
}
fn incomplete() -> OperatingError {
    OperatingError::ObservationIncomplete {
        reason: "cross producer publication is unavailable or malformed".into(),
    }
}

/// Validate the file-to-frame join once per invocation, before any producer bits are read.
/// This does not infer completion from row count or from an absent file frame.
pub(crate) fn frontier_readiness<'a>(
    metadata: &ObservationMetadata,
    files: &[ryo_relations::domain::relations::types::FileId],
    frames: &'a [crate::domain::coverage::types::FileObservationReadiness],
) -> Result<FrontierPopulation<'a>, OperatingError> {
    use ryo_values::domain::observations::service::validate_frontiers;
    use ryo_values::domain::records::types::ObservationReadinessCount;
    let count = ObservationReadinessCount(
        u32::try_from(metadata.frontiers.len()).map_err(|_| incomplete())?,
    );
    let mut indexed = std::collections::BTreeMap::new();
    for frame in frames {
        if indexed.insert(frame.file.0, &frame.root).is_some() {
            return Err(incomplete());
        }
    }
    let frames = files
        .iter()
        .map(|file| {
            let frame = indexed.get(&file.0).ok_or_else(incomplete)?;
            validate_frontiers(frame, metadata.layout, count)
                .map(|frame| (*file, frame))
                .map_err(|_| incomplete())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FrontierPopulation { frames })
}

/// Logical unavailability must agree with independently validated physical coverage.
fn frontier_compatible(
    slot: usize,
    readiness: &FrontierPopulation<'_>,
    physical: &CoverageState,
) -> Result<bool, OperatingError> {
    use ryo_values::domain::observations::service::frontier_state;
    use ryo_values::domain::records::types::ObservationReadinessSlot;
    let slot = ObservationReadinessSlot(u32::try_from(slot).map_err(|_| incomplete())?);
    for (file, frame) in &readiness.frames {
        if frontier_state(*frame, slot).map_err(|_| incomplete())? != ProducerReadiness::Complete {
            // Logical unavailability may be localized only by independently
            // admitted physical provenance, never by a cached readiness bit.
            if !matches!(physical, CoverageState::Partial { files }
                if files.contains(&None) || files.contains(&Some(*file)))
                && !matches!(physical, CoverageState::Inapplicable { files }
                    if files.contains(file))
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Restore demanded file roots through the same request owner, once per physical bucket.
pub(crate) fn restore_frontiers(
    publications: &mut CrossPublications<'_>,
    program: &std::sync::Arc<
        crate::application::file_dag::program::preparation::types::PreparedProgram,
    >,
    services: &crate::application::file_dag::program::facade_runtime::types::ProgramServices,
    publication: &crate::application::file_dag::program::facade_runtime::types::PublicationScope,
    facts: &dyn crate::outbound::facts::interface::FactsFinish,
    corpus: &[ryo_values::domain::files::types::CorpusInput],
    covered: &[crate::domain::coverage::types::Covered],
    skipped: &[ryo_values::domain::runs::types::Skipped],
    inapplicable: &[crate::domain::coverage::types::InapplicableFiles],
    frames: &[crate::domain::coverage::types::FileObservationReadiness],
    first_bucket: &[ryo_values::domain::stored_facts::types::Bucket],
    selected: &[bool],
) -> Result<(), OperatingError> {
    use crate::application::file_dag::program::facade_runtime;
    use ryo_values::domain::stored_facts::types::Bucket;

    use crate::application::file_dag::observations::cross::{PreparedFrontier, frontier_readiness};
    // One set of inapplicable files per rule, or none for the whole run.
    if !inapplicable.is_empty() && inapplicable.len() != program.program().rules().len() {
        return Err(OperatingError::NotRun);
    }
    let metadata = program
        .observation_metadata()
        .ok_or(OperatingError::NotRun)?;
    let files = (0..corpus.len())
        .map(|file| {
            u32::try_from(file)
                .map(ryo_relations::domain::relations::types::FileId)
                .map_err(|_| OperatingError::NotRun)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let readiness = match frontier_readiness(metadata, &files, frames) {
        Ok(readiness) => readiness,
        Err(OperatingError::ObservationIncomplete { .. }) => return Ok(()),
        Err(error) => return Err(error),
    };
    let mut physical =
        std::collections::BTreeMap::<Bucket, Result<PreparedFrontier, OperatingError>>::new();
    for (slot, root) in metadata.frontiers.iter().enumerate() {
        let needed = metadata
            .outputs
            .get(root.producer.0)
            .and_then(|outputs| outputs.get(root.output.0 as usize))
            .ok_or(OperatingError::NotRun)?
            .consumers
            .iter()
            .any(|consumer| selected.get(consumer.0).copied().unwrap_or(false));
        if !needed {
            continue;
        }
        let first = *first_bucket
            .get(root.producer.0)
            .ok_or(OperatingError::NotRun)?;
        let bucket =
            ryo_values::domain::stored_facts::service::frontier_bucket(first, root.frontier);
        if let std::collections::btree_map::Entry::Vacant(entry) = physical.entry(bucket) {
            let required = facade_runtime::service::required_partitions(
                program,
                root.producer,
                corpus,
                covered,
                skipped,
                inapplicable
                    .get(root.producer.0)
                    .map_or(&[][..], |files| &files.0),
                services,
            )?;
            let payload =
                facts
                    .kept(bucket)
                    .map_err(|error| OperatingError::ObservationIncomplete {
                        reason: error.to_string(),
                    })?;
            let restored =
                crate::application::file_dag::program::restored::validate_restored_frontier(
                    program,
                    root.producer,
                    root.frontier,
                    first,
                    bucket,
                    &required,
                    payload,
                    publication,
                )
                .and_then(|token| publications.prepare_frontier(token));
            entry.insert(restored);
        }
        match physical.get(&bucket).ok_or(OperatingError::NotRun)? {
            Ok(prepared) => publications.install_frontier(slot, prepared, &readiness)?,
            Err(OperatingError::ObservationIncomplete { .. }) => (),
            Err(error) => return Err(error.clone()),
        }
    }
    Ok(())
}
