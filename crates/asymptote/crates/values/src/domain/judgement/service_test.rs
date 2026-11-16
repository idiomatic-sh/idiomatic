//! What the `judgement` service makes of a run: new by the surplus, resolved when the rule was evaluated or retired,
//! unjudged otherwise, and the verdict a failed tool outranks.

use std::collections::BTreeSet;

use super::service::{compare, lowered, verdict};
use super::types::{Failure as Ended, Inventory, Problem, Run, Status};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operations::types::{Count, Day, Provenance, Reason, Record};
use crate::domain::violations::types::{
    Fingerprint, Fingerprinted, Id, Key, Level, MessageText, Place, RepoPath, RuleId, Standing,
    Suppression, ToolId, Violation,
};

/// What the run saw, against one record of count 2 for rule `r`.
#[derive(Clone, Copy, Debug)]
enum Scene {
    /// Three of the record's key: one is new.
    Surplus,
    /// One of the key, the rule evaluated: one resolved, lowered to one.
    Partly,
    /// None, the tool declaring rules without `r`: retired.
    Retired,
    /// None, the tool completed declaring nothing and reporting nothing of `r`: resolved, it ran.
    Unlisted,
    /// None, the tool failed.
    Failed,
    /// A violation with no fingerprint: new.
    Unkeyed,
    /// Both of the record's violations, moved to another file.
    Moved,
}

/// `new/accepted/resolved/stale verdict`.
type Input = Scene;
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_key_seen_more_often_than_accepted_is_new_by_the_surplus"),
        word: Word::Succeeds,
        input: || Scene::Surplus,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_key_seen_less_often_is_resolved_and_lowered"),
        word: Word::Succeeds,
        input: || Scene::Partly,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rule_the_tool_no_longer_declares_is_retired"),
        word: Word::Succeeds,
        input: || Scene::Retired,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_completed_tool_that_lists_no_rules_resolves_what_it_no_longer_reports"),
        word: Word::Succeeds,
        input: || Scene::Unlisted,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_failed_tool_leaves_its_records_unjudged_and_the_run_undetermined"),
        word: Word::Succeeds,
        input: || Scene::Failed,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_violation_with_no_fingerprint_is_new"),
        word: Word::Succeeds,
        input: || Scene::Unkeyed,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_violation_whose_code_moved_to_another_file_is_still_accepted"),
        word: Word::Succeeds,
        input: || Scene::Moved,
        expect: Expect::Value(String::new()),
    },
];

fn key() -> Key {
    Key {
        tool: ToolId("t".into()),
        rule: RuleId("r".into()),
        path: RepoPath("a.rs".into()),
        level: Level::Warning,
        standing: Standing::Reported,
        fingerprint: Fingerprint(9),
    }
}

fn record() -> Record {
    Record {
        id: Id("id".into()),
        key: key(),
        count: Count(2),
        message: MessageText("m".into()),
        since_rev: Provenance::Unknown,
        since: Day("2026-10-19".into()),
        reasons: vec![Reason("why".into())],
    }
}

fn violation(fingerprint: Fingerprinted) -> Violation {
    Violation {
        path: RepoPath("a.rs".into()),
        place: Place::Line(crate::domain::violations::types::LineNumber(1)),
        tool: ToolId("t".into()),
        rule: RuleId("r".into()),
        level: Level::Warning,
        suppression: Suppression::Unsuppressed,
        message: MessageText("m".into()),
        fingerprint,
    }
}

fn tool(status: Status, inventory: Inventory, reported: &[&str]) -> Run {
    Run {
        tool: ToolId("t".into()),
        status,
        inventory,
        reported: reported.iter().map(|rule| RuleId((*rule).into())).collect(),
    }
}

fn subject(scene: Input) -> Value {
    let known = Fingerprinted::Known(Fingerprint(9));
    let (seen, runs, expected) = match scene {
        Scene::Surplus => (
            3,
            tool(Status::Completed, Inventory::Undeclared, &["r"]),
            "1/2/0/0 Changed",
        ),
        Scene::Partly => (
            1,
            tool(Status::Completed, Inventory::Undeclared, &["r"]),
            "0/1/1/0 Changed",
        ),
        Scene::Retired => (
            0,
            tool(
                Status::Completed,
                Inventory::Declared(BTreeSet::from([RuleId("s".into())])),
                &[],
            ),
            "0/0/1/0 Changed",
        ),
        Scene::Unlisted => (
            0,
            tool(Status::Completed, Inventory::Undeclared, &[]),
            "0/0/1/0 Changed",
        ),
        Scene::Failed => (
            0,
            tool(
                Status::Failed(Ended::Unstartable(Problem("x".into()))),
                Inventory::Undeclared,
                &[],
            ),
            "0/0/0/1 Undetermined",
        ),
        Scene::Unkeyed => (
            2,
            tool(Status::Completed, Inventory::Undeclared, &["r"]),
            "1/2/0/0 Changed",
        ),
        Scene::Moved => (
            2,
            tool(Status::Completed, Inventory::Undeclared, &["r"]),
            "0/2/0/0 Pass",
        ),
    };
    let mut violations = vec![violation(known); seen];
    if let Scene::Moved = scene {
        for moved in &mut violations {
            moved.path = RepoPath("b.rs".into());
        }
    }
    if let Scene::Unkeyed = scene {
        violations.push(violation(Fingerprinted::Unknown));
    }
    let runs = [runs];
    let outcome = compare(&violations, &[record()], &runs);
    let said = format!(
        "{}/{}/{}/{} {:?}",
        outcome.new.len(),
        outcome.accepted.len(),
        outcome.resolved.len(),
        outcome.stale.len(),
        verdict(&runs, &outcome)
    );
    let lowered_right = outcome.resolved.first().is_none_or(|resolved| {
        lowered(resolved).map(|record| record.count)
            == (resolved.remaining.0 > 0).then_some(resolved.remaining)
    });
    if said == expected && lowered_right {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
