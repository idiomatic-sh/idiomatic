//! Two declared roots of one population read the provider once.
use crate::boundary_fixture::fixtures::{corpus, demand, engine, source};
use crate::project_phase::fixtures::*;
use crate::project_phase::source_facts::fixtures::{CapturedFacts, ProjectFacts};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution_domain::domain::row_emission::types::RowEmission;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation, Value};
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_values::domain::{plan_parts::types::InputNumber, stored_facts::types::Bucket};
use ryo_values::domain::{plan_parts::types::StepScope, rows::types::RowCount};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn two_declared_roots_of_one_population_read_the_provider_once() {
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
                    semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
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
                    semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
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
        let capture = captures
            .pop()
            .unwrap()
            .outcome
            .expect("actual file frontier");
        // Admission may borrow a resident view more than once. The production
        // guarantee is one source traversal and no source access during finish.
        let resolved = file_reads.get();
        assert!(resolved > 0);
        file.finish()
            .expect("file owner finishes without demanding project sources");
        assert_eq!(
            file_reads.get(),
            resolved,
            "finish cannot demand the source again"
        );
        let stored = CapturedFacts {
            captures: vec![(FileId(0), capture)],
            reads: AtomicUsize::new(0),
        };
        let project = ProjectFacts::new(relation);
        let covered = [Covered {
            file: FileId(0),
            rules: vec![RuleIndex(0)],
            partial: vec![],
        }];
        let phase = request.project_phase().unwrap();
        let project_symbols = Symbols::default();
        let mut symbols = Symbols::default();
        let partials = Partials::default();
        let inapplicable = [
            ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(),
        ];
        let refused =
            [ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default()];
        let pair = [RuleIndex(0), RuleIndex(0)];
        let occurrences = |rules| PhaseRules {
            rules,
            first: &[Bucket(0)],
            corpus: &corpus,
            covered: &covered,
            skipped: &[],
            inapplicable: &inapplicable,
            refused: &refused,
            facts: &stored,
            project: &project,
            project_symbols: &project_symbols,
            arenas: &[],
        };
        // The production sequence: one call with two declared occurrences of one population. The phase prepares every occurrence
        // before it evaluates any, so the second reuses the first's acquisition while nothing outside the engine holds it.
        let mut outcomes = crate::boundary_fixture::fixtures::project_roots(
            &phase,
            occurrences(&pair),
            &partials,
            &partials,
            &mut symbols,
        );
        assert_eq!(stored.reads.load(Ordering::SeqCst), 1);
        assert_eq!(project.reads.load(Ordering::SeqCst), 1);
        let again = outcomes.pop().unwrap().unwrap();
        let outcome = outcomes
            .pop()
            .unwrap()
            .expect("project emit runs at project scope");
        for outcome in [outcome, again] {
            let RootOutcome::Complete(table) = outcome.emit else {
                panic!("one complete project diagnostic table")
            };
            assert_eq!(table.kinds, [CellKind::File]);
            assert_eq!(table.count, RowCount(1));
            assert_eq!(table.cells, [Value::File(FileId(0))]);
        }
        // A later call holds nothing from the earlier ones: its two occurrences share one new acquisition.
        let later = crate::boundary_fixture::fixtures::project_roots(
            &phase,
            occurrences(&pair),
            &partials,
            &partials,
            &mut symbols,
        );
        assert!(later.iter().all(Result::is_ok));
        phase.finish().expect("release project publications");
        assert_eq!(
            stored.reads.load(Ordering::SeqCst),
            2,
            "the two occurrences of one call share one acquisition of the file frontiers"
        );
        assert_eq!(
            project.reads.load(Ordering::SeqCst),
            2,
            "the two occurrences of one call share one acquisition"
        );
        request.finish();
        assert_eq!(
            file_reads.get(),
            resolved,
            "project work cannot reread the file facts after their captured frontier"
        );
    }
}

/// Negative control: a second population of the same rule, observed through the observation stage, is a separate acquisition of
/// the file frontier and of the provider, and answers the same table.
#[test]
fn an_observed_population_is_a_separate_acquisition_with_the_same_answer() {
    use ryo_execution::inbound::use_cases::execution::interface::{
        BatchNumber, BatchOccurrence, ObservationBatches,
    };
    let relation = Relation::ModuleIdentity;
    let engine = engine();
    let prepared = mixed_program_with(
        &engine,
        relation,
        ObservationIdentity::Named(
            ryo_values::domain::records::types::ObservationReadinessLayout(
                ryo_values::domain::stored_facts::types::Digest([33; 16]),
            ),
        ),
    );
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
    let table = source();
    let read = |_| Ok(SourceRows::Computed(&table));
    let mut symbols = Symbols::default();
    let stored = CapturedFacts {
        captures: captured_in_file_0_of(&request, &read, &mut symbols),
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(relation);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0)],
        partial: vec![],
    }];
    let project_symbols = Symbols::default();
    let inapplicable = [Default::default(), Default::default()];
    let refused = [Default::default(), Default::default()];
    let first = [Bucket(0), Bucket(1)];
    let partials = Partials::default();
    let phase = request.project_phase().unwrap();
    let ordinary = crate::boundary_fixture::fixtures::project_root(
        &phase,
        PhaseRules {
            rules: &[RuleIndex(0)],
            first: &first,
            corpus: &corpus,
            covered: &covered,
            skipped: &[],
            inapplicable: &inapplicable,
            refused: &refused,
            facts: &stored,
            project: &project,
            project_symbols: &project_symbols,
            arenas: &[],
        },
        &partials,
        &partials,
        &mut symbols,
    )
    .expect("the ordinary population answers");
    phase.finish().unwrap();
    let after_ordinary = (
        stored.reads.load(Ordering::SeqCst),
        project.reads.load(Ordering::SeqCst),
    );
    let mut arenas = vec![];
    let stage = request
        .observations(&mut arenas)
        .expect("a named program opens its stage");
    let (ran, mut delivered) = crate::project_phase::source_facts::fixtures::run_observations(
        stage,
        ObservationBatches {
            occurrences: &[BatchOccurrence {
                batch: BatchNumber(0),
                rule: RuleIndex(0),
            }],
            readiness: &[],
            rules: &[RuleIndex(0), RuleIndex(1)],
            first: &first,
            corpus: &corpus,
            covered: &covered,
            skipped: &[],
            inapplicable: &inapplicable,
            refused: &refused,
            facts: &stored,
            project: &project,
            project_symbols: &project_symbols,
        },
        &partials,
        &mut symbols,
    );
    ran.expect("the stage runs the observed population");
    let (_, observed) = delivered.pop().unwrap();
    let after_observed = (
        stored.reads.load(Ordering::SeqCst),
        project.reads.load(Ordering::SeqCst),
    );
    assert_eq!(
        after_ordinary,
        (1, 1),
        "the ordinary population acquires the frontier and the provider once"
    );
    assert_eq!(
        after_observed,
        (2, 2),
        "the observed population is a separate acquisition of both"
    );
    let RootOutcome::Complete(ordinary) = ordinary.emit else {
        panic!("the ordinary population completes")
    };
    let Ok(RootOutcome::Complete(observed)) = observed.map(|outcome| outcome.emit) else {
        panic!("the observed population completes")
    };
    assert_eq!(observed, ordinary, "both populations answer the same table");
    assert_eq!(ordinary.cells, [Value::File(FileId(0))]);
    request.finish();
}
