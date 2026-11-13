//! The files, the function rows of each and the run through the owner that the rewrite proofs compare plans by.
use crate::boundary_fixture::fixtures::{authored_request, demand, engine};
use ryo_execution::inbound::use_cases::execution::interface::CaptureContext;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileInput;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::SourceLanguage;
use ryo_execution::inbound::use_cases::execution::interface::SourceRows;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_rewrites::CapturedBuckets;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::symbols::{fixtures::intern, types::ByteSlice};
use ryo_relations::domain::relations::fixtures::node_id;
use ryo_relations::domain::relations::types::{Offset, Position, Span};
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_relations::domain::relations::{
    fixtures::kinds,
    types::{CellKind, FileId, Relation, Value},
};
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
};
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::RowCount,
};

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

/// The function relation's columns, in place order: the cells `functions` builds follow them.
pub(super) fn function_columns() -> &'static [CellKind] {
    kinds(Relation::Function)
}

/// The function rows of `file`, their names interned in its symbols; each function is a node of the file at its place.
/// The cells follow the relation's columns by place, as `the_proofs_name_the_function_relations_columns_by_place` pins.
fn functions(
    file: FileId,
    functions: &[Function],
    symbols: &mut Symbols,
) -> ryo_values::domain::rows::types::Table {
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
    let plan = &mut crate::boundary_fixture::fixtures::plans(&mut input)[0];
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
                size: FileSize(u64::try_from(SOURCE.len()).expect("a source length fits in u64")),
                modified: Nanoseconds(0),
            })
        })
        .collect();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
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
        // The fake serves the function relation alone; a read of any other is refused, and the rule with it.
        let read = |relation| {
            if relation == Relation::Function {
                Ok(SourceRows::Computed(&table))
            } else {
                Err(OperatingError::NotRead)
            }
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
                    semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                    kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
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
            RootOutcome::Partial { .. }
            | RootOutcome::Skipped(_)
            | RootOutcome::Unknown(_)
            | RootOutcome::Failed(_)
            | RootOutcome::NoQuery
            | RootOutcome::Deferred { .. } => {
                panic!("a file answers its rows or defers them to after the files")
            }
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
        let RootOutcome::Complete(table) = crate::boundary_fixture::fixtures::project_root(
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
