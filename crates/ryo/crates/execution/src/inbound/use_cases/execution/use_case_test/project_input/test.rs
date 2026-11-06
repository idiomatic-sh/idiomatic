//! A project root cannot reconstruct an unhanded per-file step from a provider.
use super::super::super::{fixtures, interface::*, types::*};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{ColumnsView, FileId, Relation};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input as Source, StepId},
    plans::types::{Operation, StepScope},
    stored_facts::types::Bucket,
};
use std::sync::atomic::{AtomicUsize, Ordering};
#[derive(Default)]
struct UnavailableFacts {
    reads: AtomicUsize,
}
fn denied() -> facts::FactsError {
    facts::FactsError::Store {
        reason: "no per-file frontier was handed over".into(),
    }
}
impl facts::FactsWorker for UnavailableFacts {
    fn resident_stability(&self) -> facts::ResidentStability {
        facts::ResidentStability::Stable
    }
    fn rows(&self, _: Relation) -> Result<ColumnsView<'_>, facts::FactsError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        Err(denied())
    }
    fn columns_mut(&mut self) -> Result<facts::ColumnsMut<'_>, facts::FactsError> {
        Err(denied())
    }
    fn keep(&self, _: Bucket, _: facts::WordsToKeep<'_>) -> Result<(), facts::FactsError> {
        panic!("project input refusal writes no frontier")
    }
    fn release(&mut self) -> Result<(), facts::FactsError> {
        panic!("the borrowed project fixture is not an execution-owned source")
    }
    fn finished(self: Box<Self>) -> Result<Box<dyn facts::FactsFinish>, facts::FactsError> {
        panic!("the borrowed project fixture is never finished by execution")
    }
}
impl facts::FactsFinish for UnavailableFacts {
    fn kept(&self, _: Bucket) -> Result<facts::KeptBucket, facts::FactsError> {
        panic!("the plan declares no restored frontier to fetch")
    }
}
#[test]
fn project_step_without_a_declared_handover_cannot_read_a_per_file_source() {
    let mut plan = fixtures::plan(vec![
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
    let (engine, program) = fixtures::prepared(plan, vec![], vec![])
        .expect("compile exact phase dependency without inventing a handover");
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("operator-contract.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let request = engine.begin(&program, &corpus);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0)],
        partial: vec![],
    }];
    let phase = request.project_phase().unwrap();
    let scope = phase
        .project_scope(
            RuleIndex(0),
            &corpus,
            &covered,
            &[],
            &crate::inbound::use_cases::execution::types::InapplicableFiles::default(),
        )
        .unwrap();
    let facts = UnavailableFacts::default();
    let mut symbols = Symbols::default();
    let project_symbols = Symbols::default();
    let partials = Partials::default();
    // Two declared roots of the one rule: the refusal repeats without reacquiring the provider.
    let tasks = (0..2)
        .map(|_| ProjectTask {
            rule: RuleIndex(0),
            demand: RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            input: ProjectInput {
                scope: &scope,
                evaluation: &partials,
                publication: &partials,
                ordinary: request
                    .prepare_frontiers(
                        FrontierRequest {
                            demand: FrontierDemand::Diagnostic,
                            scope: &scope,
                            rule: RuleIndex(0),
                            first: Bucket(0),
                            facts: &facts,
                            project: &facts,
                            project_symbols: &project_symbols,
                        },
                        &mut symbols,
                    )
                    .unwrap(),
                bound: &[],
                arenas: &[],
            },
        })
        .collect();
    let ProjectBatch::Complete(outcomes) = phase.evaluate(tasks, &mut symbols, |_| {
        Ok::<(), OperatingError>(())
    }) else {
        panic!("the guard admits the declared roots")
    };
    for outcome in outcomes {
        let outcome = outcome.unwrap();
        assert!(
            matches!(outcome.emit, RootOutcome::Failed(OperatingError::NotRead)),
            "a missing file handover must refuse rather than become a complete empty source"
        );
        assert!(outcome.counts.fed.iter().all(|fed| fed.0 == [0; 3]));
    }
    assert_eq!(
        facts.reads.load(Ordering::Relaxed),
        0,
        "a project invocation cannot reacquire the missing per-file provider"
    );
    drop(scope);
    phase.finish().unwrap();
    request.finish();
}
