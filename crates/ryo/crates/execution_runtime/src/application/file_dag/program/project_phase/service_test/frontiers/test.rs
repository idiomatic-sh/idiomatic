//! The project phase's frontiers and demands: frontiers of another request or coverage are refused, a root answers only what its rule demands, and a scope and its phase finish together.
use super::super::super::fixtures::{
    Entry, corpus, covered, prepare_for, producer_program, program,
};
use super::fixtures::{DIAGNOSTIC, evaluate_one};
use crate::application::file_dag::program::project_phase::interface::ProgramProjectPhasePort as _;
use crate::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
use crate::application::file_dag::program::request_execution::service::begin;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::{
    Covered, InapplicableFiles, RefusedFiles, RuleIndex,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::{EmitDemand, ObservationDemand, RuleDemand};
use ryo_language::domain::symbols::types::Symbols;

/// Frontiers one request prepared are not admitted by another request's phase over the same program and corpus
/// (`PreparedFrontiers::validate_scope`).
#[test]
fn frontiers_from_another_request_are_refused() {
    let prepared = program(Entry::Phase, 1);
    let corpus = corpus();
    let covered = covered(1);
    let first = crate::application::file_dag::program::request_execution::service::begin(
        &prepared, &corpus, None,
    );
    let second = crate::application::file_dag::program::request_execution::service::begin(
        &prepared, &corpus, None,
    );
    let (owner, other) = (
        first
            .project_phase()
            .expect("the first request's phase opens"),
        second
            .project_phase()
            .expect("the second request's phase opens"),
    );
    let owned = owner
        .project_scope(
            RuleIndex(0),
            &corpus,
            &covered,
            &[],
            &InapplicableFiles::default(),
            &RefusedFiles::default(),
        )
        .expect("a declared rule has a scope");
    let frontiers =
        prepare_for(&first, &owned, RuleIndex(0)).expect("the first request's frontiers");
    let scope = other
        .project_scope(
            RuleIndex(0),
            &corpus,
            &covered,
            &[],
            &InapplicableFiles::default(),
            &RefusedFiles::default(),
        )
        .expect("a declared rule has a scope");
    assert!(
        matches!(
            evaluate_one(&other, RuleIndex(0), DIAGNOSTIC, &scope, frontiers),
            Err(OperatingError::NotRead)
        ),
        "an unrelated request cannot admit the original frontiers"
    );
    drop((owned, scope));
    owner.finish().expect("the first phase finishes");
    other.finish().expect("the second phase finishes");
    first.finish();
    second.finish();
}

/// Frontiers prepared over one coverage are not admitted beside a scope over another (`PreparedFrontiers::validate_scope`).
#[test]
fn frontiers_over_another_coverage_are_refused() {
    let prepared = program(Entry::Phase, 2);
    let corpus = corpus();
    let request = crate::application::file_dag::program::request_execution::service::begin(
        &prepared, &corpus, None,
    );
    let phase = request.project_phase().expect("the phase opens");
    let old = phase
        .project_scope(
            RuleIndex(1),
            &corpus,
            &[],
            &[],
            &InapplicableFiles::default(),
            &RefusedFiles::default(),
        )
        .expect("a declared rule has a scope");
    let frontiers =
        prepare_for(&request, &old, RuleIndex(1)).expect("frontiers over the old coverage");
    let changed = [Covered {
        file: ryo_relations::domain::relations::types::FileId(0),
        rules: vec![RuleIndex(0)],
        partial: vec![],
    }];
    let scope = phase
        .project_scope(
            RuleIndex(1),
            &corpus,
            &changed,
            &[],
            &InapplicableFiles::default(),
            &RefusedFiles::default(),
        )
        .expect("a declared rule has a scope");
    assert!(
        matches!(
            evaluate_one(&phase, RuleIndex(1), DIAGNOSTIC, &scope, frontiers),
            Err(OperatingError::NotRead)
        ),
        "a changed physical coverage declaration cannot consume frontiers prepared over another"
    );
    drop((old, scope));
    phase.finish().expect("the phase finishes");
    request.finish();
}

/// A root whose emit is not demanded answers `NoQuery` for it. The project phase's one call always demands the emit, so this is
/// the only proof of the arm; execution's producer cases (`consumers`, `union`) name this test beside their weaker check.
#[test]
fn a_root_whose_emit_is_not_demanded_answers_no_query() {
    let prepared = program(Entry::Phase, 1);
    let corpus = corpus();
    let request = crate::application::file_dag::program::request_execution::service::begin(
        &prepared, &corpus, None,
    );
    let phase = request.project_phase().expect("the phase opens");
    let scope = phase
        .project_scope(
            RuleIndex(0),
            &corpus,
            &covered(1),
            &[],
            &InapplicableFiles::default(),
            &RefusedFiles::default(),
        )
        .expect("a declared rule has a scope");
    let frontiers = prepare_for(&request, &scope, RuleIndex(0)).expect("declared frontiers");
    let silent = RuleDemand {
        emit: EmitDemand::Disabled,
        observations: ObservationDemand::NotRequired,
    };
    let outcome = evaluate_one(&phase, RuleIndex(0), silent, &scope, frontiers)
        .expect("the root is evaluated");
    assert!(matches!(outcome.emit, RootOutcome::NoQuery));
    assert!(outcome.observations.is_empty());
    drop(scope);
    phase.finish().expect("the phase finishes");
    request.finish();
}

/// A root whose observations are not demanded invokes none of its observation roots, and its outcome publishes nothing. Preparation
/// reads the producer's project relation through empty facts, so nothing refuses the observation root's read: the proof is the
/// outcome's empty observations, which a `Required` demand would fill with one outcome per cross-file observation root.
#[test]
fn a_root_whose_observations_are_not_required_invokes_no_observation_root() {
    let prepared = producer_program();
    let corpus = corpus();
    let request = crate::application::file_dag::program::request_execution::service::begin(
        &prepared, &corpus, None,
    );
    let phase = request.project_phase().expect("the phase opens");
    let scope = phase
        .project_scope(
            RuleIndex(0),
            &corpus,
            &covered(1),
            &[],
            &InapplicableFiles::default(),
            &RefusedFiles::default(),
        )
        .expect("a declared rule has a scope");
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
        .expect("declared frontiers");
    let emit_only = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    let outcome = evaluate_one(&phase, RuleIndex(0), emit_only, &scope, frontiers)
        .expect("the root is evaluated");
    assert!(outcome.observations.is_empty(), "no observation root ran");
    assert!(!matches!(
        outcome.emit,
        RootOutcome::Failed(_) | RootOutcome::Unknown(_)
    ));
    drop(scope);
    phase.finish().expect("the phase finishes");
    request.finish();
}

/// Each phase a request opens owns its own publication: a scope one phase still holds, its root evaluated, makes that phase's
/// finish refuse, and leaves another phase of the same request free to finish.
#[test]
fn finish_refuses_while_a_scope_of_its_own_phase_shares_the_publication() {
    let prepared = program(Entry::Phase, 3);
    let corpus = corpus();
    let request = begin(&prepared, &corpus, None);
    let (held, other) = (
        request.project_phase().expect("a phase opens"),
        request.project_phase().expect("a phase opens"),
    );
    let scope = held
        .project_scope(
            RuleIndex(0),
            &corpus,
            &covered(3),
            &[],
            &InapplicableFiles::default(),
            &RefusedFiles::default(),
        )
        .expect("a declared rule has a scope");
    let frontiers = prepare_for(&request, &scope, RuleIndex(0)).expect("declared frontiers");
    evaluate_one(&held, RuleIndex(0), DIAGNOSTIC, &scope, frontiers)
        .expect("the root is evaluated");
    assert!(
        other.finish().is_ok(),
        "another phase's scope does not hold this phase's publication"
    );
    assert!(
        matches!(held.finish(), Err(OperatingError::NotRead)),
        "an outstanding scope keeps its own phase's publication shared, so finish reports the refusal"
    );
    drop(scope);
    request.finish();
}
