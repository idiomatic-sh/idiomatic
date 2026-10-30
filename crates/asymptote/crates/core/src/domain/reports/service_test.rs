// ignore: idiomatic.arch/file-length - the concept spec places one service_test.rs per concept: its case table proves every rule of the reports service (moves, reuse, retention, the stamp of a range, which files a record vouches for), each row over the same few fixtures.
//! Whether a kept report and its runs still hold, and what a judgement rests on.

use std::collections::{BTreeMap, BTreeSet};

use super::service::{
    failures, moved, observed, range_ends, retained, reusable, stamp_parts, unlisted,
};
use super::types::{
    Digest, FileDigest, FileRecord, Keeping, Listed, Recorded, Report, Scanned, Stamped,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::judgement::types::{Failure as Ended, Inventory, Problem, Run, Status};
use crate::domain::manifest::types::{Argument, Argv, Fix, Group, Output, Pathspec, Seconds, Tool};
use crate::domain::operations::types::{Count, Day, Provenance, Reason, Record, Revision};
use crate::domain::renames::types::Head;
use crate::domain::violations::types::{
    Fingerprint, Fingerprinted, Id, Key, Level, MessageText, Place, RepoPath, RuleId, Standing,
    Suppression, ToolId, Violation,
};

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// A file whose digest changed has moved.
    Moved,
    /// A completed run under the same stamp is reused.
    Reused,
    /// A run under another stamp is not.
    OtherStamp,
    /// A failed run is a failure, and is never reused.
    Failed,
    /// The files a judgement rests on beyond the listed inputs.
    Unlisted,
    /// A tool of the groups not selected keeps its completed run.
    Retains,
    /// A tool selected, failed, or no longer configured keeps nothing.
    Drops,
    /// A retained run is neither observed, a failure, nor measured.
    Hidden,
    /// What the stamp of a tool that reads a git range comes to as the commits move.
    Stamps,
    /// The ends of the git ranges a command line names.
    Ranges,
    /// Which files a record vouches for: one unchanged, one resized, one recorded racily, one new and one absent.
    Unread,
}

type Input = Question;
type Value = std::borrow::Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_is_read_only_where_its_record_cannot_vouch_for_it"),
        word: Word::Succeeds,
        input: || Question::Unread,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "read resized.rs racy.rs new.rs; kept.rs keeps k, gone.rs is absent",
        )),
    },
    Case {
        name: CaseName("a_file_whose_digest_changed_has_moved"),
        word: Word::Succeeds,
        input: || Question::Moved,
        expect: Expect::Value(std::borrow::Cow::Borrowed("a.rs")),
    },
    Case {
        name: CaseName("a_completed_run_under_the_same_stamp_is_reused"),
        word: Word::Succeeds,
        input: || Question::Reused,
        expect: Expect::Value(std::borrow::Cow::Borrowed("reused")),
    },
    Case {
        name: CaseName("a_run_under_another_stamp_is_run_again"),
        word: Word::Succeeds,
        input: || Question::OtherStamp,
        expect: Expect::Value(std::borrow::Cow::Borrowed("run")),
    },
    Case {
        name: CaseName("a_failed_run_is_a_failure_and_never_reused"),
        word: Word::Succeeds,
        input: || Question::Failed,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 failed, run")),
    },
    Case {
        name: CaseName(
            "a_judgement_rests_on_every_violations_file_and_every_records_beyond_the_inputs",
        ),
        word: Word::Succeeds,
        input: || Question::Unlisted,
        expect: Expect::Value(std::borrow::Cow::Borrowed("b.rs c.rs")),
    },
    Case {
        name: CaseName("a_tool_of_the_groups_not_selected_keeps_its_completed_run_as_retained"),
        word: Word::Succeeds,
        input: || Question::Retains,
        expect: Expect::Value(std::borrow::Cow::Borrowed("t:Retained")),
    },
    Case {
        name: CaseName("a_tool_selected_failed_or_no_longer_configured_keeps_nothing"),
        word: Word::Succeeds,
        input: || Question::Drops,
        expect: Expect::Value(std::borrow::Cow::Borrowed("selected=0 failed=0 gone=0")),
    },
    Case {
        name: CaseName("the_commits_of_a_range_are_in_the_stamp_of_the_tool_that_reads_it"),
        word: Word::Succeeds,
        input: || Question::Stamps,
        expect: Expect::Value(std::borrow::Cow::Borrowed("commits differs")),
    },
    Case {
        name: CaseName(
            "the_ends_of_a_range_in_a_command_line_are_found_and_a_path_that_climbs_is_not_one",
        ),
        word: Word::Succeeds,
        input: || Question::Ranges,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "origin/main HEAD|main v1|-|-|-|origin/main HEAD main",
        )),
    },
    Case {
        name: CaseName("a_retained_run_is_neither_observed_nor_a_failure"),
        word: Word::Succeeds,
        input: || Question::Hidden,
        expect: Expect::Value(std::borrow::Cow::Borrowed("0 runs, 0 failures")),
    },
];

/// A violation in `path`.
fn violation(path: &str) -> Violation {
    Violation {
        path: RepoPath(path.into()),
        place: Place::File,
        tool: ToolId("t".into()),
        rule: RuleId("r".into()),
        level: Level::Error,
        suppression: Suppression::Unsuppressed,
        message: MessageText("m".into()),
        fingerprint: Fingerprinted::Unknown,
    }
}

/// A record of `path`.
fn record(path: &str) -> Record {
    Record {
        id: Id("a".into()),
        key: Key {
            tool: ToolId("t".into()),
            rule: RuleId("r".into()),
            path: RepoPath(path.into()),
            level: Level::Error,
            standing: Standing::Reported,
            fingerprint: Fingerprint(1),
        },
        count: Count(1),
        message: MessageText("m".into()),
        since_rev: Provenance::Unknown,
        since: Day("2026-10-19".into()),
        reasons: vec![Reason("why".into())],
    }
}

fn report(status: Status) -> Report {
    Report {
        head: Head::Detached,
        scanned: vec![Scanned {
            run: Run {
                tool: ToolId("t".into()),
                status,
                inventory: Inventory::Undeclared,
                reported: BTreeSet::new(),
            },
            violations: Vec::new(),
            stamp: Stamped::Digested(Digest("s".into())),
            inputs: Vec::new(),
            measurements: Vec::new(),
            keeping: Keeping::Current,
        }],
    }
}

/// The record of `a.rs`, stamped `1`.
fn recorded() -> Recorded {
    Recorded {
        files: [(
            RepoPath("a.rs".into()),
            FileRecord {
                digest: Digest("1".into()),
                ..FileRecord::default()
            },
        )]
        .into(),
        ..Recorded::default()
    }
}

/// A tool whose command line is `words`.
fn running(words: &[&str], output: Output) -> Tool {
    Tool {
        id: ToolId("t".into()),
        run: Argv(words.iter().map(|word| Argument((*word).into())).collect()),
        fix: Fix::Absent,
        output,
        group: Group("perf".into()),
        timeout: Seconds(1),
        inputs: vec![Pathspec(".".into())],
    }
}

/// What the stamp of a tool that reads a range comes to with the commits it resolves to.
fn stamps() -> String {
    let named: BTreeMap<Pathspec, Option<Digest>> =
        [(Pathspec(".".into()), Some(Digest("root".into())))].into();
    let parts = |commits: &[Revision]| {
        stamp_parts(&running(&["t"], Output::Sarif), &named, commits).map(|parts| parts.0.concat())
    };
    let moved = parts(&[Revision("a".into())]) != parts(&[Revision("b".into())]);
    format!("commits {}", if moved { "differs" } else { "same" })
}

/// The ends of the ranges in a command line, `|` between commands; `-` for none.
fn ranges() -> String {
    let ends = |words: &[&str]| {
        let found: Vec<String> = range_ends(&running(words, Output::ExitStatus))
            .iter()
            .map(|end| end.0.to_string())
            .collect();
        if found.is_empty() {
            "-".to_owned()
        } else {
            found.join(" ")
        }
    };
    [
        ends(&[
            "bash",
            "-c",
            "[ -z \"$(git rev-list origin/main..HEAD)\" ] || cz check --rev-range origin/main..HEAD",
        ]),
        ends(&["git", "log", "main...v1"]),
        ends(&["cargo", "dylint", "--path", "../../tools/lints"]),
        ends(&["t", "../x", "a/../b", "--flag=.."]),
        ends(&["t", "src/..", "..", "..."]),
        ends(&["t", "--rev-range=origin/main..HEAD", "main.."]),
    ]
    .join("|")
}

fn told(question: Input) -> String {
    let tool = ToolId("t".into());
    let again = |report: &Report, stamp: &str| {
        if reusable(report, &tool, &Digest(stamp.into())).is_some() {
            "reused"
        } else {
            "run"
        }
        .to_owned()
    };
    match question {
        Question::Moved => {
            let now = [FileDigest {
                path: RepoPath("a.rs".into()),
                digest: Digest("2".into()),
            }];
            moved(&recorded(), &now)
                .iter()
                .map(|path| path.0.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        }
        Question::Unlisted => {
            let listed = Listed {
                files: BTreeMap::new(),
                stamps: [(RepoPath("a.rs".into()), Digest("1".into()))].into(),
            };
            unlisted(
                &listed,
                &[violation("a.rs"), violation("b.rs")],
                &[record("c.rs"), record("b.rs")],
            )
            .iter()
            .map(|path| path.0.to_string())
            .collect::<Vec<_>>()
            .join(" ")
        }
        Question::Retains => retained(
            &report(Status::Completed).scanned,
            std::slice::from_ref(&tool),
            &[],
        )
        .iter()
        .map(|kept| format!("{}:{:?}", kept.run.tool.0, kept.keeping))
        .collect::<Vec<_>>()
        .join(" "),
        Question::Drops => {
            let done = report(Status::Completed).scanned;
            let failed = report(Status::Failed(Ended::Unstartable(Problem("x".into())))).scanned;
            format!(
                "selected={} failed={} gone={}",
                retained(
                    &done,
                    std::slice::from_ref(&tool),
                    std::slice::from_ref(&tool)
                )
                .len(),
                retained(&failed, std::slice::from_ref(&tool), &[]).len(),
                retained(&done, &[], &[]).len()
            )
        }
        Question::Stamps => stamps(),
        Question::Ranges => ranges(),
        Question::Hidden => {
            let mut done = report(Status::Completed);
            done.scanned = retained(&done.scanned.clone(), std::slice::from_ref(&tool), &[]);
            let seen = observed(&done);
            format!(
                "{} runs, {} failures",
                seen.runs.len(),
                failures(&done).len()
            )
        }
        Question::Unread => unread_files(),
        Question::Reused => again(&report(Status::Completed), "s"),
        Question::OtherStamp => again(&report(Status::Completed), "other"),
        Question::Failed => {
            let failed = report(Status::Failed(Ended::Unstartable(Problem("x".into()))));
            let runs = observed(&failed).runs;
            format!(
                "{} failed, {}",
                failures(&failed).len().min(runs.len()),
                again(&failed, "s")
            )
        }
    }
}

/// What the domain reads of five files a check stamped before, and what it records after.
fn unread_files() -> String {
    use super::{
        service::{recorded, unread},
        types::{FileStat, FileState, Moment, Size, Stats},
    };
    let record = |modified: u128, digest: &str| FileRecord {
        size: Size(1),
        modified: Moment(modified),
        digest: Digest(digest.into()),
    };
    let known = Recorded {
        files: [
            ("kept.rs", record(1, "k")),
            ("resized.rs", record(1, "r")),
            ("racy.rs", record(9_000_000_000, "y")),
        ]
        .into_iter()
        .map(|(path, kept)| (RepoPath(path.into()), kept))
        .collect(),
        taken: Moment(10_000_000_000),
    };
    let present = |size: u64, modified: u128| FileState::Present {
        size: Size(size),
        modified: Moment(modified),
    };
    let stats = Stats {
        files: [
            ("kept.rs", present(1, 1)),
            ("resized.rs", present(2, 1)),
            ("racy.rs", present(1, 9_000_000_000)),
            ("new.rs", present(1, 1)),
            ("gone.rs", FileState::Absent),
        ]
        .into_iter()
        .map(|(path, state)| FileStat {
            path: RepoPath(path.into()),
            state,
        })
        .collect(),
        taken: Moment(20_000_000_000),
    };
    let read: Vec<String> = unread(&known, &stats)
        .iter()
        .map(|path| path.0.to_string())
        .collect();
    let after = recorded(&known, &stats, &[]);
    let digest = |path: &str| {
        after
            .files
            .get(&RepoPath(path.into()))
            .map(|kept| kept.digest.0.to_string())
            .unwrap_or_default()
    };
    format!(
        "read {}; kept.rs keeps {}, gone.rs is {}",
        read.join(" "),
        digest("kept.rs"),
        digest("gone.rs")
    )
}

fn subject(input: Input) -> Value {
    std::borrow::Cow::Owned(told(input))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
