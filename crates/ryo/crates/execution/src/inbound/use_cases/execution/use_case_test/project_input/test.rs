//! A project root cannot reconstruct an unhanded per-file step from a provider.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::Covered;
use crate::inbound::use_cases::execution::interface::ExecutionUseCase;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::Partials;
use crate::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_execution_runtime::application::file_dag::program::project_phase::interface::{
    ProgramProjectPhasePort as _, ProjectRootExecution as _,
};
use ryo_execution_runtime::application::file_dag::program::project_phase::types::{
    PhasePartials, PhaseRules,
};
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectBatch;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::errors::FactsError;
use ryo_execution_runtime::outbound::facts::interface::FactsFinish;
use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
use ryo_execution_runtime::outbound::facts::types::ColumnsMut;
use ryo_execution_runtime::outbound::facts::types::KeptBucket;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution_runtime::outbound::facts::types::WordsToKeep;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{ColumnsView, FileId, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input as Source, StepId, StepScope},
    plans::types::Operation,
    stored_facts::types::Bucket,
};
use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};
#[derive(Default)]
struct UnavailableFacts {
    reads: AtomicUsize,
}
fn denied() -> FactsError {
    FactsError::Store {
        reason: "no per-file frontier was handed over".into(),
    }
}
impl FactsWorker for UnavailableFacts {
    fn resident_stability(&self) -> ResidentStability {
        ResidentStability::Stable
    }
    fn rows(&self, _: Relation) -> Result<ColumnsView<'_>, FactsError> {
        let _before = self.reads.fetch_add(1, Ordering::Relaxed);
        Err(denied())
    }
    fn columns_mut(&mut self) -> Result<ColumnsMut<'_>, FactsError> {
        Err(denied())
    }
    fn keep(&self, _: Bucket, _: WordsToKeep<'_>) -> Result<(), FactsError> {
        panic!("project input refusal writes no frontier")
    }
    fn release(&mut self) -> Result<(), FactsError> {
        panic!("the borrowed project fixture is not an execution-owned source")
    }
    fn finished(self: Box<Self>) -> Result<Box<dyn FactsFinish>, FactsError> {
        panic!("the borrowed project fixture is never finished by execution")
    }
}
impl FactsFinish for UnavailableFacts {
    fn kept(&self, _: Bucket) -> Result<KeptBucket, FactsError> {
        panic!("the plan declares no restored frontier to fetch")
    }
}
/// Evaluates `occurrences` occurrences of one cross-file rule whose project step reads a per-file
/// select no frontier hands over, with a provider that refuses every read.
fn evaluate(occurrences: Occurrences) -> Result<Value, Failure> {
    let mut plan = inputs::plan(vec![
        Operation::Select {
            from: Source::Relation(Relation::Function),
            kept: vec![ColumnIndex(0)],
            comparisons: vec![],
        },
        Operation::Project {
            from: Source::Step(StepId(0)),
            columns: vec![ColumnIndex(0)],
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
        },
    ]);
    plan.steps[1].scope = StepScope::CrossFile;
    let frontier = plan.frontier.len();
    let (_engine, program) = fixtures::requests::prepared(plan, vec![], vec![])?;
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("operator-contract.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let request = fixtures::requests::execution().execution(&program, &corpus, None);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0)],
        partial: vec![],
    }];
    let phase = request.project_phase()?;
    let facts = UnavailableFacts::default();
    let mut symbols = Symbols::default();
    let project_symbols = Symbols::default();
    let rules = vec![RuleIndex(0); occurrences.0];
    let batch = phase.evaluate_rules(
        PhaseRules {
            rules: &rules,
            first: &[Bucket(0)],
            corpus: &corpus,
            covered: &covered,
            skipped: &[],
            inapplicable: &[
                crate::inbound::use_cases::execution::interface::InapplicableFiles::default(),
            ],
            refused: &[crate::inbound::use_cases::execution::interface::RefusedFiles::default()],
            facts: &facts,
            project: &facts,
            project_symbols: &project_symbols,
            arenas: &[],
        },
        &mut symbols,
        |_| Ok::<(), OperatingError>(()),
        |_| {
            Ok(PhasePartials {
                evaluation: Partials::default(),
                publication: Partials::default(),
            })
        },
        |_| Ok(()),
        |_, outcome, _, _, _| Ok(outcome),
    );
    let (complete, emitted) = match batch {
        ProjectBatch::Complete(outcomes) => (
            true,
            outcomes
                .into_iter()
                .map(|outcome| match outcome {
                    Ok(outcome) => match outcome.emit {
                        RootOutcome::Failed(error) => Emitted::Failed(error),
                        RootOutcome::Complete(_)
                        | RootOutcome::Partial { .. }
                        | RootOutcome::Skipped(_)
                        | RootOutcome::Unknown(_)
                        | RootOutcome::NoQuery
                        | RootOutcome::Deferred { .. } => Emitted::Answered,
                    },
                    Err(error) => Emitted::Refused(format!("{error:?}")),
                })
                .collect(),
        ),
        ProjectBatch::Refused { .. } => (false, vec![]),
    };
    let provider_reads = facts.reads.load(Ordering::Relaxed);
    phase.finish()?;
    request.finish();
    Ok(Handover {
        frontier,
        complete,
        emitted: Cow::Owned(emitted),
        provider_reads,
    })
}

/// How many occurrences of the one rule the project phase evaluates.
#[derive(Clone, Copy, Debug)]
struct Occurrences(usize);

/// What one occurrence emitted.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Emitted {
    /// The root failed with this error.
    Failed(OperatingError),
    /// The root answered anything but a failure.
    Answered,
    /// The occurrence was refused, as the refusal prints.
    Refused(String),
}

/// What the project phase shows: the plan's declared frontier, whether the batch completed, what
/// each occurrence emitted, and the reads the provider was asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Handover {
    frontier: usize,
    complete: bool,
    emitted: Cow<'static, [Emitted]>,
    provider_reads: usize,
}

type Input = Occurrences;
type Value = Handover;
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName(stringify!(
        project_step_without_a_declared_handover_cannot_read_a_per_file_source
    )),
    word: Word::Succeeds,
    // A missing file handover refuses rather than becoming a complete empty source, and the refusal
    // repeats without reacquiring the provider.
    input: || Occurrences(2),
    expect: Expect::Value(Handover {
        frontier: 0,
        complete: true,
        emitted: Cow::Borrowed(&[
            Emitted::Failed(OperatingError::NotRead),
            Emitted::Failed(OperatingError::NotRead),
        ]),
        provider_reads: 0,
    }),
}];
fn subject(occurrences: Input) -> Result<Value, Failure> {
    evaluate(occurrences)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
