//! Request-graph named publications across existing cross ready batches. No source IO or producer evaluation.
use crate::application::cross_file::types::PublicationSlot;
use crate::domain::{
    coverage::types::RuleIndex,
    crossing::types::CrossPublicationTransfer,
    file_program::types::{GraphObservationRoot, GraphObservations, ObservationRetirement},
    operating::errors::OperatingError,
};
use ryo_relations::domain::relations::types::{CROSS_FILE, WorkerNumber};
use ryo_values::domain::{
    observations::types::{ObservationIndex, ProducerReadiness},
    stored_facts::types::{ArenaBytes, KeptWords},
};
use std::rc::Rc;
use std::sync::Arc;

enum CrossRoot {
    Pending,
    Complete {
        publication: crate::application::file_dag::relations::PackedPublication,
        arenas: Vec<WorkerNumber>,
    },
    Unavailable,
}
/// A complete joined file population, constructed only after every expected file frame validates.
pub(crate) struct FrontierPopulation<'a> {
    frames: Vec<ryo_values::domain::observations::types::ValidatedFrontiers<'a>>,
}
/// A checked physical frontier; aliases share this exact owned payload and common lease.
pub(crate) struct PreparedFrontier {
    publication: crate::application::file_dag::relations::PackedPublication,
    arenas: Vec<WorkerNumber>,
}
/// One invocation extends the existing arena vector; stable slot indices never move during execution.
pub(crate) struct CrossPublications<'a> {
    graph: &'a GraphObservations,
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
        graph: &'a GraphObservations,
        owner: Rc<crate::application::file_dag::relations::RelationOwner>,
        arenas: &'a mut Vec<ArenaBytes>,
    ) -> Self {
        let inherited = arenas.len();
        Self {
            graph,
            owner,
            inherited,
            references: vec![0; inherited],
            arenas,
            roots: graph
                .outputs
                .iter()
                .map(|outputs| {
                    outputs
                        .iter()
                        .map(|output| match output.root {
                            GraphObservationRoot::CrossFile => CrossRoot::Pending,
                            GraphObservationRoot::PerFile(_)
                            | GraphObservationRoot::Unavailable => CrossRoot::Unavailable,
                        })
                        .collect()
                })
                .collect(),
            remaining: graph
                .outputs
                .iter()
                .map(|outputs| {
                    outputs
                        .iter()
                        .map(|output| output.consumers.len())
                        .collect()
                })
                .collect(),
            finished: vec![false; graph.bindings.len()],
        }
    }
    pub(crate) fn arenas(&self) -> &[ArenaBytes] {
        self.arenas
    }
    /// Decode and retain one actual physical bucket through the common publication owner.
    pub(crate) fn prepare_frontier(
        &self,
        words: KeptWords,
    ) -> Result<PreparedFrontier, OperatingError> {
        let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
        let _table = crate::domain::table_words::service::decode_complete(
            &words.0,
            ryo_language::domain::symbols::service::places_mut(&mut symbols),
        )
        .map_err(|error| OperatingError::ObservationIncomplete {
            reason: error.to_string(),
        })?;
        let mut arenas = Vec::new();
        for place in ryo_language::domain::symbols::service::places_of(&symbols) {
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
        if !frontier_complete(slot, readiness)? {
            return Err(incomplete());
        }
        let entry = self.graph.frontiers.get(slot).ok_or_else(incomplete)?;
        if !matches!(
            self.graph
                .outputs
                .get(entry.producer.0)
                .and_then(|outputs| outputs.get(entry.output.0 as usize))
                .map(|output| &output.root),
            Some(GraphObservationRoot::PerFile(_))
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
        *root = CrossRoot::Complete {
            publication: prepared.publication.clone(),
            arenas: prepared.arenas.clone(),
        };
        Ok(())
    }
    /// Prepared once for one task; an unavailable producer never becomes an empty source.
    pub(crate) fn binding_readiness(
        &self,
        rule: RuleIndex,
    ) -> Result<
        std::collections::BTreeMap<
            ryo_values::domain::observations::types::ObservationBindingId,
            bool,
        >,
        OperatingError,
    > {
        let bindings = self.graph.bindings.get(rule.0).ok_or_else(incomplete)?;
        Ok(bindings.iter().map(|binding| (binding.input.input, binding.input.producers.iter().all(|producer| match producer {
            crate::domain::file_program::types::ResolvedObservationProducer::NoQuery { .. } => false,
            crate::domain::file_program::types::ResolvedObservationProducer::Output { rule, output } => matches!(self.roots.get(rule.0).and_then(|roots| roots.get(output.0 as usize)), Some(CrossRoot::Complete { .. })),
        }))).collect())
    }
    /// Only phases with demanded cross roots reserve an ordinary arena slot.
    pub(crate) fn reserve(&mut self, rule: RuleIndex) -> Result<PublicationSlot, OperatingError> {
        let outputs = self
            .graph
            .outputs
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?;
        if !outputs
            .iter()
            .any(|output| matches!(output.root, GraphObservationRoot::CrossFile))
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
    pub(crate) fn words(
        &self,
        rule: RuleIndex,
        output: ObservationIndex,
    ) -> Result<Arc<KeptWords>, OperatingError> {
        match self
            .roots
            .get(rule.0)
            .and_then(|outputs| outputs.get(output.0 as usize))
        {
            Some(CrossRoot::Complete { publication, .. }) => Ok(publication.words()),
            Some(CrossRoot::Pending | CrossRoot::Unavailable) | None => Err(incomplete()),
        }
    }
    /// Freeze this task's actual producer handles before the ready batch executes.
    /// Workers receive immutable words only; the coordinator retains the physical leases.
    pub(crate) fn bindings(
        &self,
        rule: RuleIndex,
    ) -> Result<Vec<crate::domain::crossing::types::BoundInputWords>, OperatingError> {
        let bindings = self.graph.bindings.get(rule.0).ok_or_else(incomplete)?;
        Ok(bindings.iter().filter(|binding| binding.input.schema.scope == ryo_values::domain::plans::types::StepScope::CrossFile).map(|binding| {
            let words = binding.input.producers.iter().map(|producer| match producer {
                crate::domain::file_program::types::ResolvedObservationProducer::Output { rule, output } => self.words(*rule, *output),
                crate::domain::file_program::types::ResolvedObservationProducer::NoQuery { .. } => Err(incomplete()),
            }).collect();
            crate::domain::crossing::types::BoundInputWords {
                input: binding.input.input,
                kinds: binding.input.schema.columns.iter().map(|column| column.kind).collect(),
                union: binding.union.clone(), words,
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
    ) -> Result<(), OperatingError> {
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
            .graph
            .outputs
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?;
        let mut seen = vec![false; outputs.len()];
        for root in &transfer.roots {
            let output = root.output.0 as usize;
            if outputs
                .get(output)
                .is_none_or(|output| !matches!(output.root, GraphObservationRoot::CrossFile))
                || seen.get(output).copied().unwrap_or(true)
            {
                return Err(OperatingError::NotRun);
            }
            seen[output] = true;
            if (root.readiness == ProducerReadiness::Complete) != root.words.is_some() {
                return Err(incomplete());
            }
            for arena in &root.arenas {
                if *arena == CROSS_FILE || arena.0 as usize >= self.arenas.len() {
                    return Err(incomplete());
                }
            }
        }
        self.arenas[at] = transfer.arena;
        for root in transfer.roots {
            let output = root.output.0 as usize;
            if let Some(words) = root.words {
                // The pack boundary listed each referenced arena once. Validate uniqueness
                // rather than silently over-counting a malformed transfer's lease.
                let mut referenced = Vec::with_capacity(root.arenas.len());
                for arena in root.arenas {
                    if referenced.contains(&arena) {
                        return Err(incomplete());
                    }
                    referenced.push(arena);
                    let count = self
                        .references
                        .get_mut(arena.0 as usize)
                        .ok_or_else(incomplete)?;
                    *count = count.checked_add(1).ok_or(OperatingError::NotRun)?;
                }
                let backings = referenced
                    .iter()
                    .map(|worker| self.arenas.get(worker.0 as usize).ok_or_else(incomplete))
                    .collect::<Result<Vec<_>, _>>()?;
                let publication = self.owner.packed_publication(words, &backings)?;
                self.roots[rule.0][output] = CrossRoot::Complete {
                    publication,
                    arenas: referenced,
                };
            } else {
                self.roots[rule.0][output] = CrossRoot::Unavailable;
            }
        }
        for (output, declared) in outputs.iter().enumerate() {
            if matches!(declared.root, GraphObservationRoot::CrossFile) && !seen[output] {
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
        for edge in &self.graph.retire[consumer.0] {
            if !matches!(
                self.graph.outputs[edge.producer.0][edge.output.0 as usize].root,
                GraphObservationRoot::CrossFile
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
        if let CrossRoot::Complete {
            publication,
            arenas,
        } = released
        {
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
    graph: &GraphObservations,
    files: &[ryo_relations::domain::relations::types::FileId],
    frames: &'a [crate::domain::coverage::types::FileObservationReadiness],
) -> Result<FrontierPopulation<'a>, OperatingError> {
    use ryo_values::domain::observations::{
        service::validate_frontiers, types::ObservationReadinessCount,
    };
    let count =
        ObservationReadinessCount(u32::try_from(graph.frontiers.len()).map_err(|_| incomplete())?);
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
            validate_frontiers(frame, graph.layout, count).map_err(|_| incomplete())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FrontierPopulation { frames })
}

/// Logical completion proof, separate from the physical packed payload.
pub(crate) fn frontier_complete(
    slot: usize,
    readiness: &FrontierPopulation<'_>,
) -> Result<bool, OperatingError> {
    use ryo_values::domain::observations::{
        service::frontier_state, types::ObservationReadinessSlot,
    };
    let slot = ObservationReadinessSlot(u32::try_from(slot).map_err(|_| incomplete())?);
    for frame in &readiness.frames {
        if frontier_state(*frame, slot).map_err(|_| incomplete())? != ProducerReadiness::Complete {
            return Ok(false);
        }
    }
    Ok(true)
}
