//! A deferred project root feeds nothing at file scope, and its file frontier walks the source once.
use crate::boundary_fixture::fixtures::{corpus, demand, engine, source};
use crate::project_phase::fixtures::mixed_program;
use ryo_execution::inbound::use_cases::execution::interface::CaptureContext;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileInput;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::RowEmission;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::SourceLanguage;
use ryo_execution::inbound::use_cases::execution::interface::SourceRows;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::{plan_parts::types::InputNumber, stored_facts::types::Bucket};
use std::cell::Cell;

#[test]
fn a_deferred_project_root_feeds_no_file_step_and_its_frontier_walks_the_source_once() {
    for relation in [
        Relation::ProjectNamespaceMembership,
        Relation::ModuleIdentity,
        Relation::ManifestDiagnostic,
    ] {
        let engine = engine();
        let prepared = mixed_program(&engine, relation);
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
        let table = source();
        let file_reads = Cell::new(0);
        let read = |actual| {
            assert_eq!(
                actual,
                Relation::Function,
                "project facts must never be read by the file owner"
            );
            file_reads.set(file_reads.get() + 1);
            Ok(SourceRows::Computed(&table))
        };
        let elements = ryo_language::domain::elements::types::Elements::default();
        let mut symbols = Symbols::default();
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &[FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Forwarded,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::DiagnosticAndObservations,
                    }],
                    source: ryo_values::domain::files::types::SourceView(b"abc"),
                    elements: &elements,
                    read: &read,
                    semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                    kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
                },
                &mut symbols,
            )
            .expect("open actual file owner");
        let mut emitted = vec![];
        assert!(matches!(
            file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                emitted.extend_from_slice(row);
                Ok(())
            })
            .unwrap(),
            RowEmission::Deferred {
                phase: StepScope::CrossFile
            }
        ));
        assert!(emitted.is_empty());
        assert_eq!(file_reads.get(), 0);
        assert!(
            file.fixture_counts(RuleIndex(0))
                .unwrap()
                .fed
                .iter()
                .all(|count| count.0 == [0; 3])
        );
        file.finish()
            .expect("finish separately declared deferred forward root");
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &[FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::DiagnosticAndObservations,
                    }],
                    source: ryo_values::domain::files::types::SourceView(b"abc"),
                    elements: &elements,
                    read: &read,
                    semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                    kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
                },
                &mut symbols,
            )
            .expect("open actual file owner");
        assert!(matches!(
            file.rule(RuleIndex(0), demand(), &mut symbols)
                .unwrap()
                .emit,
            RootOutcome::Deferred {
                phase: StepScope::CrossFile
            }
        ));
        assert_eq!(file_reads.get(), 0);
        assert!(
            file.fixture_counts(RuleIndex(0))
                .unwrap()
                .fed
                .iter()
                .all(|count| count.0 == [0; 3])
        );
        let mut arena = vec![];
        let mut places = vec![];
        let mut words = Words::default();
        let mut captures = file
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
            .expect("file frontier still executes");
        assert_eq!(
            captures.len(),
            1,
            "project-owned frontier is not captured as a file input"
        );
        assert_eq!(captures[0].input, InputNumber(0));
        let _capture = captures
            .pop()
            .unwrap()
            .outcome
            .expect("actual file frontier");
        // Admission may borrow a resident view more than once. The production
        // guarantee is one source traversal and no source access during finish.
        let resolved = file_reads.get();
        assert!(resolved > 0);
        assert_eq!(file.fixture_walk_counts().passes.0, 1);
        assert_eq!(
            file.fixture_walk_counts().source_rows.0,
            u64::try_from(table.count.0).expect("a row count fits in u64")
        );
        assert_eq!(file.fixture_counts(RuleIndex(0)).unwrap().fed[1].0, [0; 3]);
        file.finish()
            .expect("file owner finishes without demanding project sources");
        assert_eq!(
            file_reads.get(),
            resolved,
            "finish cannot demand the source again"
        );
    }
}
