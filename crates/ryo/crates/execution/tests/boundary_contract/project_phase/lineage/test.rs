//! A step each file handed over keeps the lineage of the file it was made in: an antijoin after the files of the files of the
//! functions by themselves, the rule refused on the second file, is uncertain in that file alone, because each row it keeps and
//! each row a missing one would have matched lie in the same file.
use crate::boundary_fixture::fixtures::{FileRun, ran, run_file_into};
use crate::boundary_fixture::fixtures::{authored_request, corpus, engine};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::CaptureContext;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::{EmittedRoot, FileEnd, RuleAfter};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::CapturedFacts;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::fixtures::kinds;
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation, Value as Cell};
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::stored_facts::types::{ArenaBytes, Bucket};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as StepInput, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::RowCount,
};
use std::borrow::Cow;
use std::sync::atomic::AtomicUsize;

/// The file the rule was refused on; the other hands its functions over.
type Input = FileId;
/// The emit's answer, as text: its kind, rows and the files it is uncertain in.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_handed_step_s_rows_lie_in_the_file_that_made_them"),
    word: Word::Succeeds,
    input: || FileId(1),
    expect: Expect::Value(Cow::Borrowed("partial RowCount(0) [Some(FileId(1))]")),
}];

fn subject(refused_file: Input) -> Result<Value, Failure> {
    use ryo_language::domain::elements::types::Elements;
    let file = kinds(Relation::Function)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .and_then(|at| u32::try_from(at).ok())
        .ok_or(())?;
    let mut input = authored_request(1);
    let plan = &mut crate::boundary_fixture::fixtures::plans(&mut input)[0];
    plan.steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: StepInput::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![ColumnIndex(file)],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Antijoin {
                left: StepInput::Step(StepId(0)),
                right: StepInput::Step(StepId(0)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            },
        },
    ];
    plan.frontier = vec![StepInput::Step(StepId(0))];
    plan.emit.step = StepId(1);
    let program = engine().prepare(input).map_err(|_| ())?;
    let mut corpus = corpus();
    let mut second = corpus[0].clone();
    if let ryo_values::domain::files::types::CorpusInput::Available(entry) = &mut second {
        entry.path = ryo_language::domain::files::types::FilePath("second.py".into());
    }
    corpus.push(second);
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let mut arena = vec![];
    let mut places = vec![];
    let mut words = Words::default();
    let mut captured_files = vec![];
    for file_id in [FileId(0), FileId(1)]
        .into_iter()
        .filter(|each| *each != refused_file)
    {
        let mut symbols = Symbols::default();
        let mut table = crate::boundary_fixture::fixtures::named_functions(&mut symbols);
        let width = table.kinds.len();
        for row in table.cells.chunks_exact_mut(width) {
            row[usize::try_from(file).map_err(|_| ())?] = Cell::File(file_id);
            row[0] = Cell::Node(ryo_relations::domain::relations::fixtures::node_id(
                file_id,
                ryo_relations::domain::relations::types::Position(0),
            ));
        }
        table.cells.truncate(width);
        table.count = RowCount(1);
        let mut facts = TableFacts::new(&[(Relation::Function, &table)]);
        let elements = Elements::default();
        let mut rules = run_file_into(
            &request,
            FileRun {
                file: file_id,
                source: b"mainother",
                ..FileRun::of(
                    &[FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::DiagnosticAndObservations,
                    }],
                    &elements,
                )
            },
            &mut symbols,
            CaptureContext {
                worker: WorkerNumber(0),
                arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                places: &mut places,
                words: &mut words,
            },
            &mut facts,
        );
        let rule = ran(&mut rules, 0);
        let EmittedRoot::Delivered(RootOutcome::Deferred { .. }) = rule.emit else {
            return Err(());
        };
        let RuleAfter::Ran { mut captures } = rule.after else {
            return Err(());
        };
        let capture = captures.pop().ok_or(())?.outcome.map_err(|_| ())?;
        let FileEnd::Finished(_) = rules.end else {
            return Err(());
        };
        captured_files.push((file_id, capture));
    }
    let stored = CapturedFacts {
        captures: captured_files,
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let covered = [FileId(0), FileId(1)]
        .into_iter()
        .filter(|each| *each != refused_file)
        .map(|file| Covered {
            file,
            rules: vec![RuleIndex(0)],
            partial: vec![],
        })
        .collect::<Vec<_>>();
    let phase = request.project_phase().map_err(|_| ())?;
    let project_symbols = Symbols::default();
    let mut symbols = Symbols::default();
    let partials = Partials::default();
    let arenas = [ArenaBytes(arena)];
    let outcome = crate::boundary_fixture::fixtures::project_root(
        &phase,
        PhaseRules {
            rules: &[RuleIndex(0)],
            first: &[Bucket(0)],
            corpus: &corpus,
            covered: &covered,
            skipped: &[],
            inapplicable: &[
                ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(
                ),
            ],
            refused: &[
                ryo_execution::inbound::use_cases::execution::interface::RefusedFiles(vec![
                    refused_file,
                ]),
            ],
            facts: &stored,
            project: &project,
            project_symbols: &project_symbols,
            arenas: &arenas,
        },
        &partials,
        &partials,
        &mut symbols,
    );
    Ok(Cow::Owned(match outcome.map(|each| each.emit) {
        Ok(RootOutcome::Complete(table)) => format!("complete {:?}", table.count),
        Ok(RootOutcome::Partial { table, affected }) => {
            format!("partial {:?} {affected:?}", table.count)
        }
        Ok(RootOutcome::Failed(error)) => format!("failed {error:?}"),
        Ok(RootOutcome::Unknown(error)) => format!("unknown {error:?}"),
        Ok(RootOutcome::Deferred { phase }) => format!("deferred {phase:?}"),
        Ok(RootOutcome::Skipped(_)) => "skipped".to_owned(),
        Ok(RootOutcome::NoQuery) => "no query".to_owned(),
        Err(error) => format!("refused {error:?}"),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
