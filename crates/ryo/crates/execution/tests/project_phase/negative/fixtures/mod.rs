//! One project submission through the public phase, with the roots its guard was asked about.
use crate::boundary_fixture::corpus;
use ryo_execution::inbound::use_cases::execution::{
    interface::{
        ExecutionRequest, ExecutionUseCase, ObservationExecution, ProjectRootExecution,
        ProjectStage,
    },
    types::*,
    use_case::ExecutionUseCaseImpl,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::{
    files::types::CorpusInput,
    observations::types::ObservationReadinessLayout,
    stored_facts::types::{Bucket, Digest},
};

/// No compiled rule in these programs has a file frontier or a project relation.
struct NoFacts;
impl facts::FactsFinish for NoFacts {
    fn kept(&self, _: Bucket) -> Result<facts::KeptBucket, facts::FactsError> {
        panic!("a submission control has no file frontier to fetch")
    }
}
impl facts::FactsWorker for NoFacts {
    fn resident_stability(&self) -> facts::ResidentStability {
        facts::ResidentStability::Stable
    }
    fn rows(
        &self,
        _: ryo_relations::domain::relations::types::Relation,
    ) -> Result<ryo_relations::domain::relations::types::ColumnsView<'_>, facts::FactsError> {
        panic!("a submission control has no project relation to read")
    }
    fn columns_mut(&mut self) -> Result<facts::ColumnsMut<'_>, facts::FactsError> {
        panic!("a submission control writes no facts")
    }
    fn keep(&self, _: Bucket, _: facts::WordsToKeep<'_>) -> Result<(), facts::FactsError> {
        panic!("a submission control writes no facts")
    }
    fn release(&mut self) -> Result<(), facts::FactsError> {
        panic!("a submission control writes no facts")
    }
    fn finished(self: Box<Self>) -> Result<Box<dyn facts::FactsFinish>, facts::FactsError> {
        panic!("a submission control writes no facts")
    }
}

/// The two public entries that accept a set of project roots.
#[derive(Clone, Copy)]
pub(crate) enum Entry {
    /// Ordinary roots through the request's project phase.
    Phase,
    /// Named roots through the observation stage that owns that phase.
    Stage,
}
enum Held<'a> {
    Phase(ProjectPhase<'a>),
    Stage(ObservationStage<'a>),
}
impl Held<'_> {
    fn scope(&self, rule: RuleIndex, corpus: &[CorpusInput], covered: &[Covered]) -> ProjectScope {
        let scope = match self {
            Held::Phase(phase) => phase.project_scope(rule, corpus, covered, &[]),
            Held::Stage(stage) => stage.project_scope(rule, corpus, covered, &[]),
        };
        scope.unwrap()
    }
    fn evaluate<E>(
        &self,
        tasks: Vec<ProjectTask<'_>>,
        symbols: &mut Symbols,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
    ) -> ProjectBatch<Result<ProjectRuleOutcome, OperatingError>, E> {
        match self {
            Held::Phase(phase) => phase.evaluate(tasks, symbols, guard),
            Held::Stage(stage) => stage.evaluate(tasks, symbols, guard),
        }
    }
    fn finish(self) {
        match self {
            Held::Phase(phase) => phase.finish().unwrap(),
            Held::Stage(stage) => stage.finish_stage().unwrap(),
        }
    }
}

/// What the caller declared: the scope and frontiers come from `declared`, while the
/// task names `named`. The two differ only when the caller submits a root nobody declared.
#[derive(Clone, Copy)]
pub(crate) struct Root {
    pub declared: usize,
    pub named: usize,
}
pub(crate) struct Submitted {
    pub outcomes: Vec<Result<ProjectRuleOutcome, OperatingError>>,
    pub guarded: Vec<RuleIndex>,
    pub refused: Option<OperatingError>,
}
/// Evaluate `roots` through the real project phase of a request over `rule_count` rules.
/// `refuse` is the rule the guard rejects, if any.
pub(crate) fn submit(
    engine: &ExecutionUseCaseImpl,
    entry: Entry,
    rule_count: usize,
    roots: &[Root],
    refuse: Option<RuleIndex>,
) -> Submitted {
    let mut authored = crate::boundary_fixture::authored_request(rule_count);
    if matches!(entry, Entry::Stage) {
        authored.observations = ObservationIdentity::Named(ObservationReadinessLayout(Digest([7; 16])));
    }
    let prepared = engine.prepare(authored).expect("valid public request");
    let corpus = corpus();
    let request = engine.begin(&prepared, &corpus);
    let covered = [Covered {
        file: FileId(0),
        rules: (0..rule_count).map(RuleIndex).collect(),
        partial: vec![],
    }];
    let mut arenas = Vec::new();
    let held = match entry {
        Entry::Phase => Held::Phase(request.project_phase().unwrap()),
        Entry::Stage => Held::Stage(request.observations(&mut arenas).unwrap()),
    };
    let project_symbols = Symbols::default();
    let mut symbols = Symbols::default();
    let partials = Partials::default();
    let mut scopes = Vec::new();
    let mut frontiers = Vec::new();
    for root in roots {
        let scope = held.scope(RuleIndex(root.declared), &corpus, &covered);
        frontiers.push(
            request
                .prepare_frontiers(
                    FrontierRequest {
                        demand: FrontierDemand::Diagnostic,
                        scope: &scope,
                        rule: RuleIndex(root.declared),
                        first: Bucket(0),
                        facts: &NoFacts,
                        project: &NoFacts,
                        project_symbols: &project_symbols,
                    },
                    &mut symbols,
                )
                .unwrap(),
        );
        scopes.push(scope);
    }
    let tasks = roots
        .iter()
        .zip(&scopes)
        .zip(frontiers)
        .map(|((root, scope), ordinary)| ProjectTask {
            rule: RuleIndex(root.named),
            demand: RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            input: ProjectInput {
                scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &[],
                arenas: &[],
            },
        })
        .collect();
    let mut guarded = Vec::new();
    let batch = held.evaluate(tasks, &mut symbols, |rule| {
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
