//! The files, the function rows of each and the run through the owner that the rewrite proofs compare plans by.
use super::super::*;
use ryo_execution_domain::domain::frontier_capture::types::CapturedFrontier;
use ryo_execution_runtime::outbound::facts::errors::FactsError;
use ryo_execution_runtime::outbound::facts::interface::FactsFinish;
use ryo_execution_runtime::outbound::facts::types::{BucketSource, KeptBucket};
use ryo_language::domain::symbols::{fixtures::intern, types::ByteSlice};
use ryo_relations::domain::relations::fixtures::node_id;
use ryo_relations::domain::relations::types::{Offset, Position, Span};
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
};
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::stored_facts::types::{ArenaBytes, KeptWords, PackedWords};

/// The text of every file: the two names its functions take.
const SOURCE: &[u8] = b"mainother";

/// The columns of the function relation, in its order.
pub(super) const ID: ColumnIndex = ColumnIndex(0);
pub(super) const NAME: ColumnIndex = ColumnIndex(1);
pub(super) const FILE: ColumnIndex = ColumnIndex(2);
pub(super) const SPAN: ColumnIndex = ColumnIndex(3);

/// One function of a file: whether it is named `main`, and where it starts and ends.
#[derive(Clone, Copy)]
pub(super) struct Function {
    pub main: bool,
    pub start: u32,
    pub end: u32,
}

/// The functions of three files. The first function of the first two files has the same span in both, so a join on spans alone
/// pairs functions of two files; the third file has no function named `main`.
pub(super) fn files() -> Vec<Vec<Function>> {
    let function = |main, start, end| Function { main, start, end };
    vec![
        vec![function(true, 0, 4), function(false, 4, 9)],
        vec![function(false, 0, 4), function(true, 4, 9)],
        vec![function(false, 0, 4)],
    ]
}

/// The function rows of `file`, their names interned in its symbols; each function is a node of the file at its place.
fn functions(
    file: FileId,
    functions: &[Function],
    symbols: &mut Symbols,
) -> ryo_values::domain::rows::types::Table {
    assert_eq!(
        kinds(Relation::Function),
        [
            CellKind::Node,
            CellKind::Symbol,
            CellKind::File,
            CellKind::Span
        ],
        "the proofs name the function relation's columns by place"
    );
    let main = intern(symbols, ByteSlice(SOURCE), Offset(0), Offset(4));
    let other = intern(symbols, ByteSlice(SOURCE), Offset(4), Offset(9));
    let mut cells = Vec::new();
    for (place, function) in functions.iter().enumerate() {
        cells.extend([
            Value::Node(node_id(file, Position(u32::try_from(place).unwrap()))),
            Value::Symbol(if function.main { main } else { other }),
            Value::File(file),
            Value::Span(Span {
                start: Offset(function.start),
                end: Offset(function.end),
            }),
        ]);
    }
    ryo_values::domain::rows::types::Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(functions.len()),
        cells,
    }
}

/// A selection of the function relation's columns, made in each file.
pub(super) fn functions_kept(kept: &[ColumnIndex]) -> PlanStep {
    PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: Input::Relation(Relation::Function),
            comparisons: vec![],
            kept: kept.to_vec(),
        },
    }
}

/// The frontiers the files captured, by bucket.
struct CapturedBuckets {
    captures: Vec<(FileId, CapturedFrontier)>,
}
impl FactsFinish for CapturedBuckets {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        let mut words = vec![];
        let mut sources = vec![];
        for (file, capture) in self
            .captures
            .iter()
            .filter(|(_, capture)| capture.bucket == bucket)
        {
            sources.push(BucketSource {
                file: *file,
                start: ryo_execution_runtime::outbound::facts::types::KeptWordsStart(words.len()),
                length: ryo_execution_runtime::outbound::facts::types::KeptWordsLength(
                    capture.words.0.len(),
                ),
                proof: capture.proof.clone(),
            });
            words.extend_from_slice(&capture.words.0);
        }
        Ok(KeptBucket {
            words: Arc::new(KeptWords(PackedWords(words))),
            sources,
        })
    }
}

/// The rows of a table, each written out, in order: the proofs compare rows, not the order a run hands them over in.
fn written(table: &ryo_values::domain::rows::types::Table) -> Vec<String> {
    let width = table.kinds.len().max(1);
    let mut rows: Vec<String> = table
        .cells
        .chunks_exact(width)
        .map(|row| format!("{row:?}"))
        .collect();
    rows.sort_unstable();
    rows
}

/// The rows a plan of `steps` emitting `emitted` answers over the functions of `files`, run through the owner as a run runs it:
/// each file's steps in the file, then the steps after the files over what every file captured.
pub(super) fn rows(steps: Vec<PlanStep>, emitted: StepId, files: &[Vec<Function>]) -> Vec<String> {
    let crossing = steps.iter().any(|step| step.scope == StepScope::CrossFile);
    let engine = engine();
    let mut input = authored_request(1);
    let plan = &mut crate::boundary_fixture::plans(&mut input)[0];
    plan.frontier =
        ryo_query_language::inbound::use_cases::compile::fixtures::rewrites::frontier(&steps);
    plan.steps = steps;
    plan.emit = Emit {
        step: emitted,
        subject: ColumnIndex(0),
        key: vec![],
        headings: vec![],
    };
    let program = engine.prepare(input).expect("prepare the plan");
    let corpus: Vec<CorpusInput> = (0..files.len())
        .map(|place| {
            CorpusInput::Available(CorpusEntry {
                executable: ExecutablePermission::Unknown,
                path: ryo_language::domain::files::types::FilePath(format!("file-{place}").into()),
                size: FileSize(SOURCE.len() as u64),
                modified: Nanoseconds(0),
            })
        })
        .collect();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let mut arena = vec![];
    let mut places = vec![];
    let mut words = Words::default();
    let mut captures = vec![];
    let mut answered = vec![];
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: if crossing {
            CaptureIntent::DiagnosticAndObservations
        } else {
            CaptureIntent::Disabled
        },
    }];
    for (place, each) in files.iter().enumerate() {
        let file_id = FileId(u32::try_from(place).unwrap());
        let mut symbols = Symbols::default();
        let table = functions(file_id, each, &mut symbols);
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
                    intents: &intents,
                    source: ryo_values::domain::files::types::SourceView(SOURCE),
                    elements: &elements,
                    read: &read,
                    semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
                },
                &mut symbols,
            )
            .expect("open the file");
        match file
            .rule(RuleIndex(0), demand(), &mut symbols)
            .expect("run the file")
            .emit
        {
            RootOutcome::Complete(table) => answered.extend(written(&table)),
            RootOutcome::Deferred {
                phase: StepScope::CrossFile,
            } => {
                for capture in file
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
                    .expect("capture the file's frontiers")
                {
                    captures.push((file_id, capture.outcome.expect("a complete capture")));
                }
            }
            _ => panic!("a file answers its rows or defers them to after the files"),
        }
        let _finished = file.finish().expect("release the file");
    }
    if crossing {
        let facts = CapturedBuckets { captures };
        let project = ProjectFacts::empty(Relation::ModuleIdentity);
        let covered: Vec<Covered> = (0..files.len())
            .map(|place| Covered {
                file: FileId(u32::try_from(place).unwrap()),
                rules: vec![RuleIndex(0)],
                partial: vec![],
            })
            .collect();
        let phase = request.project_phase().expect("the project phase");
        let project_symbols = Symbols::default();
        let mut symbols = Symbols::default();
        let partials = Partials::default();
        let arenas = [ArenaBytes(arena)];
        let RootOutcome::Complete(table) = crate::boundary_fixture::project_root(
            &phase,
            PhaseRules {
                rules: &[RuleIndex(0)],
                first: &[Bucket(0)],
                corpus: &corpus,
                covered: &covered,
                skipped: &[],
                inapplicable: &[ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default()],
                refused: &[ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default()],
                facts: &facts,
                project: &project,
                project_symbols: &project_symbols,
                arenas: &arenas,
            },
            &partials,
            &partials,
            &mut symbols,
        )
        .expect("run the steps after the files")
        .emit
        else {
            panic!("the steps after the files complete")
        };
        answered.extend(written(&table));
        phase.finish().expect("finish the project phase");
    }
    request.finish();
    answered.sort_unstable();
    answered
}
