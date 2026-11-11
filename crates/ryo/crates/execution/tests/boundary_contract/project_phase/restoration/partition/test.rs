//! Actual two-file named restoration and absence publication, including a
//! downstream projection that would erase the completeness key if fused early.
use super::fixtures::Mode;
use super::fixtures::authored;
use crate::boundary_fixture::fixtures::Observed as _;
use crate::boundary_fixture::fixtures::{corpus, engine, source};
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityReason;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileObservationReadiness;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::Partial;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::{
    BatchNumber, BatchOccurrence, ObservationBatches, OccurrenceNumber, PhasePartials,
};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectObservation;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::CapturedFacts;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{FileId, Relation, Value};
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::records::types::ProducerReadiness;
use ryo_values::domain::rows::types::RowCount;
use ryo_values::domain::runs::types::SkipReason;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::Bucket;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) fn exercise(empty: bool, project_away_file: bool, stale_complete: bool, mode: Mode) {
    exercise_with(empty, project_away_file, stale_complete, mode, &[]);
}
pub(super) fn exercise_with(
    empty: bool,
    project_away_file: bool,
    stale_complete: bool,
    mode: Mode,
    inapplicable: &[FileId],
) {
    let engine = engine();
    let authored = authored(project_away_file, mode);
    let producer_id = authored.rules[0].id.clone();
    let prepared = engine.prepare(authored).expect("real mixed named program");
    let mut corpus = corpus();
    let mut second = corpus[0].clone();
    let CorpusInput::Available(entry) = &mut second else {
        unreachable!()
    };
    entry.path = FilePath("second.py".into());
    corpus.push(second);
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut table = source();
    if empty {
        table.cells.clear();
        table.count = RowCount(0);
    }
    let read = |_| Ok(SourceRows::Computed(&table));
    let mut captures = vec![];
    let mut readiness = vec![];
    for file_id in [FileId(0), FileId(1)] {
        let mut symbols = Symbols::default();
        let intents = [
            if file_id == FileId(0) {
                FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Disabled,
                    observations: ObservationDemand::Required,
                    capture: CaptureIntent::Observations,
                }
            } else {
                FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected)
            },
            FileRuleIntent::Undemanded,
            FileRuleIntent::Undemanded,
        ];
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: ResidentStability::Stable,
                    file: file_id,
                    evaluation: &[],
                    publication: &[],
                    intents: &intents,
                    source: ryo_values::domain::files::types::SourceView(b"abc"),
                    elements: &elements,
                    read: &read,
                    semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
                },
                &mut symbols,
            )
            .unwrap();
        if file_id == FileId(0) {
            let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
            let capture = file
                .capture_frontiers(
                    RuleIndex(0),
                    Bucket(0),
                    &mut symbols,
                    CaptureContext {
                        worker: WorkerNumber(0),
                        arena: ryo_execution::inbound::use_cases::execution::interface::Arena(
                            &mut arena,
                        ),
                        places: &mut places,
                        words: &mut words,
                    },
                )
                .unwrap()
                .pop()
                .unwrap();
            file.commit_frontier(RuleIndex(0), capture.input).unwrap();
            captures.push((file_id, capture.outcome.unwrap()));
        }
        readiness.push(FileObservationReadiness {
            file: file_id,
            root: file
                .finish()
                .unwrap()
                .readiness
                .observed("the finished file observed its readiness"),
        });
    }
    if stale_complete {
        // Model persisted Complete bits for a now rejected file. Only current
        // physical provenance can place the uncertainty; stale bits cannot erase it.
        readiness[1].root = Arc::clone(&readiness[0].root);
    }
    let skipped = vec![Skipped {
        rule: producer_id,
        path: FilePath("second.py".into()),
        reason: SkipReason::SyntaxRejected,
    }];
    let stored = CapturedFacts {
        captures,
        reads: AtomicUsize::new(0),
    };
    let project_facts = if matches!(mode, Mode::CurrentRekey { .. }) {
        ProjectFacts::empty(Relation::ModuleIdentity)
    } else {
        ProjectFacts::new(Relation::ModuleIdentity)
    };
    let project_symbols = Symbols::default();
    let publication_partials = if matches!(mode, Mode::Current | Mode::CurrentRekey { .. }) {
        use ryo_execution::inbound::use_cases::metadata::interface::{FilePartial, Lacking};
        Partials {
            entries: vec![FilePartial {
                file: ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::File(
                    FileId(1),
                ),
                partial: Partial {
                    relation: Relation::ModuleIdentity,
                    cause: ryo_values::domain::stored_facts::types::PartialCause(
                        "missing project partition".into(),
                    ),
                    lacking: Lacking::Any,
                    at: ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::File(
                        FileId(1),
                    ),
                },
            }],
            unavailable_files: vec![],
        }
    } else {
        Partials::default()
    };
    let consumer_inapplicable = [
        ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(),
        ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(),
        ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles(
            inapplicable.to_vec(),
        ),
    ];
    let refusals = vec![
        ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default();
        consumer_inapplicable.len()
    ];
    let first = [Bucket(0), Bucket(10), Bucket(20)];
    // The producer is restored, not selected; rule 1 publishes in the first batch, and rule 2 reads its population in the second.
    // `forge` relabels rule 1's publication Complete before the stage installs it.
    let run = |forge: bool, project: &ProjectFacts| {
        let mut arenas = vec![];
        let stage = request.observations(&mut arenas).unwrap();
        let mut symbols = Symbols::default();
        crate::project_phase::source_facts::fixtures::run_observations_with(
            stage,
            ObservationBatches {
                occurrences: &[
                    BatchOccurrence {
                        batch: BatchNumber(0),
                        rule: RuleIndex(1),
                    },
                    BatchOccurrence {
                        batch: BatchNumber(1),
                        rule: RuleIndex(2),
                    },
                ],
                readiness: &readiness,
                rules: &[RuleIndex(0), RuleIndex(1), RuleIndex(2)],
                first: &first,
                corpus: &corpus,
                covered: &[],
                skipped: &skipped,
                inapplicable: &consumer_inapplicable,
                refused: &refusals,
                facts: &stored,
                project,
                project_symbols: &project_symbols,
            },
            &|number| PhasePartials {
                evaluation: Partials::default(),
                publication: if number == OccurrenceNumber(0) {
                    publication_partials.clone()
                } else {
                    Partials::default()
                },
            },
            &|_, install| {
                install.map(|mut install| {
                    if forge {
                        for root in &mut install.transfer.roots {
                            root.readiness = ProducerReadiness::Complete;
                        }
                    }
                    install
                })
            },
            &mut symbols,
        )
    };
    if matches!(mode, Mode::Current | Mode::CurrentRekey { .. }) {
        let forged = run(true, &project_facts);
        assert!(
            forged.0.is_err(),
            "a Partial owner proof cannot be relabeled Complete"
        );
    }
    let project_facts = if matches!(mode, Mode::CurrentRekey { .. }) {
        ProjectFacts::empty(Relation::ModuleIdentity)
    } else {
        ProjectFacts::new(Relation::ModuleIdentity)
    };
    let (ran, delivered) = run(false, &project_facts);
    ran.unwrap();
    let mut delivered = delivered.into_iter().map(|(_, outcome)| outcome.unwrap());
    let mut outcome = delivered.next().unwrap();
    let ProjectOutputOutcome {
        outcome: observed, ..
    } = outcome.observations.pop().unwrap();
    match (mode, observed) {
        (Mode::Current | Mode::CurrentRekey { .. }, ProjectObservation::Partial { table, .. }) => {
            if matches!(mode, Mode::CurrentRekey { .. }) {
                assert_eq!(
                    table.count,
                    RowCount(0),
                    "the actual incomplete project source has no known row"
                );
            } else {
                assert_eq!(
                    table.cells,
                    vec![Value::File(FileId(0)), Value::Number(Integer(7))]
                );
            }
        }
        (_, ProjectObservation::Complete { table, .. }) => {
            assert_eq!(table.count, RowCount(0));
        }
        _ => panic!("the actual project population keeps its exact completeness"),
    }
    let outcomes = delivered.collect::<Vec<_>>();
    assert_eq!(outcomes.len(), 1);
    for outcome in outcomes {
        let RootOutcome::Partial { table, affected } = outcome.emit else {
            panic!("only the bad physical partition must remain explicitly unknown")
        };
        if matches!(mode, Mode::CurrentRekey { file_key: false }) {
            assert!(
                affected.contains(&None),
                "late physical tagging must not narrow re-keyed uncertainty back to file 1"
            );
        } else {
            assert_eq!(
                affected,
                if matches!(mode, Mode::Rekey { file_key: false }) {
                    vec![None]
                } else {
                    vec![Some(FileId(1))]
                }
            );
        }
        let values = match mode {
            Mode::PartialLeft { .. } => vec![7],
            Mode::Rekey { file_key: true } => vec![7],
            Mode::Rekey { file_key: false } => vec![],
            Mode::Current => vec![8, 8],
            Mode::CurrentRekey { file_key: false } => vec![],
            Mode::CurrentRekey { file_key: true } => vec![7],
            Mode::PartialRight if empty => vec![20, 20, 30],
            Mode::PartialRight => vec![],
        };
        let expected = values
            .iter()
            .flat_map(|value| {
                if project_away_file {
                    vec![Value::Number(Integer(*value))]
                } else {
                    vec![Value::File(FileId(0)), Value::Number(Integer(*value))]
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            table.cells, expected,
            "known rows preserve their multiplicity and order for {mode:?}"
        );
        assert_eq!(table.count, RowCount(values.len()));
    }
    assert_eq!(
        project_facts.reads.load(Ordering::SeqCst),
        usize::from(matches!(mode, Mode::Current | Mode::CurrentRekey { .. })),
        "the consumer's population reads the provider once"
    );
    request.finish();
}
