//! A project submission answers exactly the roots it names, and refuses by name the ones it did not declare. These are the runtime's
//! own contracts on the crate-private root evaluation: a caller of `evaluate_occurrences` cannot assemble such roots.
use super::fixtures::{Entry, corpus, covered, prepare_for, program};
use crate::application::file_dag::program::observation_stage::interface::ObservationExecution as _;
use crate::application::file_dag::program::observation_stage::types::ObservationStage;
use crate::application::file_dag::program::project_phase::interface::ProjectStage as _;
use crate::application::file_dag::program::project_phase::types::{PhaseScope, ProjectInput, ProjectPhase, ProjectTask};
use crate::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
use crate::application::file_dag::program::rule_outcome::types::{ProjectBatch, ProjectOutputOutcome, RuleOutcome};
use ryo_execution_domain::domain::coverage::types::{Covered, InapplicableFiles, Partials, RuleIndex};
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
            .project_scope(rule, corpus, covered, &[], &InapplicableFiles::default())
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
pub(crate) fn submit(entry: Entry, rule_count: usize, roots: &[Root], refuse: Option<RuleIndex>) -> Submitted {
    let prepared = program(entry, rule_count);
    let corpus = corpus();
    let request = crate::application::file_dag::program::request_execution::service::begin(&prepared, &corpus);
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
                ordinary: prepare_for(&request, scope, RuleIndex(root.declared)).expect("declared frontiers"),
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
    let request = crate::application::file_dag::program::request_execution::service::begin(&prepared, &corpus);
    let covered = covered(1);
    let (owner, other) = (
        request.project_phase().expect("the first phase opens"),
        request.project_phase().expect("the second phase opens"),
    );
    let scope = owner
        .project_scope(RuleIndex(0), &corpus, &covered, &[], &InapplicableFiles::default())
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


fn root(rule: usize) -> Root {
    Root {
        declared: rule,
        named: rule,
    }
}

#[test]
fn an_omitted_declared_root_gets_no_outcome_and_no_guard_and_every_submitted_root_gets_one() {
    let all = [root(0), root(1), root(2)];
    for count in 0..=all.len() {
        let submitted = submit(Entry::Phase, 3, &all[..count], None);
        assert!(submitted.refused.is_none());
        assert_eq!(
            submitted.outcomes.len(),
            count,
            "one outcome per submitted root, none invented for the rest"
        );
        assert!(submitted.outcomes.iter().all(Result::is_ok));
    }
    let omitted = submit(Entry::Phase, 3, &[root(0), root(2)], None);
    assert_eq!(omitted.outcomes.len(), 2);
    assert!(
        !omitted.guarded.contains(&RuleIndex(1)),
        "the omitted root is never admitted, guarded or delivered"
    );
}

#[test]
fn an_extra_root_is_refused_by_name_without_displacing_the_declared_roots() {
    let extra = Root {
        declared: 0,
        named: 3,
    };
    let submitted = submit(Entry::Phase, 3, &[root(0), extra, root(2)], None);
    assert!(submitted.refused.is_none());
    assert_eq!(submitted.outcomes.len(), 3, "the extra root keeps its own slot");
    assert!(submitted.outcomes[0].is_ok());
    assert!(
        matches!(submitted.outcomes[1], Err(OperatingError::NotRun)),
        "a rule the program never declared is a typed refusal"
    );
    assert!(submitted.outcomes[2].is_ok());

    let guarded = submit(Entry::Phase, 3, &[root(0), extra, root(2)], Some(RuleIndex(3)));
    assert!(matches!(guarded.refused, Some(OperatingError::WrongBinding)));
    assert_eq!(
        guarded.outcomes.len(),
        1,
        "refusal at the extra root completes only the roots declared before it"
    );
    assert!(!guarded.guarded.contains(&RuleIndex(2)));
}

/// The scope and frontiers come from rule 0; the task names rule 1, which is in range.
#[test]
fn an_in_range_root_the_session_did_not_declare_is_refused_by_name() {
    let undeclared = Root {
        declared: 0,
        named: 1,
    };
    for entry in [Entry::Phase, Entry::Stage] {
        let submitted = submit(entry, 3, &[root(0), undeclared, root(2)], None);
        assert!(submitted.refused.is_none());
        assert_eq!(submitted.outcomes.len(), 3, "the undeclared root keeps its own slot");
        assert!(submitted.outcomes[0].is_ok());
        assert!(
            matches!(submitted.outcomes[1], Err(OperatingError::NotRead)),
            "a rule the session did not declare is refused although it is in range"
        );
        assert!(submitted.outcomes[2].is_ok());
    }
}

/// The guard is asked about the undeclared in-range root by name; its refusal ends the batch there.
#[test]
fn a_guard_refusal_at_an_undeclared_in_range_root_completes_only_the_roots_before_it() {
    let undeclared = Root {
        declared: 0,
        named: 1,
    };
    let refused = submit(Entry::Phase, 3, &[root(0), undeclared, root(2)], Some(RuleIndex(1)));
    assert!(matches!(refused.refused, Some(OperatingError::WrongBinding)));
    assert_eq!(refused.outcomes.len(), 1);
    assert!(refused.outcomes[0].is_ok());
    assert!(refused.guarded.contains(&RuleIndex(1)));
    assert!(!refused.guarded.contains(&RuleIndex(2)));
}

/// A root submitted twice is two submissions: each gets its own outcome, neither is dropped, and a duplicated undeclared root is
/// refused every time rather than once.
#[test]
fn a_duplicated_root_keeps_one_outcome_per_submission() {
    let twice = submit(Entry::Phase, 3, &[root(0), root(0)], None);
    assert!(twice.refused.is_none());
    assert_eq!(twice.outcomes.len(), 2);
    assert!(twice.outcomes.iter().all(Result::is_ok));
    assert!(twice.guarded.iter().all(|rule| *rule == RuleIndex(0)));
    assert!(!twice.guarded.is_empty());

    let undeclared = Root {
        declared: 0,
        named: 1,
    };
    let refused = submit(Entry::Phase, 3, &[root(0), undeclared, undeclared], None);
    assert_eq!(refused.outcomes.len(), 3);
    assert!(refused.outcomes[0].is_ok());
    assert!(matches!(refused.outcomes[1], Err(OperatingError::NotRead)));
    assert!(matches!(refused.outcomes[2], Err(OperatingError::NotRead)));
}

/// Both holders of roots, ordinary and named: collected delivery and the host guard callback.
#[test]
fn every_holder_of_roots_refuses_an_incomplete_extra_or_undeclared_root() {
    let extra = Root {
        declared: 0,
        named: 3,
    };
    let undeclared = Root {
        declared: 0,
        named: 1,
    };
    for entry in [Entry::Phase, Entry::Stage] {
        let complete = submit(entry, 3, &[root(0), root(1), root(2)], None);
        assert_eq!(complete.outcomes.len(), 3);
        let omitted = submit(entry, 3, &[root(0), root(2)], None);
        assert_eq!(omitted.outcomes.len(), 2);
        assert!(!omitted.guarded.contains(&RuleIndex(1)));
        let collected = submit(entry, 3, &[root(0), extra, root(2)], None);
        assert_eq!(collected.outcomes.len(), 3);
        assert!(matches!(collected.outcomes[1], Err(OperatingError::NotRun)));
        let callback = submit(entry, 3, &[root(0), extra, root(2)], Some(RuleIndex(3)));
        assert!(matches!(callback.refused, Some(OperatingError::WrongBinding)));
        assert_eq!(callback.outcomes.len(), 1);
        // In range, but the session declared rule 0: refused, never run undeclared.
        let mismatch = submit(entry, 3, &[root(0), undeclared, root(2)], None);
        assert_eq!(mismatch.outcomes.len(), 3);
        assert!(mismatch.outcomes[0].is_ok() && mismatch.outcomes[2].is_ok());
        assert!(matches!(mismatch.outcomes[1], Err(OperatingError::NotRead)));
        // A declared root submitted twice keeps one outcome per submission.
        let twice = submit(entry, 3, &[root(0), root(0)], None);
        assert_eq!(twice.outcomes.len(), 2);
        assert!(twice.outcomes.iter().all(Result::is_ok));
    }
}

#[test]
fn a_scope_belongs_to_its_own_phase() {
    assert!(matches!(foreign_phase_outcome(), Err(OperatingError::NotRead)));
}
