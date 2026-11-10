//! A project root cannot reconstruct an unhanded per-file step from a provider.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::Covered;
use crate::inbound::use_cases::execution::interface::ExecutionUseCase;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::Partials;
use crate::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use ryo_execution_runtime::application::file_dag::program::project_phase::interface::{
    ProjectRootExecution as _, ProjectStage as _,
};
use ryo_execution_runtime::application::file_dag::program::project_phase::types::{
    PhasePartials, PhaseRules,
};
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
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
        self.reads.fetch_add(1, Ordering::Relaxed);
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
fn project_step_without_a_declared_handover_cannot_read_a_per_file_source() -> Result<(), Failure> {
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
    assert!(plan.frontier.is_empty());
    let (engine, program) = fixtures::requests::prepared(plan, vec![], vec![])
        .expect("compile exact phase dependency without inventing a handover");
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
    let phase = request.project_phase().unwrap();
    let facts = UnavailableFacts::default();
    let mut symbols = Symbols::default();
    let project_symbols = Symbols::default();
    // Two declared occurrences of the one rule: the refusal repeats without reacquiring the provider.
    let ProjectBatch::Complete(outcomes) = phase.evaluate_rules(
        PhaseRules {
            rules: &[RuleIndex(0), RuleIndex(0)],
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
    ) else {
        panic!("the guard admits the declared occurrences")
    };
    for outcome in outcomes {
        let outcome = outcome.expect("the occurrence is scoped, prepared and evaluated");
        assert!(
            matches!(outcome.emit, RootOutcome::Failed(OperatingError::NotRead)),
            "a missing file handover must refuse rather than become a complete empty source"
        );
    }
    assert_eq!(
        facts.reads.load(Ordering::Relaxed),
        0,
        "a project invocation cannot reacquire the missing per-file provider"
    );
    phase.finish().unwrap();
    request.finish();
    Ok(())
}
/// The scenarios of this group, one row each.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ProjectStepWithoutADeclaredHandoverCannotReadAPerFileSource,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName(stringify!(
        project_step_without_a_declared_handover_cannot_read_a_per_file_source
    )),
    word: Word::Succeeds,
    input: || Scenario::ProjectStepWithoutADeclaredHandoverCannotReadAPerFileSource,
    expect: Expect::Value(()),
}];
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ProjectStepWithoutADeclaredHandoverCannotReadAPerFileSource => {
            project_step_without_a_declared_handover_cannot_read_a_per_file_source()
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
