//! Named restoration retains partition-qualified physical coverage despite cached readiness.
use crate::boundary_fixture::fixtures::Observed as _;
use crate::boundary_fixture::fixtures::{FileRun, ran, run_file};
use crate::boundary_fixture::fixtures::{corpus, engine, source};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileObservationReadiness;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::{FileEnd, RuleAfter};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_restoration::ConsumerFrontier;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::CapturedFacts;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Relation, Value};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::{
    files::types::CorpusInput,
    runs::types::{RuleApplicability, SkipReason, Skipped},
};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn restored_named_frontier_keeps_skipped_partition_provenance_despite_complete_readiness() {
    use ryo_execution::inbound::use_cases::execution::interface::{
        BatchNumber, BatchOccurrence, ObservationBatches,
    };
    for rejected in [false, true] {
        let engine = engine();
        let mut authored = crate::file_intents::fixtures::named_request();
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
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
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
            let facts = TableFacts::new(&[(Relation::Function, &table)]);
            let mut symbols = Symbols::default();
            // The consumer's own file frontier is kept in its own bucket.
            let mut rules = run_file(
                &request,
                FileRun {
                    file: file_id,
                    first: &[Bucket(0), Bucket(10)],
                    ..FileRun::of(
                        &[
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
                        &elements,
                    )
                },
                &mut symbols,
                &facts,
            );
            let RuleAfter::Ran {
                captures: mut produced,
            } = ran(&mut rules, 0).after
            else {
                panic!("the producer runs and captures")
            };
            let capture = produced.pop().unwrap();
            captures.push((file_id, capture.outcome.unwrap()));
            let RuleAfter::Ran { captures: consumed } = ran(&mut rules, 1).after else {
                panic!("the consumer runs and captures")
            };
            for consumer in consumed {
                consumer_captures.push((file_id, consumer.outcome.unwrap()));
            }
            let FileEnd::Finished(root) = rules.end else {
                panic!("the file finishes")
            };
            let root = root.observed("the finished file observed its readiness");
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
        let mut arenas =
            ryo_execution::inbound::use_cases::execution::interface::ObservationArenas::default();
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
                occurrences: &[BatchOccurrence {
                    batch: BatchNumber(0),
                    rule: RuleIndex(1),
                }],
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
