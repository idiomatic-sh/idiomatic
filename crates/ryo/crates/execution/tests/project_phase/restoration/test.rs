//! Named restoration retains partition-qualified physical coverage despite cached readiness.
use crate::boundary_fixture::Observed as _;
use ryo_execution::inbound::use_cases::execution::interface::FileObservationReadiness;
use ryo_execution_runtime::application::file_dag::observations::types::RuleSelection;
use ryo_execution_runtime::application::file_dag::frontiers::types::InputReadiness;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use super::super::*;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::{
    files::types::CorpusInput,
    runs::types::{RuleApplicability, SkipReason, Skipped},
};

#[test]
fn restored_named_frontier_keeps_skipped_partition_provenance_despite_complete_readiness() {
    use ryo_execution::inbound::use_cases::execution::interface::ObservationExecution;
    use ryo_execution::inbound::use_cases::execution::interface::ObservationPublication;
    use ryo_values::domain::plan_parts::types::ObservationBindingId;
    for rejected in [false, true] {
        let engine = engine();
        let mut authored = crate::file_intents::named_request();
        authored.rules[0].applicability = RuleApplicability::AcceptedSyntax;
        let producer_id = authored.rules[0].id.clone();
        let prepared = engine.prepare(authored).unwrap();
        let mut corpus = corpus();
        let mut second = corpus[0].clone();
        let CorpusInput::Available(entry) = &mut second else {
            unreachable!()
        };
        entry.path = FilePath("second.py".into());
        corpus.push(second);
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
        let elements = ryo_language::domain::elements::types::Elements::default();
        let mut captures = vec![];
        let mut readiness = vec![];
        for file_id in [FileId(0), FileId(1)] {
            let mut table = source();
            for cell in &mut table.cells {
                match cell {
                    Value::File(actual) => *actual = file_id,
                    Value::Node(actual) => {
                        *actual = ryo_relations::domain::relations::fixtures::node_id(
                            file_id,
                            ryo_relations::domain::relations::types::Position(2),
                        )
                    }
                    _ => {}
                }
            }
            let read = |_| Ok(SourceRows::Computed(&table));
            let mut symbols = Symbols::default();
            let mut file = request
                .open_file(
                    FileInput {
                        language: SourceLanguage::Recognized,
                        resident_stability: ResidentStability::Stable,
                        file: file_id,
                        evaluation: &[],
                        publication: &[],
                        intents: &[
                            FileRuleIntent::Run {
                                diagnostic: DiagnosticDelivery::Disabled,
                                observations: ObservationDemand::Required,
                                capture: CaptureIntent::Observations,
                            },
                            FileRuleIntent::Undemanded,
                        ],
                        source: ryo_values::domain::files::types::SourceView(b"abc"),
                        elements: &elements,
                        read: &read,
                        semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                        kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
                    },
                    &mut symbols,
                )
                .unwrap();
            let mut arena = vec![];
            let mut places = vec![];
            let mut words = Words::default();
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
            let root = file.finish().unwrap().readiness.observed("the finished file observed its readiness");
            assert!(root.complete.0.iter().any(|bits| *bits != 0));
            readiness.push(FileObservationReadiness {
                file: file_id,
                root,
            });
        }
        let skipped = if rejected {
            // A persisted Complete readiness claim cannot override current syntax rejection.
            captures.pop();
            vec![Skipped {
                rule: producer_id,
                path: FilePath("second.py".into()),
                reason: SkipReason::SyntaxRejected,
            }]
        } else {
            vec![]
        };
        let stored = CapturedFacts {
            captures,
            reads: AtomicUsize::new(0),
        };
        let mut arenas = vec![];
        let mut stage = request.observations(&mut arenas).unwrap();
        stage
            .restore_frontiers(
                &stored,
                &corpus,
                &[],
                &skipped,
                &[],
                &readiness,
                &[Bucket(0), Bucket(10)],
                &[RuleSelection::Unselected, RuleSelection::Selected],
            )
            .unwrap();
        assert_eq!(
            stored.reads.load(Ordering::SeqCst),
            1,
            "actual restoration validates the packed frontier"
        );
        assert_eq!(
            stage
                .binding_readiness(RuleIndex(1))
                .unwrap()
                .get(&ObservationBindingId(0)),
            Some(&InputReadiness::Ready)
        );
    }
}
