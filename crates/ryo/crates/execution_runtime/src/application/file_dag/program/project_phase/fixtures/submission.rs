//! The submission of a caller-built root set through the crate-private root evaluation of a phase or a stage.
use super::{Entry, corpus, covered, prepare_for, program};
use crate::application::file_dag::program::observation_stage::interface::ObservationExecution as _;
use crate::application::file_dag::program::observation_stage::types::ObservationStage;
use crate::application::file_dag::program::project_phase::interface::ProjectStage as _;
use crate::application::file_dag::program::project_phase::types::{
    PhaseScope, ProjectInput, ProjectPhase, ProjectTask,
};
use crate::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
use crate::application::file_dag::program::rule_outcome::types::{
    ProjectBatch, ProjectOutputOutcome, RuleOutcome,
};
use ryo_execution_domain::domain::coverage::types::{
    Covered, InapplicableFiles, Partials, RefusedFiles, RuleIndex,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::{EmitDemand, ObservationDemand, RuleDemand};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::CorpusInput;

pub(crate) enum Held<'a> {
    Phase(ProjectPhase<'a>),
    Stage(ObservationStage<'a>),
}
impl Held<'_> {
    fn phase(&self) -> &ProjectPhase<'_> {
        match self {
            Held::Phase(phase) => phase,
            Held::Stage(stage) => &stage.phase,
        }
    }
    fn scope(&self, rule: RuleIndex, corpus: &[CorpusInput], covered: &[Covered]) -> PhaseScope {
        self.phase()
            .project_scope(
                rule,
                corpus,
                covered,
                &[],
                &InapplicableFiles::default(),
                &RefusedFiles::default(),
            )
            .expect("a declared rule has a scope")
    }
    fn finish(self) {
        match self {
            Held::Phase(phase) => phase.finish().expect("the phase finishes"),
            Held::Stage(stage) => stage.finish_stage().expect("the stage finishes"),
        }
    }
}

/// What the caller declared: the scope and frontiers come from `declared`, while the task names `named`. The two differ only when
/// the caller submits a root nobody declared.
#[derive(Clone, Copy)]
pub(crate) struct Root {
    pub(crate) declared: usize,
    pub(crate) named: usize,
}
pub(crate) struct Submitted {
    pub(crate) outcomes: Vec<Result<RuleOutcome<ProjectOutputOutcome>, OperatingError>>,
    pub(crate) guarded: Vec<RuleIndex>,
    pub(crate) refused: Option<OperatingError>,
}
/// Evaluate `roots` through the real project phase of a request over `rule_count` rules; `refuse` is the rule the guard rejects,
/// if any.
pub(crate) fn submit(
    entry: Entry,
    rule_count: usize,
    roots: &[Root],
    refuse: Option<RuleIndex>,
) -> Submitted {
    let prepared = program(entry, rule_count);
    let corpus = corpus();
    let request = crate::application::file_dag::program::request_execution::service::begin(
        &prepared, &corpus,
        None,
    );
    let covered = covered(rule_count);
    let mut arenas = Vec::new();
    let held = match entry {
        Entry::Phase => Held::Phase(request.project_phase().expect("the phase opens")),
        Entry::Stage => Held::Stage(request.observations(&mut arenas).expect("the stage opens")),
    };
    let mut symbols = Symbols::default();
    let partials = Partials::default();
    let scopes: Vec<PhaseScope> = roots
        .iter()
        .map(|root| held.scope(RuleIndex(root.declared), &corpus, &covered))
        .collect();
    let tasks = roots
        .iter()
        .zip(&scopes)
        .map(|(root, scope)| ProjectTask {
            rule: RuleIndex(root.named),
            demand: RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            input: ProjectInput {
                scope,
                evaluation: &partials,
                publication: &partials,
                ordinary: prepare_for(&request, scope, RuleIndex(root.declared))
                    .expect("declared frontiers"),
                bound: &[],
                arenas: &[],
            },
        })
        .collect();
    let mut guarded = Vec::new();
    let batch = held.phase().evaluate(tasks, &mut symbols, |rule| {
        guarded.push(rule);
        if Some(rule) == refuse {
            Err(OperatingError::WrongBinding)
        } else {
            Ok(())
        }
    });
    let (outcomes, refused) = match batch {
        ProjectBatch::Complete(outcomes) => (outcomes, None),
        ProjectBatch::Refused { completed, error } => (completed, Some(error)),
    };
    drop(scopes);
    held.finish();
    request.finish();
    Submitted {
        outcomes,
        guarded,
        refused,
    }
}

/// A scope opened by one phase of the request, evaluated through another phase of it.
pub(crate) fn foreign_phase_outcome() -> Result<RuleOutcome<ProjectOutputOutcome>, OperatingError> {
    let prepared = program(Entry::Phase, 1);
    let corpus = corpus();
    let request = crate::application::file_dag::program::request_execution::service::begin(
        &prepared, &corpus,
        None,
    );
    let covered = covered(1);
    let (owner, other) = (
        request.project_phase().expect("the first phase opens"),
        request.project_phase().expect("the second phase opens"),
    );
    let scope = owner
        .project_scope(
            RuleIndex(0),
            &corpus,
            &covered,
            &[],
            &InapplicableFiles::default(),
            &RefusedFiles::default(),
        )
        .expect("a declared rule has a scope");
    let ordinary = prepare_for(&request, &scope, RuleIndex(0)).expect("declared frontiers");
    let partials = Partials::default();
    let batch = other.evaluate(
        vec![ProjectTask {
            rule: RuleIndex(0),
            demand: RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            input: ProjectInput {
                scope: &scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &[],
                arenas: &[],
            },
        }],
        &mut Symbols::default(),
        |_| Ok::<(), OperatingError>(()),
    );
    drop(scope);
    owner.finish().expect("the first phase finishes");
    other.finish().expect("the second phase finishes");
    request.finish();
    match batch {
        ProjectBatch::Complete(mut done) => done.remove(0),
        ProjectBatch::Refused { error, .. } => Err(error),
    }
}
