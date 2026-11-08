//! Actual two-file named restoration and absence publication, including a
//! downstream projection that would erase the completeness key if fused early.
use crate::boundary_fixture::Observed as _;
use ryo_execution::inbound::use_cases::execution::interface::ObservationExecution as _;
use ryo_execution::inbound::use_cases::execution::interface::ObservationPublication as _;
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityReason;
use ryo_execution::inbound::use_cases::execution::interface::FileObservationReadiness;
use ryo_execution::inbound::use_cases::execution::interface::{InapplicableFiles, RefusedFiles};
use ryo_execution::inbound::use_cases::execution::interface::Partial;
use ryo_execution_runtime::application::file_dag::observations::types::AdmittedOutput;
use ryo_execution_runtime::application::file_dag::observations::types::RuleSelection;
use ryo_execution_runtime::application::file_dag::frontiers::types::InputReadiness;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use super::super::super::*;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::plan_parts::types::ObservationBindingId;
use ryo_values::domain::records::types::CrossPublicationTransfer;
use ryo_values::domain::records::types::PackedObservedRoot;
use ryo_values::domain::records::types::ProducerReadiness;
use ryo_values::domain::runs::types::SkipReason;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use ryo_values::domain::stored_facts::types::KeptWords;
use ryo_values::domain::table_words::fixtures::encode_table;
use ryo_values::domain::table_words::fixtures::pack;
use ryo_values::domain::table_words::types::WordSlice;
use super::fixtures::Mode;
use super::fixtures::authored;

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
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
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
                        arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
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
            root: file.finish().unwrap().readiness.observed("the finished file observed its readiness"),
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
                file: ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::File(FileId(1)),
                partial: Partial {
                    relation: Relation::ModuleIdentity,
                    cause: ryo_values::domain::stored_facts::types::PartialCause(
                        "missing project partition".into(),
                    ),
                    lacking: Lacking::Any,
                    at: ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::File(FileId(1)),
                },
            }],
            unavailable_files: vec![],
        }
    } else {
        Partials::default()
    };
    let partials = Partials::default();
    let mut arenas = vec![];
    let mut stage = request.observations(&mut arenas).unwrap();
    stage.finish(RuleIndex(0)).unwrap();
    stage
        .restore_frontiers(
            &stored,
            &corpus,
            &[],
            &skipped,
            &[],
            &[],
            &readiness,
            &[Bucket(0), Bucket(10), Bucket(20)],
            &[RuleSelection::Unselected, RuleSelection::Selected, RuleSelection::Selected],
        )
        .unwrap();
    let mut symbols = Symbols::default();
    let slot = stage.reserve(RuleIndex(1)).unwrap();
    let first = [Bucket(0), Bucket(10), Bucket(20)];
    let mut outcome = crate::boundary_fixture::project_root(
        &stage,
        PhaseOccurrences {
            occurrences: &[PhaseOccurrence {
                slot,
                ..crate::boundary_fixture::ordinary_occurrence(RuleIndex(1))
            }],
            first: &first,
            corpus: &corpus,
            covered: &[],
            skipped: &skipped,
            inapplicable: &vec![ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(); 3],
            facts: &stored,
            project: &project_facts,
            project_symbols: &project_symbols,
            arenas: stage.arenas(),
        },
        &partials,
        &publication_partials,
        &mut symbols,
    )
    .unwrap();
    let ProjectOutputOutcome {
        output,
        outcome: observed,
    } = outcome.observations.pop().unwrap();
    let (table, publication, readiness) = match (mode, observed) {
        (
            Mode::Current | Mode::CurrentRekey { .. },
            ProjectObservation::Partial { table, publication },
        ) => {
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
            (
                table,
                publication,
                ProducerReadiness::Unknown(ryo_values::domain::records::types::ObservationReason(
                    "physical source facts are incomplete".into(),
                )),
            )
        }
        (_, ProjectObservation::Complete { table, publication }) => {
            assert_eq!(table.count, RowCount(0));
            (table, publication, ProducerReadiness::Complete)
        }
        _ => panic!("the actual project population keeps its exact completeness"),
    };
    let PublicationSlot::Reserved(worker) = slot else {
        panic!("demanded publication arena")
    };
    let mut words = Words::default();
    encode_table(&table, &[], &mut words);
    let transfer = CrossPublicationTransfer {
        worker,
        arena: ArenaBytes::default(),
        roots: vec![PackedObservedRoot {
            output,
            words: ryo_values::domain::records::types::RootWords::Published(KeptWords(pack(
                WordSlice(&words.0),
            ))),
            readiness,
            arenas: vec![],
        }],
    };
    if matches!(mode, Mode::Current | Mode::CurrentRekey { .. }) {
        let mut forged = transfer.clone();
        forged.roots[0].readiness = ProducerReadiness::Complete;
        assert!(
            stage
                .install(
                    RuleIndex(1),
                    slot,
                    forged,
                    vec![AdmittedOutput { output, publication: publication.clone() }]
                )
                .is_err(),
            "a Partial owner proof cannot be relabeled Complete"
        );
    }
    stage
        .install(RuleIndex(1), slot, transfer, vec![AdmittedOutput { output, publication }])
        .unwrap();
    stage.finish(RuleIndex(1)).unwrap();
    let bound = stage.bindings(RuleIndex(2)).unwrap();
    assert_eq!(
        stage
            .binding_readiness(RuleIndex(2))
            .unwrap()
            .get(&ObservationBindingId(0)),
        Some(&InputReadiness::Ready)
    );
    // One acquisition of the frontier sources serves both declared occurrences of the one population.
    let consumer = PhaseOccurrence {
        bound: &bound,
        ..crate::boundary_fixture::ordinary_occurrence(RuleIndex(2))
    };
    let consumer_inapplicable = [
        ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(),
        ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(),
        ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles(inapplicable.to_vec()),
    ];
    let outcomes = crate::boundary_fixture::project_roots(
        &stage,
        PhaseOccurrences {
            occurrences: &[consumer, consumer],
            first: &first,
            corpus: &corpus,
            covered: &[],
            skipped: &skipped,
            inapplicable: &consumer_inapplicable,
            facts: &stored,
            project: &project_facts,
            project_symbols: &project_symbols,
            arenas: stage.arenas(),
        },
        &partials,
        &partials,
        &mut symbols,
    );
    for outcome in outcomes {
        let RootOutcome::Partial { table, affected } = outcome.unwrap().emit else {
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
        "two declared roots of one population read the provider once"
    );
    drop(bound);
    stage.finish(RuleIndex(2)).unwrap();
    stage.finish_stage().unwrap();
    request.finish();
}
