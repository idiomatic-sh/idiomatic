//! Injected immutable source rows and the exact file frontier captured by the public owner.
use ryo_execution_domain::domain::frontier_capture::types::CapturedFrontier;
use ryo_execution_runtime::outbound::facts::errors::FactsError;
use ryo_execution_runtime::outbound::facts::interface::FactsFinish;
use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
use ryo_execution_runtime::outbound::facts::types::BucketSource;
use ryo_execution_runtime::outbound::facts::types::ColumnsMut;
use ryo_execution_runtime::outbound::facts::types::KeptBucket;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution_runtime::outbound::facts::types::WordsToKeep;
use ryo_relations::domain::relations::{
    fixtures::kinds,
    types::{CellKind, FileId, Offset, Relation, Span, Value},
};
use ryo_relations::domain::relations::{
    fixtures::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::stored_facts::types::{Bucket, KeptWords, PackedWords};
use ryo_execution::inbound::use_cases::execution::interface::{
    AdmittedOutput, Install, ObservationBatches, ObservationExecution, OccurrenceNumber,
    OperatingError, Partials, PhasePartials, PhaseStop, ProjectOutputOutcome, RuleOutcome,
};
use ryo_execution_runtime::application::file_dag::program::observation_stage::types::ObservationStage;
use ryo_language::domain::symbols::types::Symbols;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(in super::super) struct ProjectFacts {
    relation: Relation,
    columns: Vec<FactColumn>,
    pub reads: AtomicUsize,
}
impl ProjectFacts {
    /// A real declared schema with no observed project rows.
    pub fn empty(relation: Relation) -> Self {
        let mut facts = Self::new(relation);
        for column in &mut facts.columns {
            column.0.clear();
        }
        facts
    }
    pub fn new(relation: Relation) -> Self {
        let columns = kinds(relation)
            .iter()
            .map(|kind| {
                let value = match kind {
                    CellKind::File => Value::File(FileId(0)),
                    CellKind::Symbol => {
                        Value::Symbol(ryo_relations::domain::terms::types::Symbol(0))
                    }
                    CellKind::Span => Value::Span(Span {
                        start: Offset(0),
                        end: Offset(0),
                    }),
                    _ => panic!("project fixture follows its actual catalog schema"),
                };
                FactColumn(vec![to_bits(value).0])
            })
            .collect();
        Self {
            relation,
            columns,
            reads: AtomicUsize::new(0),
        }
    }
}
fn unsupported() -> FactsError {
    FactsError::Store {
        reason: "immutable project fixture does not write facts".into(),
    }
}
impl FactsWorker for ProjectFacts {
    fn resident_stability(&self) -> ResidentStability {
        ResidentStability::Stable
    }
    fn rows(&self, relation: Relation) -> Result<ColumnsView<'_>, FactsError> {
        assert_eq!(relation, self.relation);
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(ColumnsView {
            kinds: kinds(relation),
            columns: &self.columns,
        })
    }
    fn columns_mut(&mut self) -> Result<ColumnsMut<'_>, FactsError> {
        Err(unsupported())
    }
    fn keep(&self, _: Bucket, _: WordsToKeep<'_>) -> Result<(), FactsError> {
        Err(unsupported())
    }
    fn release(&mut self) -> Result<(), FactsError> {
        Err(unsupported())
    }
    fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError> {
        Err(unsupported())
    }
}

pub(in super::super) struct CapturedFacts {
    pub captures: Vec<(FileId, CapturedFrontier)>,
    pub reads: AtomicUsize,
}
impl FactsFinish for CapturedFacts {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        let mut words = vec![];
        let mut sources = vec![];
        for (file, capture) in &self.captures {
            assert_eq!(bucket, capture.bucket);
            sources.push(BucketSource {
                file: *file,
                start: ryo_execution_runtime::outbound::facts::types::KeptWordsStart(words.len()),
                length: ryo_execution_runtime::outbound::facts::types::KeptWordsLength(
                    capture.words.0.len(),
                ),
                proof: capture.proof.clone(),
            });
            words.extend_from_slice(&capture.words.0);
        }
        Ok(KeptBucket {
            words: std::sync::Arc::new(KeptWords(PackedWords(words))),
            sources,
        })
    }
}

/// What one declared occurrence of an observation stage was delivered: its outcome, or why it has none.
pub(in super::super) type Delivered = (
    OccurrenceNumber,
    Result<RuleOutcome<ProjectOutputOutcome>, PhaseStop<OperatingError>>,
);

/// Every declared occurrence of `batches` through the stage's one call, each evaluated over the same `partials`. An occurrence
/// whose slot reserved a worker publishes each complete or partial output it observed, packed as a caller packs it; every
/// occurrence is delivered in declaration order. Answers what the stage answered and each delivered occurrence.
pub(in super::super) fn run_observations(
    stage: ObservationStage<'_>,
    batches: ObservationBatches<'_>,
    partials: &Partials,
    symbols: &mut Symbols,
) -> (Result<(), OperatingError>, Vec<Delivered>) {
    run_observations_with(
        stage,
        batches,
        &|_| PhasePartials {
            evaluation: partials.clone(),
            publication: partials.clone(),
        },
        &|_, install| install,
        symbols,
    )
}

/// [`run_observations`] with each occurrence's own partials, and what each occurrence hands the stage to install edited by
/// `install` first, for a control that forges or withholds a publication.
pub(in super::super) fn run_observations_with(
    stage: ObservationStage<'_>,
    batches: ObservationBatches<'_>,
    partials: &dyn Fn(OccurrenceNumber) -> PhasePartials,
    install: &dyn Fn(OccurrenceNumber, Option<Install>) -> Option<Install>,
    symbols: &mut Symbols,
) -> (Result<(), OperatingError>, Vec<Delivered>) {
    let mut delivered = vec![];
    let ran = stage.run_batches(
        batches,
        symbols,
        |_| Ok::<(), OperatingError>(()),
        |number| Ok(partials(number)),
        |_| Ok(()),
        |_, outcome, arenas, worker, symbols: &mut Symbols| {
            let packed = match worker {
                Some(worker) => published(&outcome, arenas, worker, symbols)?,
                None => None,
            };
            Ok((outcome, packed))
        },
        |number, result| match result {
            Ok((outcome, packed)) => {
                delivered.push((number, Ok(outcome)));
                Ok(install(number, packed))
            }
            Err(stop) => {
                delivered.push((number, Err(stop)));
                Ok(None)
            }
        },
    );
    (ran, delivered)
}

/// The publication a caller hands the stage for one evaluated occurrence: each complete or partial output packed into the arena of
/// `worker`, as the caller's own completion packs it. An occurrence that observed no output publishes nothing.
pub(in super::super) fn published(
    outcome: &RuleOutcome<ProjectOutputOutcome>,
    arenas: &[ryo_values::domain::stored_facts::types::ArenaBytes],
    worker: ryo_relations::domain::relations::types::WorkerNumber,
    symbols: &mut Symbols,
) -> Result<Option<Install>, OperatingError> {
    use ryo_execution::inbound::use_cases::execution::interface::ProjectObservation;
    use ryo_relations::domain::relations::types::Origin;
    use ryo_values::domain::records::types::{
        CrossPublicationTransfer, ObservationReason, PackedObservedRoot, ProducerReadiness,
        RootWords,
    };
    use ryo_values::domain::stored_facts::types::ArenaBytes;
    use ryo_values::domain::table_words::{
        fixtures::{encode_table, pack},
        types::WordSlice,
    };
    let mut arena = ArenaBytes::default();
    let mut roots = vec![];
    let mut publications = vec![];
    for each in &outcome.observations {
        let (table, publication, readiness) = match &each.outcome {
            ProjectObservation::Complete { table, publication } => {
                (table, publication, ProducerReadiness::Complete)
            }
            ProjectObservation::Partial { table, publication } => (
                table,
                publication,
                ProducerReadiness::Unknown(ObservationReason(
                    "physical source facts are incomplete".into(),
                )),
            ),
            ProjectObservation::Other(_) => continue,
        };
        let mut table = table.clone();
        let mut places = vec![];
        let mut words = ryo_relations::domain::relations::types::Words::default();
        ryo_execution::application::publication::service::publication(
            ryo_execution::application::publication::types::PublicationInput {
                table: &mut table,
                symbols: &mut *symbols,
                inherited: arenas,
                worker,
                arena: &mut arena,
                places: &mut places,
            },
        )
        .expect("an observed table packs into its worker's arena");
        // The arenas the packed names live in, as the caller's own packing records them.
        let mut named = vec![];
        for place in &places {
            if let Origin::Derived(origin) = place.origin
                && !named.contains(&origin)
            {
                named.push(origin);
            }
        }
        encode_table(&table, &places, &mut words);
        roots.push(PackedObservedRoot {
            output: each.output,
            words: RootWords::Published(KeptWords(pack(WordSlice(&words.0)))),
            readiness,
            arenas: named,
        });
        publications.push(AdmittedOutput {
            output: each.output,
            publication: publication.clone(),
        });
    }
    if publications.is_empty() {
        return Ok(None);
    }
    Ok(Some(Install {
        transfer: CrossPublicationTransfer {
            worker,
            arena,
            roots,
        },
        publications,
    }))
}
