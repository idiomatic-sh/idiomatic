//! Project grouping reads only the function names actually captured.
use super::super::*;

/// Migration control for the old pipeline proof's invalid schema and arbitrary cross-file hashes.
/// Every grouped name below is obtained from a real captured Function population.
#[test]
fn project_grouping_reads_only_actual_captured_function_names() {
    use ryo_language::domain::symbols::{fixtures::hash_of, types::ByteSlice};
    use ryo_relations::domain::relations::types::Integer;
    use ryo_values::domain::stored_facts::types::ArenaBytes;
    let engine = engine();
    let mut input = authored_request(1);
    crate::boundary_fixture::plans(&mut input)[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![ColumnIndex(1)],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::GroupCount {
                from: Input::Step(StepId(0)),
                keys: vec![ColumnIndex(0)],
                distinct: None,
            },
        },
    ];
    crate::boundary_fixture::plans(&mut input)[0].frontier = vec![Input::Step(StepId(0))];
    crate::boundary_fixture::plans(&mut input)[0].emit.step = StepId(1);
    let program = engine
        .prepare(input)
        .expect("prepare actual mixed-phase count");
    let mut corpus = corpus();
    if let ryo_values::domain::files::types::CorpusInput::Available(entry) = &mut corpus[0] {
        entry.size = ryo_values::domain::files::types::FileSize(9);
    }
    let mut second = corpus[0].clone();
    if let ryo_values::domain::files::types::CorpusInput::Available(entry) = &mut second {
        entry.path = ryo_language::domain::files::types::FilePath("second.py".into());
    }
    corpus.push(second);
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let mut arena = vec![];
    let mut places = vec![];
    let mut words = Words::default();
    let mut captured_files = vec![];
    let main_hash = hash_of(ByteSlice(b"main"));
    let other_hash = hash_of(ByteSlice(b"other"));
    for file_id in [FileId(0), FileId(1)] {
        let mut symbols = Symbols::default();
        let mut table = crate::boundary_fixture::named_functions(&mut symbols);
        let width = table.kinds.len();
        if file_id == FileId(0) {
            table.cells.truncate(width);
            table.count = RowCount(1);
        } else {
            table.cells.drain(..width);
            table.count = RowCount(2);
        }
        for (index, row) in table.cells.chunks_exact_mut(width).enumerate() {
            row[0] = Value::Node(ryo_relations::domain::relations::fixtures::node_id(
                file_id,
                ryo_relations::domain::relations::types::Position(index as u32),
            ));
            row[2] = Value::File(file_id);
        }
        let read = |relation| {
            assert_eq!(relation, Relation::Function);
            Ok(SourceRows::Computed(&table))
        };
        let elements = ryo_language::domain::elements::types::Elements::default();
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: ResidentStability::Stable,
                    file: file_id,
                    evaluation: &[],
                    publication: &[],
                    intents: &[FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::DiagnosticAndObservations,
                    }],
                    source: ryo_values::domain::files::types::SourceView(b"mainother"),
                    elements: &elements,
                    read: &read,
                    semantic: ryo_execution::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: ryo_execution::application::file_dag::program::file_session::types::KindSupport::Absent,
                },
                &mut symbols,
            )
            .expect("open actual file");
        assert!(matches!(
            file.rule(RuleIndex(0), demand(), &mut symbols)
                .unwrap()
                .emit,
            RootOutcome::Deferred {
                phase: StepScope::CrossFile
            }
        ));

        let mut captures = file
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
            .expect("capture actual names");
        assert_eq!(captures.len(), 1);
        let capture = captures.pop().unwrap().outcome.expect("complete capture");
        file.finish().expect("release file owner");
        captured_files.push((file_id, capture));
    }
    let stored = CapturedFacts {
        captures: captured_files,
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let covered = [FileId(0), FileId(1)].map(|file| Covered {
        file,
        rules: vec![RuleIndex(0)],
        partial: vec![],
    });
    let phase = request.project_phase().unwrap();
    let scope = phase
        .project_scope(
            RuleIndex(0),
            &corpus,
            &covered,
            &[],
            &ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(),
        )
        .unwrap();
    let project_symbols = Symbols::default();
    let mut symbols = Symbols::default();
    let ordinary = request
        .prepare_frontiers(
            FrontierRequest {
                demand: FrontierDemand::Diagnostic,
                scope: &scope,
                rule: RuleIndex(0),
                first: Bucket(0),
                facts: &stored,
                project: &project,
                project_symbols: &project_symbols,
            },
            &mut symbols,
        )
        .expect("admit exact persisted frame");
    assert_eq!(
        project.reads.load(Ordering::SeqCst),
        0,
        "no unrelated project facts"
    );
    assert_eq!(stored.reads.load(Ordering::SeqCst), 1);
    let partials = Partials::default();
    let arenas = [ArenaBytes(arena)];
    let RootOutcome::Complete(result) = crate::boundary_fixture::project_root(
        &phase,
        RuleIndex(0),
        demand(),
        ProjectInput {
            scope: &scope,
            evaluation: &partials,
            publication: &partials,
            ordinary,
            bound: &[],
            arenas: &arenas,
        },
        &mut symbols,
    )
    .unwrap()
    .emit
    else {
        panic!("project count must complete")
    };
    assert_eq!(result.count, RowCount(2));
    assert_eq!(result.kinds, [CellKind::Symbol, CellKind::Number]);
    let mut actual = result
        .cells
        .chunks_exact(2)
        .map(|row| {
            let [Value::Name(name), Value::Number(Integer(count))] = row else {
                panic!("name and count")
            };
            (name.hash, *count)
        })
        .collect::<Vec<_>>();
    let mut expected = vec![(main_hash, 2), (other_hash, 1)];
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
    drop(scope);
    phase.finish().unwrap();
    request.finish();
}
