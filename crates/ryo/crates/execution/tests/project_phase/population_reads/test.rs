//! Two declared roots of one population read the provider once.
use super::super::*;

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
        let mut outcomes = crate::boundary_fixture::project_roots(
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
        let later = crate::boundary_fixture::project_roots(
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
