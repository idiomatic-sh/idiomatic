//! A project submission answers exactly the roots it names, and refuses by name the ones it did not declare. These are the runtime's
//! own contracts on the crate-private root evaluation: a caller of `evaluate_occurrences` cannot assemble such roots.
use super::fixtures::Entry;
use super::fixtures::submission::{Root, foreign_phase_outcome, submit};
use super::fixtures::{corpus, covered, prepare_for, program};
use crate::application::file_dag::frontiers::errors::PreparationError;
use crate::application::file_dag::program::project_phase::interface::ProjectStage as _;
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
use crate::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
use crate::application::file_dag::program::request_execution::service::begin;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::coverage::types::{InapplicableFiles, RefusedFiles};
use ryo_execution_domain::domain::operating::errors::OperatingError;

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
    assert_eq!(
        submitted.outcomes.len(),
        3,
        "the extra root keeps its own slot"
    );
    assert!(submitted.outcomes[0].is_ok());
    assert!(
        matches!(submitted.outcomes[1], Err(OperatingError::NotRun)),
        "a rule the program never declared is a typed refusal"
    );
    assert!(submitted.outcomes[2].is_ok());

    let guarded = submit(
        Entry::Phase,
        3,
        &[root(0), extra, root(2)],
        Some(RuleIndex(3)),
    );
    assert!(matches!(
        guarded.refused,
        Some(OperatingError::WrongBinding)
    ));
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
        assert_eq!(
            submitted.outcomes.len(),
            3,
            "the undeclared root keeps its own slot"
        );
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
    let refused = submit(
        Entry::Phase,
        3,
        &[root(0), undeclared, root(2)],
        Some(RuleIndex(1)),
    );
    assert!(matches!(
        refused.refused,
        Some(OperatingError::WrongBinding)
    ));
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
        assert!(matches!(
            callback.refused,
            Some(OperatingError::WrongBinding)
        ));
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
    assert!(matches!(
        foreign_phase_outcome(),
        Err(OperatingError::NotRead)
    ));
}

/// A scope is opened only for a rule the program declared, in the request's project phase and in the phase its observation stage
/// owns; and the phase's frontiers are prepared only for a declared rule.
#[test]
fn frontiers_and_scopes_of_an_undeclared_rule_are_refused() {
    for entry in [Entry::Phase, Entry::Stage] {
        let prepared = program(entry, 3);
        let corpus = corpus();
        let request = begin(&prepared, &corpus, None);
        let mut arenas = Vec::new();
        let check = |phase: &ProjectPhase<'_>| {
            let scope = |rule| {
                phase.project_scope(
                    rule,
                    &corpus,
                    &covered(3),
                    &[],
                    &InapplicableFiles::default(),
                    &RefusedFiles::default(),
                )
            };
            assert!(scope(RuleIndex(2)).is_ok());
            assert!(matches!(scope(RuleIndex(3)), Err(OperatingError::NotRun)));
            if matches!(entry, Entry::Stage) {
                return;
            }
            let declared = scope(RuleIndex(0)).expect("a declared rule has a scope");
            assert!(prepare_for(&request, &declared, RuleIndex(0)).is_ok());
            assert!(matches!(
                prepare_for(&request, &declared, RuleIndex(3)),
                Err(PreparationError::Metadata(OperatingError::NotRead))
            ));
        };
        match entry {
            Entry::Phase => {
                let phase = request.project_phase().expect("the phase opens");
                check(&phase);
                phase.finish().expect("the phase finishes");
            }
            Entry::Stage => {
                let stage = request.observations(&mut arenas).expect("the stage opens");
                check(&stage.phase);
                stage.finish_stage().expect("the stage finishes");
            }
        }
        request.finish();
    }
}

#[cfg(test)]
mod frontiers_test;
