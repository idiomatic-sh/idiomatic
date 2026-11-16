//! What the `operations` fold decides: a descendant wins, a concurrent removal wins, concurrent acceptances keep
//! both reasons and the stricter count, a snapshot beside a concurrent operation keeps what that one decided, and a
//! broken graph is refused.

use std::collections::BTreeMap;

use super::errors::OperationsError;
use super::fixtures::{
    coverage_across_snapshots, drained, flickering, id, kept, operation, other,
    perf_removal_across_snapshot, record, removal_across_snapshot, since,
};
use super::service::{fold, shape, snapshot};
use super::types::{Operation, OperationId, Shape};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The graph to fold.
#[derive(Clone, Copy, Debug)]
enum Graph {
    /// One accepts, a child removes.
    Descendant,
    /// Two concurrent: one accepts, the other removes.
    ConcurrentRemoval,
    /// Two concurrent acceptances of one record, counts 2 and 1.
    ConcurrentAcceptances,
    /// A child names a parent that is not there.
    MissingParent,
    /// Two operations name each other.
    Cycle,
    /// A record with a count of zero.
    ZeroCount,
    /// A snapshot of a folded state.
    Snapshot,
    /// An operation that decides one record twice.
    Twice,
    /// A snapshot written beside an operation another branch appended concurrently, which removes the record the
    /// snapshot carries and accepts another.
    SnapshotBesideConcurrent,
    /// A chain of 70 operations each accepting one record, a snapshot after them, then 64 more: whether the lock is
    /// due a snapshot before it, after it, after 62 more and after the 64th.
    SinceCompaction,
    /// One accepts a record, a child removes it, and another branch accepts it from the first: the verdict uncompacted
    /// and after a snapshot replaced the two.
    RemovalAcrossSnapshot,
    /// Two snapshots in a row, then an operation whose parent the first snapshot covered.
    CoverageAcrossSnapshots,
    /// 70 operations that accept a record and remove it again: whether the lock is due a snapshot, and after one.
    Drained,
    /// One record whose count alternates over 70 operations: whether the lock is due a snapshot, and after one.
    Flickering,
    /// A perf record set, removed, and set again from the first by a concurrent branch: the verdict uncompacted and after a
    /// snapshot replaced the first two.
    PerfRemovalAcrossSnapshot,
}

/// What the fold kept: each record as `id:count:reasons`, joined.
type Input = Graph;
type Value = std::borrow::Cow<'static, str>;
type Failure = OperationsError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_descendant_removal_supersedes_the_acceptance"),
        word: Word::Succeeds,
        input: || Graph::Descendant,
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
    Case {
        name: CaseName("a_concurrent_removal_wins"),
        word: Word::Succeeds,
        input: || Graph::ConcurrentRemoval,
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
    Case {
        name: CaseName("concurrent_acceptances_keep_both_reasons_and_the_stricter_count"),
        word: Word::Succeeds,
        input: || Graph::ConcurrentAcceptances,
        expect: Expect::Value(std::borrow::Cow::Borrowed("a:1:one|two")),
    },
    Case {
        name: CaseName("a_snapshot_folds_to_its_records_and_is_compacted"),
        word: Word::Succeeds,
        input: || Graph::Snapshot,
        expect: Expect::Value(std::borrow::Cow::Borrowed("a:1:one|compacted")),
    },
    Case {
        name: CaseName(
            "a_snapshot_beside_a_concurrent_operation_keeps_what_the_concurrent_one_decided",
        ),
        word: Word::Succeeds,
        input: || Graph::SnapshotBesideConcurrent,
        expect: Expect::Value(std::borrow::Cow::Borrowed("b:1:two|open")),
    },
    Case {
        name: CaseName("a_snapshot_settles_the_history_before_it_so_compaction_is_bounded"),
        word: Word::Succeeds,
        input: || Graph::SinceCompaction,
        expect: Expect::Value(std::borrow::Cow::Borrowed("open compacted compacted open")),
    },
    Case {
        name: CaseName("a_removal_concurrent_with_a_branch_wins_uncompacted_and_after_a_snapshot"),
        word: Word::Succeeds,
        input: || Graph::RemovalAcrossSnapshot,
        expect: Expect::Value(std::borrow::Cow::Borrowed("uncompacted: compacted:")),
    },
    Case {
        name: CaseName("a_parent_the_first_of_two_snapshots_covered_is_still_covered"),
        word: Word::Succeeds,
        input: || Graph::CoverageAcrossSnapshots,
        expect: Expect::Value(std::borrow::Cow::Borrowed("a:1:late|one")),
    },
    Case {
        name: CaseName("a_lock_whose_records_were_all_removed_is_compacted_at_the_bound"),
        word: Word::Succeeds,
        input: || Graph::Drained,
        expect: Expect::Value(std::borrow::Cow::Borrowed("open open compacted")),
    },
    Case {
        name: CaseName("a_record_that_changes_with_every_operation_is_compacted_at_the_bound"),
        word: Word::Succeeds,
        input: || Graph::Flickering,
        expect: Expect::Value(std::borrow::Cow::Borrowed("open open compacted compacted")),
    },
    Case {
        name: CaseName(
            "a_perf_removal_concurrent_with_a_branch_wins_uncompacted_and_after_a_snapshot",
        ),
        word: Word::Succeeds,
        input: || Graph::PerfRemovalAcrossSnapshot,
        expect: Expect::Value(std::borrow::Cow::Borrowed("uncompacted: compacted:")),
    },
    Case {
        name: CaseName("a_missing_parent_is_refused"),
        word: Word::Refuses,
        input: || Graph::MissingParent,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(OperationsError::MissingParent {
                    operation: "1".into(),
                    parent: "9".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_cycle_is_refused"),
        word: Word::Refuses,
        input: || Graph::Cycle,
        expect: Expect::error(OperationsError::Cycle),
    },
    Case {
        name: CaseName("a_record_decided_twice_by_one_operation_is_refused"),
        word: Word::Refuses,
        input: || Graph::Twice,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(OperationsError::Twice {
                    operation: "1".into(),
                    record: "a".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_record_of_count_zero_is_refused"),
        word: Word::Refuses,
        input: || Graph::ZeroCount,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(OperationsError::Invalid {
                    operation: "1".into(),
                    record: "a".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

fn graph(shape: Graph) -> BTreeMap<OperationId, Operation> {
    let entries = match shape {
        Graph::Descendant => vec![
            ("1", operation(&[], vec![record(1, "one")], Vec::new())),
            ("2", operation(&["1"], Vec::new(), vec![record(1, "one")])),
        ],
        Graph::ConcurrentRemoval => vec![
            ("0", operation(&[], vec![record(1, "one")], Vec::new())),
            ("1", operation(&["0"], vec![record(1, "two")], Vec::new())),
            ("2", operation(&["0"], Vec::new(), vec![record(1, "one")])),
        ],
        Graph::ConcurrentAcceptances | Graph::Snapshot => vec![
            ("1", operation(&[], vec![record(2, "one")], Vec::new())),
            ("2", operation(&[], vec![record(1, "two")], Vec::new())),
        ],
        Graph::MissingParent => vec![("1", operation(&["9"], Vec::new(), Vec::new()))],
        Graph::Cycle => vec![
            ("1", operation(&["2"], Vec::new(), Vec::new())),
            ("2", operation(&["1"], Vec::new(), Vec::new())),
        ],
        Graph::ZeroCount => vec![("1", operation(&[], vec![record(0, "one")], Vec::new()))],
        Graph::SnapshotBesideConcurrent => vec![
            ("1", operation(&[], vec![record(2, "one")], Vec::new())),
            (
                "2",
                operation(&["1"], vec![other(1, "two")], vec![record(2, "one")]),
            ),
        ],
        Graph::SinceCompaction
        | Graph::RemovalAcrossSnapshot
        | Graph::CoverageAcrossSnapshots
        | Graph::Drained
        | Graph::Flickering
        | Graph::PerfRemovalAcrossSnapshot => Vec::new(),
        Graph::Twice => vec![(
            "1",
            operation(&[], vec![record(1, "one")], vec![record(1, "one")]),
        )],
    };
    entries
        .into_iter()
        .map(|(name, operation)| (id(name), operation))
        .collect()
}

fn told(form: Input) -> Result<String, Failure> {
    let mut operations = graph(form);
    let own = match form {
        Graph::RemovalAcrossSnapshot => Some(removal_across_snapshot()),
        Graph::CoverageAcrossSnapshots => Some(coverage_across_snapshots()),
        Graph::Drained => Some(drained()),
        Graph::Flickering => Some(flickering()),
        Graph::PerfRemovalAcrossSnapshot => Some(perf_removal_across_snapshot()),
        Graph::Descendant
        | Graph::ConcurrentRemoval
        | Graph::ConcurrentAcceptances
        | Graph::MissingParent
        | Graph::Cycle
        | Graph::ZeroCount
        | Graph::Snapshot
        | Graph::Twice
        | Graph::SnapshotBesideConcurrent
        | Graph::SinceCompaction => None,
    };
    if let Some(answer) = own {
        return answer;
    }
    if let Graph::SinceCompaction = form {
        return since(&mut operations);
    }
    if let Graph::SnapshotBesideConcurrent = form {
        // The snapshot is of operation 1 alone: the branch that wrote it had not seen operation 2.
        let before: BTreeMap<OperationId, Operation> = operations
            .iter()
            .filter(|(name, _)| name.0.as_ref() == "1")
            .map(|(name, operation)| (name.clone(), operation.clone()))
            .collect();
        let folded = fold(&before)?;
        // The snapshot replaces operation 1, whose file it deletes: operation 2 names a parent it covers.
        let written = snapshot(&before, &folded, &folded.records);
        operations.retain(|name, _| !written.covers.contains(name));
        operations.extend([(id("3"), written)]);
        let after = fold(&operations)?;
        let state = if shape(&operations, &after) == Shape::Compacted {
            "compacted"
        } else {
            "open"
        };
        return Ok(format!("{}|{state}", kept(&after)));
    }
    let folded = fold(&operations)?;
    if let Graph::Snapshot = form {
        let carried = vec![record(1, "one")];
        // The snapshot replaces every operation, whose files it deletes.
        let written = snapshot(&operations, &folded, &carried);
        operations.retain(|name, _| !written.covers.contains(name));
        operations.extend([(id("3"), written)]);
        let after = fold(&operations)?;
        let state = if shape(&operations, &after) == Shape::Compacted {
            "compacted"
        } else {
            "open"
        };
        return Ok(format!("{}|{state}", kept(&after)));
    }
    Ok(kept(&folded))
}

fn subject(input: Input) -> Result<Value, Failure> {
    told(input).map(std::borrow::Cow::Owned)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
