//! A project submission answers exactly the roots it names, and refuses by name the ones it did not declare.
use super::super::fixtures::{Entry, Root, submit};
use crate::boundary_fixture::engine;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;

fn root(rule: usize) -> Root {
    Root {
        declared: rule,
        named: rule,
    }
}

#[test]
fn an_omitted_declared_root_gets_no_outcome_and_no_guard_and_every_submitted_root_gets_one() {
    let engine = engine();
    let all = [root(0), root(1), root(2)];
    for count in 0..=all.len() {
        let submitted = submit(&engine, Entry::Phase, 3, &all[..count], None);
        assert!(submitted.refused.is_none());
        assert_eq!(
            submitted.outcomes.len(),
            count,
            "one outcome per submitted root, none invented for the rest"
        );
        assert!(submitted.outcomes.iter().all(Result::is_ok));
    }
    let omitted = submit(&engine, Entry::Phase, 3, &[root(0), root(2)], None);
    assert_eq!(omitted.outcomes.len(), 2);
    assert!(
        !omitted.guarded.contains(&RuleIndex(1)),
        "the omitted root is never admitted, guarded or delivered"
    );
}

#[test]
fn an_extra_root_is_refused_by_name_without_displacing_the_declared_roots() {
    let engine = engine();
    let extra = Root {
        declared: 0,
        named: 3,
    };
    let submitted = submit(&engine, Entry::Phase, 3, &[root(0), extra, root(2)], None);
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
        &engine,
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
    let engine = engine();
    let undeclared = Root {
        declared: 0,
        named: 1,
    };
    for entry in [Entry::Phase, Entry::Stage] {
        let submitted = submit(&engine, entry, 3, &[root(0), undeclared, root(2)], None);
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
    let engine = engine();
    let undeclared = Root {
        declared: 0,
        named: 1,
    };
    let refused = submit(
        &engine,
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

/// A root submitted twice is two submissions: each gets its own outcome, neither is dropped,
/// and a duplicated undeclared root is refused every time rather than once.
#[test]
fn a_duplicated_root_keeps_one_outcome_per_submission() {
    let engine = engine();
    let twice = submit(&engine, Entry::Phase, 3, &[root(0), root(0)], None);
    assert!(twice.refused.is_none());
    assert_eq!(twice.outcomes.len(), 2);
    assert!(twice.outcomes.iter().all(Result::is_ok));
    assert!(twice.guarded.iter().all(|rule| *rule == RuleIndex(0)));
    assert!(!twice.guarded.is_empty());

    let undeclared = Root {
        declared: 0,
        named: 1,
    };
    let refused = submit(
        &engine,
        Entry::Phase,
        3,
        &[root(0), undeclared, undeclared],
        None,
    );
    assert_eq!(refused.outcomes.len(), 3);
    assert!(refused.outcomes[0].is_ok());
    assert!(matches!(refused.outcomes[1], Err(OperatingError::NotRead)));
    assert!(matches!(refused.outcomes[2], Err(OperatingError::NotRead)));
}
