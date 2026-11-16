//! The named outputs the files kept, restored for the selected consumers: each physical bucket read and proven once, each
//! logical output checked against the readiness the files recorded.

use crate::application::file_dag::frontiers::service::validate_bucket;
use crate::application::file_dag::program::admission::types::RequiredPartitions;
use crate::application::file_dag::program::preparation::types::{
    ObservationMetadata, OutputRoot, PreparedProgram,
};
use crate::application::file_dag::program::request_execution::types::RequestExecution;
use crate::application::file_dag::publications::constants::RESTORED_FRAME_REJECTED;
use crate::application::file_dag::publications::service::{incomplete, output_place, worker_place};
use crate::application::file_dag::publications::types::{
    CrossRoot, FileFrontiers, FrontierCompatibility, FrontierPopulation, FrontierSlot,
    Publications, RestoredOutput, RuleSelection,
};
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::types::KeptBucket;
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::coverage::types::{FileObservationReadiness, RuleIndex};
use ryo_execution_domain::domain::executable_program::service::{layout, rule as rule_nodes};
use ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::service::places_mut;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::kinds as relation_kinds;
use ryo_relations::domain::relations::types::{CROSS_FILE, CellKind, FileId, Origin, WorkerNumber};
use ryo_values::domain::plan_parts::types::{Input, InputNumber};
use ryo_values::domain::records::types::ProducerReadiness;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::table_words::service::decode_complete;
use std::collections::BTreeMap;
use std::sync::Arc;

impl Publications<'_> {
    /// Restores, for the selected consumers, each named output the files kept: each physical bucket read and proven once, each
    /// logical output checked against the readiness the files recorded.
    ///
    /// # Errors
    ///
    /// A refusal other than an incomplete observation: an unreadable program, a corpus the program cannot number.
    pub(crate) fn restore(
        &mut self,
        execution: &RequestExecution,
        facts: &dyn FactsFinish,
        scoping: crate::application::file_dag::frontiers::types::PhaseScoping<'_>,
        frames: &[FileObservationReadiness],
        first_bucket: &[ryo_values::domain::stored_facts::types::Bucket],
        selected: &[RuleSelection],
    ) -> Result<(), OperatingError> {
        let program: &PreparedProgram = &execution.program;
        let executable = &execution.pass.program;
        let rules = program.rule_count().0;
        if (!scoping.inapplicable.is_empty() && scoping.inapplicable.len() != rules)
            || (!scoping.refused.is_empty() && scoping.refused.len() != rules)
        {
            return Err(OperatingError::NotRun);
        }
        let metadata = program
            .observation_metadata()
            .ok_or(OperatingError::NotRun)?;
        let files = (0..scoping.corpus.len())
            .map(|file| {
                u32::try_from(file)
                    .map(FileId)
                    .map_err(|_| OperatingError::NotRun)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let readiness = match frontier_readiness(metadata, &files, frames) {
            Ok(readiness) => readiness,
            Err(OperatingError::ObservationIncomplete { .. }) => return Ok(()),
            Err(error) => return Err(error),
        };
        let mut physical = BTreeMap::<
            ryo_values::domain::stored_facts::types::Bucket,
            Result<RestoredOutput, OperatingError>,
        >::new();
        for (slot, root) in metadata.frontiers.iter().enumerate() {
            let consumers = &metadata
                .outputs
                .get(root.producer.0)
                .and_then(|outputs| outputs.get(output_place(root.output).0))
                .ok_or(OperatingError::NotRun)?
                .consumers;
            let mut needed = RuleSelection::Unselected;
            for consumer in consumers {
                if *selected.get(consumer.0).ok_or(OperatingError::NotRun)?
                    == RuleSelection::Selected
                {
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
                let restored = (|| {
                    let required =
                        crate::application::file_dag::frontiers::service::required_files(
                            program,
                            execution,
                            root.producer,
                            scoping,
                        )?;
                    let payload = facts.kept(bucket).map_err(|error| {
                        OperatingError::ObservationIncomplete {
                            reason: error.to_string(),
                        }
                    })?;
                    let coverage = restored_coverage(
                        program,
                        root.producer,
                        root.frontier,
                        first,
                        bucket,
                        &required,
                        &payload,
                    )?;
                    let kinds = frontier_kinds(program, executable, root.producer, root.frontier)?;
                    let arenas = self.arenas_of(&payload.words)?;
                    Ok(RestoredOutput {
                        words: payload.words,
                        kinds,
                        coverage,
                        arenas,
                        decoded: std::cell::OnceCell::new(),
                    })
                })();
                entry.insert(restored);
            }
            match physical.get(&bucket).ok_or(OperatingError::NotRun)? {
                Ok(restored) => self.install_restored(FrontierSlot(slot), restored, &readiness)?,
                Err(OperatingError::ObservationIncomplete { .. }) => (),
                Err(error) => return Err(error.clone()),
            }
        }
        Ok(())
    }

    /// The inherited arenas the texts of `words` are in, each once: a text of any other arena is a malformed frontier.
    fn arenas_of(
        &self,
        words: &ryo_values::domain::stored_facts::types::KeptWords,
    ) -> Result<Vec<WorkerNumber>, OperatingError> {
        let mut symbols = Symbols::default();
        decode_complete(&words.0, places_mut(&mut symbols)).map_err(|error| {
            OperatingError::ObservationIncomplete {
                reason: error.to_string(),
            }
        })?;
        let mut arenas = Vec::new();
        for place in ryo_language::domain::names::service::places_of(&symbols) {
            if let Origin::Derived(worker) = place.origin {
                if worker == CROSS_FILE || worker_place(worker).0 >= self.inherited.0 {
                    return Err(incomplete());
                }
                if !arenas.contains(&worker) {
                    arenas.push(worker);
                }
            }
        }
        Ok(arenas)
    }

    /// Installs one restored output in its slot: each logical output keeps its own readiness even when names share one bucket.
    fn install_restored(
        &mut self,
        slot: FrontierSlot,
        restored: &RestoredOutput,
        readiness: &FrontierPopulation<'_>,
    ) -> Result<(), OperatingError> {
        if frontier_compatible(slot, readiness, &restored.coverage)?
            == FrontierCompatibility::Incompatible
        {
            return Err(incomplete());
        }
        let entry = self.metadata.frontiers.get(slot.0).ok_or_else(incomplete)?;
        if !matches!(
            self.metadata
                .outputs
                .get(entry.producer.0)
                .and_then(|outputs| outputs.get(output_place(entry.output).0))
                .map(|output| &output.root),
            Some(OutputRoot::PerFile)
        ) {
            return Err(incomplete());
        }
        let root = self
            .roots
            .get_mut(entry.producer.0)
            .and_then(|outputs| outputs.get_mut(output_place(entry.output).0))
            .ok_or_else(incomplete)?;
        if !matches!(root, CrossRoot::Unavailable) {
            return Err(incomplete());
        }
        for arena in &restored.arenas {
            let count = self
                .references
                .get_mut(worker_place(*arena).0)
                .ok_or_else(incomplete)?;
            count.0 = count.0.checked_add(1).ok_or_else(incomplete)?;
        }
        *root = CrossRoot::Restored(RestoredOutput {
            words: Arc::clone(&restored.words),
            kinds: restored.kinds.clone(),
            coverage: restored.coverage.clone(),
            arenas: restored.arenas.clone(),
            decoded: std::cell::OnceCell::new(),
        });
        Ok(())
    }
}

/// Validates the file-to-frame join once per invocation, before any producer's rows are read: it never infers completion
/// from a row count or from a file with no frame.
///
/// # Errors
///
/// An incomplete observation when a file has no frame, two, or one of another layout.
pub(crate) fn frontier_readiness<'f>(
    metadata: &ObservationMetadata,
    files: &[FileId],
    frames: &'f [FileObservationReadiness],
) -> Result<FrontierPopulation<'f>, OperatingError> {
    use ryo_values::domain::observations::service::validate_frontiers;
    use ryo_values::domain::records::types::ObservationReadinessCount;
    let count = ObservationReadinessCount(
        u32::try_from(metadata.frontiers.len()).map_err(|_| incomplete())?,
    );
    let mut indexed = BTreeMap::new();
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

/// Whether a logical unavailability agrees with the independently proven physical coverage.
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
        if frontier_state(*frame, slot).map_err(|_| incomplete())? != ProducerReadiness::Complete
            // Logical unavailability is localized only by independently admitted physical provenance, never by a readiness bit.
            && !matches!(physical, CoverageState::Partial { files, .. }
                if files.contains(&None) || files.contains(&Some(*file)))
            && !matches!(physical, CoverageState::Inapplicable { files } if files.contains(file))
        {
            return Ok(FrontierCompatibility::Incompatible);
        }
    }
    Ok(FrontierCompatibility::Compatible)
}

/// The coverage of a named output the files kept of `producer`'s entry `input`, proven against the files it read: a frame of a
/// file its rule was syntax-rejected on is refused.
///
/// # Errors
///
/// `RESTORED_FRAME_REJECTED`, or a proof that does not fit the program.
pub(crate) fn restored_coverage(
    program: &PreparedProgram,
    producer: RuleIndex,
    input: InputNumber,
    first: Bucket,
    bucket: Bucket,
    required: &RequiredPartitions,
    payload: &KeptBucket,
) -> Result<CoverageState, OperatingError> {
    let rule_id = program.rule_id(producer)?;
    for source in &payload.sources {
        let path = required
            .request
            .path(source.file)
            .ok_or(OperatingError::NotRead)?;
        if required
            .skipped
            .iter()
            .any(|skipped| &skipped.rule == rule_id && &skipped.path == path)
        {
            return Err(OperatingError::ObservationIncomplete {
                reason: RESTORED_FRAME_REJECTED.into(),
            });
        }
    }
    validate_bucket(program, producer, input, first, bucket, required, payload)
}

/// What each column of `producer`'s frontier entry `input` holds: a step's output columns, a relation's.
///
/// # Errors
///
/// `NotRun` for an entry outside the program; `WrongOperation` for an entry the files do not keep.
pub(crate) fn frontier_kinds(
    program: &PreparedProgram,
    executable: &ExecutableProgram,
    producer: RuleIndex,
    input: InputNumber,
) -> Result<Box<[CellKind]>, OperatingError> {
    match program
        .frontiers(producer)?
        .get(input.0)
        .ok_or(OperatingError::NotRun)?
    {
        Input::Step(id) => {
            let node = rule_nodes(executable, producer)?
                .steps
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .ok_or(OperatingError::NotRun)?
                .clone()?;
            Ok(layout(executable, node)?.into())
        }
        Input::Relation(relation) => Ok(relation_kinds(*relation).into()),
        Input::BoundObservation(_) => Err(OperatingError::WrongOperation),
    }
}
