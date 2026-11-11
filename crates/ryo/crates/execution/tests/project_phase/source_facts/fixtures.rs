//! Injected immutable source rows and the exact file frontier captured by the public owner.
use ryo_execution::inbound::use_cases::execution::interface::ObservationStage;
use ryo_execution::inbound::use_cases::execution::interface::{
    AdmittedOutput, Install, ObservationBatches, ObservationExecution, OccurrenceNumber,
    OperatingError, Partials, PhasePartials, PhaseStop, ProjectOutputOutcome, RuleOutcome,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::stored_facts::types::KeptWords;

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
