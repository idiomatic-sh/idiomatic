//! Restore demanded file roots through the same request owner, once per physical bucket.
use crate::application::file_dag::observations::cross::incomplete;
use crate::application::file_dag::observations::types::CrossRoot;
use crate::application::file_dag::observations::types::{
    CrossPublications, FileFrontiers, FrontierCompatibility, FrontierPopulation, FrontierSlot,
    PreparedFrontier, RuleSelection,
};
use crate::application::file_dag::program::preparation::types::ObservationMetadata;
use crate::application::file_dag::program::preparation::types::OutputRoot;
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::CROSS_FILE;
use ryo_values::domain::records::types::ProducerReadiness;

impl CrossPublications<'_> {
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
                if worker == CROSS_FILE || worker.0 as usize >= self.inherited.0 {
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
        slot: FrontierSlot,
        prepared: &PreparedFrontier,
        readiness: &FrontierPopulation<'_>,
    ) -> Result<(), OperatingError> {
        if frontier_compatible(slot, readiness, prepared.token.coverage())?
            == FrontierCompatibility::Incompatible
        {
            return Err(incomplete());
        }
        let entry = self.metadata.frontiers.get(slot.0).ok_or_else(incomplete)?;
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
            count.0 = count.0.checked_add(1).ok_or_else(incomplete)?;
        }
        *root = CrossRoot::FileFrontier {
            publication: prepared.publication.clone(),
            token: prepared.token.clone(),
            arenas: prepared.arenas.clone(),
        };
        Ok(())
    }
}

/// Validate the file-to-frame join once per invocation, before any producer bits are read.
/// This does not infer completion from row count or from an absent file frame.
pub(crate) fn frontier_readiness<'a>(
    metadata: &ObservationMetadata,
    files: &[ryo_relations::domain::relations::types::FileId],
    frames: &'a [ryo_execution_domain::domain::coverage::types::FileObservationReadiness],
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
                .map(|frontiers| FileFrontiers {
                    file: *file,
                    frontiers,
                })
                .map_err(|_| incomplete())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FrontierPopulation { frames })
}

/// Logical unavailability must agree with independently validated physical coverage.
fn frontier_compatible(
    slot: FrontierSlot,
    readiness: &FrontierPopulation<'_>,
    physical: &CoverageState,
) -> Result<FrontierCompatibility, OperatingError> {
    use ryo_values::domain::observations::service::frontier_state;
    use ryo_values::domain::records::types::ObservationReadinessSlot;
    let slot = ObservationReadinessSlot(u32::try_from(slot.0).map_err(|_| incomplete())?);
    for entry in &readiness.frames {
        let (file, frame) = (&entry.file, &entry.frontiers);
        if frontier_state(*frame, slot).map_err(|_| incomplete())? != ProducerReadiness::Complete {
            // Logical unavailability may be localized only by independently
            // admitted physical provenance, never by a cached readiness bit.
            if !matches!(physical, CoverageState::Partial { files, .. }
                if files.contains(&None) || files.contains(&Some(*file)))
                && !matches!(physical, CoverageState::Inapplicable { files }
                    if files.contains(file))
            {
                return Ok(FrontierCompatibility::Incompatible);
            }
        }
    }
    Ok(FrontierCompatibility::Compatible)
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
    covered: &[ryo_execution_domain::domain::coverage::types::Covered],
    skipped: &[ryo_values::domain::runs::types::Skipped],
    inapplicable: &[ryo_execution_domain::domain::coverage::types::InapplicableFiles],
    refused: &[ryo_execution_domain::domain::coverage::types::RefusedFiles],
    frames: &[ryo_execution_domain::domain::coverage::types::FileObservationReadiness],
    first_bucket: &[ryo_values::domain::stored_facts::types::Bucket],
    selected: &[RuleSelection],
) -> Result<(), OperatingError> {
    use crate::application::file_dag::program::facade_runtime;
    use ryo_values::domain::stored_facts::types::Bucket;

    // One set of inapplicable files and of refused files per rule, or none for the whole run.
    let rules = program.program().rules().len();
    if (!inapplicable.is_empty() && inapplicable.len() != rules)
        || (!refused.is_empty() && refused.len() != rules)
    {
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
        let consumers = &metadata
            .outputs
            .get(root.producer.0)
            .and_then(|outputs| outputs.get(root.output.0 as usize))
            .ok_or(OperatingError::NotRun)?
            .consumers;
        let mut needed = RuleSelection::Unselected;
        for consumer in consumers {
            if *selected.get(consumer.0).ok_or(OperatingError::NotRun)? == RuleSelection::Selected {
                needed = RuleSelection::Selected;
                break;
            }
        }
        if needed == RuleSelection::Unselected {
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
                refused
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
            Ok(prepared) => {
                publications.install_frontier(FrontierSlot(slot), prepared, &readiness)?
            }
            Err(OperatingError::ObservationIncomplete { .. }) => (),
            Err(error) => return Err(error.clone()),
        }
    }
    Ok(())
}
