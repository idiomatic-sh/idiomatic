//! Named restoration retains partition-qualified physical coverage despite cached readiness.
use super::super::*;
use crate::boundary_fixture::Observed as _;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileObservationReadiness;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::outbound::facts::errors::FactsError;
use ryo_execution_runtime::outbound::facts::interface::FactsFinish;
use ryo_execution_runtime::outbound::facts::types::KeptBucket;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::{
    files::types::CorpusInput,
    runs::types::{RuleApplicability, SkipReason, Skipped},
};

#[test]
fn restored_named_frontier_keeps_skipped_partition_provenance_despite_complete_readiness() {
    use ryo_execution::inbound::use_cases::execution::interface::{
        BatchNumber, BatchOccurrence, ObservationBatches,
    };
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
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
        let elements = ryo_language::domain::elements::types::Elements::default();
        let mut captures = vec![];
        let mut consumer_captures = vec![];
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
                    Value::Symbol(_)
                    | Value::Name(_)
                    | Value::Span(_)
                    | Value::Number(_)
                    | Value::Flag(_) => {}
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
                            FileRuleIntent::Run {
                                diagnostic: DiagnosticDelivery::Disabled,
                                observations: ObservationDemand::NotRequired,
                                capture: CaptureIntent::DiagnosticAndObservations,
                            },
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
            // The consumer's own file frontier, kept in its own bucket.
            for consumer in file
                .capture_frontiers(
                    RuleIndex(1),
                    Bucket(10),
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
            {
                file.commit_frontier(RuleIndex(1), consumer.input).unwrap();
                consumer_captures.push((file_id, consumer.outcome.unwrap()));
            }
            let root = file
                .finish()
                .unwrap()
                .readiness
                .observed("the finished file observed its readiness");
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
        let stage = request.observations(&mut arenas).unwrap();
        let covered = [Covered {
            file: FileId(0),
            rules: vec![RuleIndex(1)],
            partial: vec![],
        }];
        let project = ProjectFacts::new(Relation::ModuleIdentity);
        let project_symbols = Symbols::default();
        let mut symbols = Symbols::default();
        // The producer is not selected: the consumer reads it only from the restored frontier.
        let (ran, delivered) = crate::project_phase::source_facts::fixtures::run_observations(
            stage,
            ObservationBatches {
                occurrences: &[BatchOccurrence { batch: BatchNumber(0), rule: RuleIndex(1) }],
                readiness: &readiness,
                rules: &[RuleIndex(0), RuleIndex(1)],
                first: &[Bucket(0), Bucket(10)],
                corpus: &corpus,
                covered: &covered,
                skipped: &skipped,
                inapplicable: &[Default::default(), Default::default()],
                refused: &[Default::default(), Default::default()],
                facts: &ConsumerFrontier {
                    producer: &stored,
                    consumer: &CapturedFacts {
                        captures: consumer_captures,
                        reads: AtomicUsize::new(0),
                    },
                },
                project: &project,
                project_symbols: &project_symbols,
            },
            &Partials::default(),
            &mut symbols,
        );
        assert_eq!(
            stored.reads.load(Ordering::SeqCst),
            1,
            "actual restoration validates the packed frontier"
        );
        ran.unwrap();
        let [(_, consumer)] = delivered.as_slice() else {
            panic!("one declared occurrence")
        };
        // The restored frontier is ready to the consumer, so a skipped producer partition with stale Complete bits still answers
        // the consumer's rows from both files: what is uncertain stays in the restored publication, not in this readiness.
        let Ok(consumer) = consumer else {
            panic!("the restored frontier binds its consumer")
        };
        let RootOutcome::Complete(table) = &consumer.emit else {
            panic!("the consumer of a ready restored frontier completes")
        };
        assert_eq!(
            table.cells,
            [
                Value::Node(ryo_relations::domain::relations::fixtures::node_id(
                    FileId(0),
                    ryo_relations::domain::relations::types::Position(2)
                )),
                Value::Node(ryo_relations::domain::relations::fixtures::node_id(
                    FileId(1),
                    ryo_relations::domain::relations::types::Position(2)
                )),
            ]
        );
    }
}

/// The restored producer frontier and, in its own bucket, the consumer's file frontier.
struct ConsumerFrontier<'a> {
    producer: &'a CapturedFacts,
    consumer: &'a CapturedFacts,
}
impl FactsFinish for ConsumerFrontier<'_> {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        if bucket == Bucket(10) {
            return self.consumer.kept(bucket);
        }
        self.producer.kept(bucket)
    }
}
