//! The project phase's frontiers and demands: frontiers of another request or coverage are refused, a root answers only what its rule demands, and a scope and its phase finish together.
use super::super::super::fixtures::{
    Entry, corpus, covered, prepare_for, producer_program, program,
};
use super::fixtures::{DIAGNOSTIC, evaluate_one};
use crate::application::file_dag::program::project_phase::interface::ProgramProjectPhasePort as _;
use crate::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
use crate::application::file_dag::program::request_execution::service::begin;
use crate::application::file_dag::program::rule_outcome::types::{
    ProjectOutputOutcome, RootOutcome, RuleOutcome,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::{
    Covered, InapplicableFiles, RefusedFiles, RuleIndex,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::{EmitDemand, ObservationDemand, RuleDemand};
use ryo_language::domain::symbols::types::Symbols;
use std::borrow::Cow;

/// Frontiers one request prepared are not admitted by another request's phase over the same program and corpus
/// (`PreparedFrontiers::validate_scope`).
fn frontiers_from_another_request_are_refused() -> Result<Phased, OperatingError> {
    let prepared = program(Entry::Phase, 1);
    let corpus = corpus();
    let covered = covered(1);
    let first = begin(&prepared, &corpus, None);
    let second = begin(&prepared, &corpus, None);
    let (owner, other) = (first.project_phase()?, second.project_phase()?);
    let owned = owner.project_scope(
        RuleIndex(0),
        &corpus,
        &covered,
        &[],
        &InapplicableFiles::default(),
        &RefusedFiles::default(),
    )?;
    let frontiers = prepare_for(&first, &owned, RuleIndex(0)).expect("the declared frontiers");
    let scope = other.project_scope(
        RuleIndex(0),
        &corpus,
        &covered,
        &[],
        &InapplicableFiles::default(),
        &RefusedFiles::default(),
    )?;
    // An unrelated request cannot admit the original frontiers.
    let evaluated = Evaluated::from(evaluate_one(
        &other,
        RuleIndex(0),
        DIAGNOSTIC,
        &scope,
        frontiers,
    ));
    drop((owned, scope));
    let finishes = vec![owner.finish().err(), other.finish().err()];
    first.finish();
    second.finish();
    Ok(phased(evaluated, finishes))
}

/// Frontiers prepared over one coverage are not admitted beside a scope over another (`PreparedFrontiers::validate_scope`).
fn frontiers_over_another_coverage_are_refused() -> Result<Phased, OperatingError> {
    let prepared = program(Entry::Phase, 2);
    let corpus = corpus();
    let request = begin(&prepared, &corpus, None);
    let phase = request.project_phase()?;
    let old = phase.project_scope(
        RuleIndex(1),
        &corpus,
        &[],
        &[],
        &InapplicableFiles::default(),
        &RefusedFiles::default(),
    )?;
    let frontiers = prepare_for(&request, &old, RuleIndex(1)).expect("the declared frontiers");
    let changed = [Covered {
        file: ryo_relations::domain::relations::types::FileId(0),
        rules: vec![RuleIndex(0)],
        partial: vec![],
    }];
    let scope = phase.project_scope(
        RuleIndex(1),
        &corpus,
        &changed,
        &[],
        &InapplicableFiles::default(),
        &RefusedFiles::default(),
    )?;
    // A changed physical coverage declaration cannot consume frontiers prepared over another.
    let evaluated = Evaluated::from(evaluate_one(
        &phase,
        RuleIndex(1),
        DIAGNOSTIC,
        &scope,
        frontiers,
    ));
    drop((old, scope));
    let finishes = vec![phase.finish().err()];
    request.finish();
    Ok(phased(evaluated, finishes))
}

/// A root whose emit is not demanded answers `NoQuery` for it. The project phase's one call always demands the emit, so this is
/// the only proof of the arm; execution's producer cases (`consumers`, `union`) name this test beside their weaker check.
fn a_root_whose_emit_is_not_demanded_answers_no_query() -> Result<Phased, OperatingError> {
    let prepared = program(Entry::Phase, 1);
    let corpus = corpus();
    let request = begin(&prepared, &corpus, None);
    let phase = request.project_phase()?;
    let scope = phase.project_scope(
        RuleIndex(0),
        &corpus,
        &covered(1),
        &[],
        &InapplicableFiles::default(),
        &RefusedFiles::default(),
    )?;
    let frontiers = prepare_for(&request, &scope, RuleIndex(0)).expect("the declared frontiers");
    let silent = RuleDemand {
        emit: EmitDemand::Disabled,
        observations: ObservationDemand::NotRequired,
    };
    let evaluated = Evaluated::from(evaluate_one(
        &phase,
        RuleIndex(0),
        silent,
        &scope,
        frontiers,
    ));
    drop(scope);
    let finishes = vec![phase.finish().err()];
    request.finish();
    Ok(phased(evaluated, finishes))
}

/// A root whose observations are not demanded invokes none of its observation roots, and its outcome publishes nothing. Preparation
/// reads the producer's project relation through empty facts, so nothing refuses the observation root's read: the proof is the
/// outcome's empty observations, which a `Required` demand would fill with one outcome per cross-file observation root.
fn a_root_whose_observations_are_not_required_invokes_no_observation_root()
-> Result<Phased, OperatingError> {
    let prepared = producer_program();
    let corpus = corpus();
    let request = begin(&prepared, &corpus, None);
    let phase = request.project_phase()?;
    let scope = phase.project_scope(
        RuleIndex(0),
        &corpus,
        &covered(1),
        &[],
        &InapplicableFiles::default(),
        &RefusedFiles::default(),
    )?;
    // The exported observation's source is a frontier input, so preparation reads its project relation: no rows here.
    let project = crate::outbound::facts::interface::FactsPort::worker(
        &crate::outbound::facts::fixtures::fake::FactsFake::default(),
    )
    .expect("a fake project worker");
    let frontiers = request
        .prepare_frontiers(
            crate::application::file_dag::program::project_phase::types::FrontierRequest {
                demand: crate::application::file_dag::program::project_phase::types::FrontierDemand::Diagnostic,
                scope: &scope,
                rule: RuleIndex(0),
                first: ryo_values::domain::stored_facts::types::Bucket(0),
                facts: &super::super::super::fixtures::NoFacts,
                project: project.as_ref(),
                project_symbols: &Symbols::default(),
            },
            &mut Symbols::default(),
        )
        .expect("the declared frontiers");
    let emit_only = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    let evaluated = Evaluated::from(evaluate_one(
        &phase,
        RuleIndex(0),
        emit_only,
        &scope,
        frontiers,
    ));
    drop(scope);
    let finishes = vec![phase.finish().err()];
    request.finish();
    Ok(phased(evaluated, finishes))
}

/// Each phase a request opens owns its own publication: a scope one phase still holds, its root evaluated, makes that phase's
/// finish refuse, and leaves another phase of the same request free to finish.
fn finish_refuses_while_a_scope_of_its_own_phase_shares_the_publication()
-> Result<Phased, OperatingError> {
    let prepared = program(Entry::Phase, 3);
    let corpus = corpus();
    let request = begin(&prepared, &corpus, None);
    let (held, other) = (request.project_phase()?, request.project_phase()?);
    let scope = held.project_scope(
        RuleIndex(0),
        &corpus,
        &covered(3),
        &[],
        &InapplicableFiles::default(),
        &RefusedFiles::default(),
    )?;
    let frontiers = prepare_for(&request, &scope, RuleIndex(0)).expect("the declared frontiers");
    let evaluated = Evaluated::from(evaluate_one(
        &held,
        RuleIndex(0),
        DIAGNOSTIC,
        &scope,
        frontiers,
    ));
    // Another phase's scope does not hold this phase's publication; an outstanding scope keeps
    // its own phase's publication shared, so its finish reports the refusal.
    let finishes = vec![other.finish().err(), held.finish().err()];
    drop(scope);
    request.finish();
    Ok(phased(evaluated, finishes))
}

/// What the evaluated root answered: its emit and the observation outcomes it published, or the
/// refusal of its evaluation.
#[derive(Debug, PartialEq)]
enum Evaluated {
    Root { emit: Emit, observations: usize },
    Refused(OperatingError),
}

/// The kind of a root's emit, with its rows or error.
#[derive(Debug, PartialEq)]
enum Emit {
    Complete(usize),
    Partial(usize),
    Skipped,
    Unknown(OperatingError),
    Failed(OperatingError),
    NoQuery,
    Deferred,
}

impl From<Result<RuleOutcome<ProjectOutputOutcome>, OperatingError>> for Evaluated {
    fn from(evaluated: Result<RuleOutcome<ProjectOutputOutcome>, OperatingError>) -> Self {
        evaluated.map_or_else(Self::Refused, |outcome| Self::Root {
            emit: match outcome.emit {
                RootOutcome::Complete(table) => Emit::Complete(table.count.0),
                RootOutcome::Partial { table, .. } => Emit::Partial(table.count.0),
                RootOutcome::Skipped(_) => Emit::Skipped,
                RootOutcome::Unknown(error) => Emit::Unknown(error),
                RootOutcome::Failed(error) => Emit::Failed(error),
                RootOutcome::NoQuery => Emit::NoQuery,
                RootOutcome::Deferred { .. } => Emit::Deferred,
            },
            observations: outcome.observations.len(),
        })
    }
}

/// What a row's phases observed: the evaluation, and each phase's finish refusal in finishing order.
#[derive(Debug, PartialEq)]
struct Phased {
    evaluated: Evaluated,
    finishes: Cow<'static, [Option<OperatingError>]>,
}

fn phased(evaluated: Evaluated, finishes: Vec<Option<OperatingError>>) -> Phased {
    Phased {
        evaluated,
        finishes: Cow::Owned(finishes),
    }
}

/// What a row opens and evaluates.
#[derive(Clone, Copy, Debug)]
enum Frontiers {
    /// Frontiers one request prepared, evaluated in another request's phase.
    OtherRequest,
    /// Frontiers prepared over one coverage, evaluated beside a scope over another.
    OtherCoverage,
    /// A root evaluated without its emit demanded.
    EmitUndemanded,
    /// A producer root evaluated without its observations required.
    ObservationsUnrequired,
    /// A root evaluated in one of two phases, finished while its scope is still held.
    HeldScope,
}

type Input = Frontiers;
type Value = Phased;
type Failure = OperatingError;

const fn evaluated(evaluated: Evaluated, finishes: &'static [Option<OperatingError>]) -> Phased {
    Phased {
        evaluated,
        finishes: Cow::Borrowed(finishes),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("frontiers_from_another_request_are_refused"),
        word: Word::Succeeds,
        input: || Frontiers::OtherRequest,
        expect: Expect::Value(evaluated(
            Evaluated::Refused(OperatingError::NotRead),
            &[None, None],
        )),
    },
    Case {
        name: CaseName("frontiers_over_another_coverage_are_refused"),
        word: Word::Succeeds,
        input: || Frontiers::OtherCoverage,
        expect: Expect::Value(evaluated(
            Evaluated::Refused(OperatingError::NotRead),
            &[None],
        )),
    },
    Case {
        name: CaseName("a_root_whose_emit_is_not_demanded_answers_no_query"),
        word: Word::Succeeds,
        input: || Frontiers::EmitUndemanded,
        expect: Expect::Value(evaluated(
            Evaluated::Root {
                emit: Emit::NoQuery,
                observations: 0,
            },
            &[None],
        )),
    },
    Case {
        name: CaseName("a_root_whose_observations_are_not_required_invokes_no_observation_root"),
        word: Word::Succeeds,
        input: || Frontiers::ObservationsUnrequired,
        expect: Expect::Value(evaluated(
            Evaluated::Root {
                emit: Emit::Deferred,
                observations: 0,
            },
            &[None],
        )),
    },
    Case {
        name: CaseName("finish_refuses_while_a_scope_of_its_own_phase_shares_the_publication"),
        word: Word::Succeeds,
        input: || Frontiers::HeldScope,
        expect: Expect::Value(evaluated(
            Evaluated::Root {
                emit: Emit::Deferred,
                observations: 0,
            },
            &[None, Some(OperatingError::NotRead)],
        )),
    },
];

/// Opens the row's request phases and scopes, evaluates its root, and finishes the phases.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Frontiers::OtherRequest => frontiers_from_another_request_are_refused(),
        Frontiers::OtherCoverage => frontiers_over_another_coverage_are_refused(),
        Frontiers::EmitUndemanded => a_root_whose_emit_is_not_demanded_answers_no_query(),
        Frontiers::ObservationsUnrequired => {
            a_root_whose_observations_are_not_required_invokes_no_observation_root()
        }
        Frontiers::HeldScope => {
            finish_refuses_while_a_scope_of_its_own_phase_shares_the_publication()
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
