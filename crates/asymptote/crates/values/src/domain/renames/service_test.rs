//! What the `renames` service carries: a record through the renames after its operation was added, through the
//! working tree's, and what a ratchet then persists.

use std::collections::{BTreeMap, BTreeSet};

use super::service::{changes, earliest, introduced_by, on_branch, operation_of, project, split};
use super::types::{Basis, Introduction, Parent, Rename, RenameEvent};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operations::types::{
    Count, Day, OperationId, Provenance, Reason, Record, Revision,
};
use crate::domain::violations::types::{
    Fingerprint, Id, Key, Level, MessageText, RepoPath, RuleId, Standing, ToolId,
};

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// A rename committed after the record's operation was added carries it.
    Committed,
    /// A rename committed before it does not.
    Before,
    /// A rename in the working tree carries it.
    Worktree,
    /// The union of two branches' records keeps the larger count and both reasons.
    Branch,
    /// A carried projection is added at the new path and removed at the old; an unmatched one only removed.
    Persisted,
    /// A file that is no operation names none.
    NoOperation,
}

type Input = Question;
type Value = std::borrow::Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rename_after_the_acceptance_carries_the_record"),
        word: Word::Succeeds,
        input: || Question::Committed,
        expect: Expect::Value(std::borrow::Cow::Borrowed("b.rs")),
    },
    Case {
        name: CaseName("a_rename_before_the_acceptance_does_not"),
        word: Word::Succeeds,
        input: || Question::Before,
        expect: Expect::Value(std::borrow::Cow::Borrowed("a.rs")),
    },
    Case {
        name: CaseName("a_rename_in_the_working_tree_carries_the_record"),
        word: Word::Succeeds,
        input: || Question::Worktree,
        expect: Expect::Value(std::borrow::Cow::Borrowed("c.rs")),
    },
    Case {
        name: CaseName("a_branch_union_keeps_the_larger_count_and_both_reasons"),
        word: Word::Succeeds,
        input: || Question::Branch,
        expect: Expect::Value(std::borrow::Cow::Borrowed("2:one|two")),
    },
    Case {
        name: CaseName("a_carried_projection_moves_and_an_unmatched_one_is_dropped"),
        word: Word::Succeeds,
        input: || Question::Persisted,
        expect: Expect::Value(std::borrow::Cow::Borrowed("+b.rs -a.rs -x.rs")),
    },
    Case {
        name: CaseName("a_file_whose_stem_is_no_digest_names_no_operation"),
        word: Word::Refuses,
        input: || Question::NoOperation,
        expect: Expect::error(()),
    },
];

fn record(path: &str, count: u32, reason: &str) -> Record {
    Record {
        id: Id(path.into()),
        key: Key {
            tool: ToolId("t".into()),
            rule: RuleId("r".into()),
            path: RepoPath(path.into()),
            level: Level::Error,
            standing: Standing::Reported,
            fingerprint: Fingerprint(1),
        },
        count: Count(count),
        message: MessageText("m".into()),
        since_rev: Provenance::Unknown,
        since: Day("2026-10-19".into()),
        reasons: vec![Reason(reason.into())],
    }
}

fn revision(text: &str) -> Revision {
    Revision(text.into())
}

fn rename(from: &str, to: &str) -> Rename {
    Rename {
        from: RepoPath(from.into()),
        to: RepoPath(to.into()),
    }
}

/// An outcome in which every projection is wholly resolved, as nothing reports it.
fn unused(
    moved: &[crate::domain::renames::types::Migration],
) -> crate::domain::judgement::types::Judgement {
    crate::domain::judgement::types::Judgement {
        resolved: moved
            .iter()
            .map(|migration| crate::domain::judgement::types::Resolved {
                record: migration.projected.clone(),
                count: Count(1),
                remaining: Count(0),
                resolution: crate::domain::judgement::types::Resolution::NotReported,
            })
            .collect(),
        ..crate::domain::judgement::types::Judgement::default()
    }
}

fn id_of(key: &Key) -> Id {
    Id(key.path.0.clone())
}

fn told(question: Input) -> Result<String, Failure> {
    let digest = "a".repeat(64);
    let operation = OperationId(digest.clone().into());
    let introductions = vec![Introduction {
        path: RepoPath(format!(".asymptote/operations/{digest}.lock").into()),
        revision: revision("c2"),
        parent: Parent::Commit(revision("c1")),
    }];
    let introduced = introduced_by(&introductions);
    let origins = BTreeMap::from([(Id("a.rs".into()), vec![operation])]);
    let found = earliest(&introductions, &origins, &introduced).is_some();
    let basis = Basis::Operations {
        origins: &origins,
        introduced: &introduced,
    };
    let event = |name: &str, renames| RenameEvent {
        revision: revision(name),
        renames,
    };
    let carried_to = |history: &[RenameEvent], worktree: &[Rename]| {
        let projection = project(
            vec![record("a.rs", 1, "one")],
            &revision("head"),
            history,
            worktree,
            basis,
            &id_of,
        );
        projection
            .records
            .first()
            .map(|record| record.key.path.0.to_string())
            .unwrap_or_default()
    };
    Ok(match question {
        Question::Committed if found => carried_to(
            &[
                event("c2", Vec::new()),
                event("c3", vec![rename("a.rs", "b.rs")]),
            ],
            &[],
        ),
        Question::Before => carried_to(
            &[
                event("c0", vec![rename("a.rs", "b.rs")]),
                event("c2", Vec::new()),
            ],
            &[],
        ),
        Question::Worktree => carried_to(&[], &[rename("a.rs", "c.rs")]),
        Question::Branch => {
            let united = on_branch(
                vec![record("a.rs", 1, "one")],
                vec![record("a.rs", 2, "two")],
            );
            let first = united.first().ok_or(())?;
            let reasons = first
                .reasons
                .iter()
                .map(|reason| &*reason.0)
                .collect::<Vec<_>>();
            format!("{}:{}", first.count.0, reasons.join("|"))
        }
        Question::Persisted => {
            let moved = project(
                vec![record("a.rs", 1, "one"), record("x.rs", 1, "one")],
                &revision("head"),
                &[],
                &[rename("a.rs", "b.rs"), rename("x.rs", "y.rs")],
                Basis::Unknown,
                &id_of,
            )
            .migrations;
            let observed: BTreeSet<Key> = moved
                .iter()
                .filter(|migration| migration.projected.key.path.0.as_ref() == "b.rs")
                .map(|migration| migration.projected.key.clone())
                .collect();
            let persisted = changes(
                &split(&moved, &observed, &unused(&moved)),
                Vec::new(),
                Vec::new(),
            );
            let added = persisted
                .added
                .iter()
                .map(|record| format!("+{}", record.key.path.0));
            let removed = persisted
                .removed
                .iter()
                .map(|record| format!("-{}", record.key.path.0));
            added.chain(removed).collect::<Vec<_>>().join(" ")
        }
        Question::NoOperation => {
            return operation_of(&RepoPath("a/README.md".into()))
                .map(|_| String::new())
                .ok_or(());
        }
        Question::Committed => String::new(),
    })
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
