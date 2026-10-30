// ignore: idiomatic.arch/file-length - the use-case spec places one use_case_test.rs per use case and no second one: its case table proves every promise of check over the fakes, the store's lifecycle among them (unchanged reuse reads no contents, an edit runs only the tools it touched, a refused write keeps the store), and the rows share one scenario and one runner.
//! What the `check` use case promises, against the fakes of the ports it calls.

use super::errors::CheckError;
use super::fixtures::{
    LOG, Scenario, after_a_commit, improved, improving, reading_a_range, request, scenario,
    two_tools, up, up_request, use_case, with_sarif,
};
use super::interface::CheckUseCase as _;
use super::types::{Against, CheckRequest, Fixing, Reuse};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::judgement::types::Status;
use crate::domain::manifest::types::ProductRoot;
use crate::domain::operations::types::{Day, Operation, Provenance, Reason, Revision};
use crate::domain::ratchet::service::accepting;
use crate::domain::reports::service::failures;
use crate::domain::selection::types::Selectors;
use crate::inbound::use_cases::up::interface::UpUseCase as _;
use crate::outbound::check_store::types::KeptReport;
use crate::outbound::lock::interface::LockPort as _;
use crate::outbound::sarif::fixtures::fake::SarifFake;

/// What happens before the check that is judged.
#[derive(Clone, Copy, Debug)]
enum Scene {
    /// Nothing: the first check.
    First,
    /// A check, then nothing moves.
    Warm,
    /// A check, then the file the tool reads is edited.
    Edited,
    /// A check of a tool that reads a git range, then nothing moves.
    RangeKept,
    /// A check of a tool that reads a git range, then the commit it names moves.
    RangeMoved,
    /// Both violations recorded in the lock.
    Accepted,
    /// The tool's output is no SARIF log.
    Unreadable,
    /// Judged against another revision's branch, which accepted nothing.
    Since,
    /// Started where no product is.
    Nowhere,
    /// Judged against another branch in a tree with no commit.
    Detached,
    /// Both the worktree rename lookup and the lock graph fail.
    DualFailure,
    /// A perf measure below its best, in a plain check.
    ImprovedPlain,
    /// The same, in a check that fixes.
    ImprovedFixing,
    /// The same, in a check judged against a revision.
    ImprovedSince,
    /// A check, then nothing moves: how many file contents the second check read.
    Unread,
    /// Two tools reading different directories checked, then a file only `docs` reads is edited.
    OneInputEdited,
    /// A check, then nineteen more, the file the tool reads edited before every other one.
    ManyRuns,
    /// A check, then an edit, then a check whose store cannot be saved.
    SaveRefused,
    /// A check, then an edit, then a check in which the tool's output is no SARIF log.
    FailedAfterWarm,
    /// Two tools checked, then a file only `docs` reads is edited and `docs` prints no SARIF log: a check, `up`, and
    /// a second check.
    FailedBesideReused,
}

/// `new/accepted/resolved verdict runs`, the tools' runs counted.
type Input = Scene;
type Value = std::borrow::Cow<'static, str>;
type Failure = CheckError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("worktree_failure_precedes_a_simultaneously_invalid_lock"),
        word: Word::Fails,
        input: || Scene::DualFailure,
        expect: Expect::Refusal(|error| {
            if matches!(error, CheckError::Repository(crate::outbound::repository::errors::RepositoryError::Failed { reason }) if reason == "worktree unavailable")
            {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_plain_check_rewrites_an_improved_best_in_the_lock"),
        word: Word::Succeeds,
        input: || Scene::ImprovedPlain,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "improved 1 Written; operations 2",
        )),
    },
    Case {
        name: CaseName("a_check_that_fixes_never_writes_the_lock"),
        word: Word::Succeeds,
        input: || Scene::ImprovedFixing,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "improved 1 Withheld; operations 1",
        )),
    },
    Case {
        name: CaseName("a_check_against_a_revision_never_writes_the_lock"),
        word: Word::Succeeds,
        input: || Scene::ImprovedSince,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "improved 1 Withheld; operations 1",
        )),
    },
    Case {
        name: CaseName("many_runs_in_a_row_leave_a_bounded_number_of_files"),
        word: Word::Succeeds,
        input: || Scene::ManyRuns,
        expect: Expect::Value(std::borrow::Cow::Borrowed("bounded")),
    },
    Case {
        name: CaseName("a_first_check_reports_every_violation_as_new"),
        word: Word::Succeeds,
        input: || Scene::First,
        expect: Expect::Value(std::borrow::Cow::Borrowed("2/0/0 Changed ran 1")),
    },
    Case {
        name: CaseName("a_check_whose_inputs_did_not_move_reuses_the_last_run"),
        word: Word::Succeeds,
        input: || Scene::Warm,
        expect: Expect::Value(std::borrow::Cow::Borrowed("2/0/0 Changed reused 1")),
    },
    Case {
        name: CaseName("a_tool_that_reads_a_git_range_is_reused_while_its_commits_did_not_move"),
        word: Word::Succeeds,
        input: || Scene::RangeKept,
        expect: Expect::Value(std::borrow::Cow::Borrowed("2/0/0 Changed reused 1")),
    },
    Case {
        name: CaseName("a_tool_that_reads_a_git_range_runs_again_when_a_commit_of_it_moved"),
        word: Word::Succeeds,
        input: || Scene::RangeMoved,
        expect: Expect::Value(std::borrow::Cow::Borrowed("2/0/0 Changed ran 2")),
    },
    Case {
        name: CaseName("a_check_after_an_edit_runs_the_tool_again"),
        word: Word::Succeeds,
        input: || Scene::Edited,
        expect: Expect::Value(std::borrow::Cow::Borrowed("2/0/0 Changed ran 2")),
    },
    Case {
        name: CaseName("violations_the_lock_accepts_pass"),
        word: Word::Succeeds,
        input: || Scene::Accepted,
        expect: Expect::Value(std::borrow::Cow::Borrowed("0/2/0 Pass reused 1")),
    },
    Case {
        name: CaseName("against_a_branch_that_accepted_nothing_both_are_introduced"),
        word: Word::Succeeds,
        input: || Scene::Since,
        expect: Expect::Value(std::borrow::Cow::Borrowed("2/0/0 Changed ran 1")),
    },
    Case {
        name: CaseName("a_tool_whose_output_is_no_log_leaves_the_run_undetermined"),
        word: Word::Succeeds,
        input: || Scene::Unreadable,
        expect: Expect::Value(std::borrow::Cow::Borrowed("0/0/0 Undetermined ran 1")),
    },
    Case {
        name: CaseName("a_check_whose_inputs_did_not_move_reads_no_input_contents"),
        word: Word::Succeeds,
        input: || Scene::Unread,
        expect: Expect::Value(std::borrow::Cow::Borrowed("reused; 0 contents read")),
    },
    Case {
        name: CaseName("an_edit_runs_again_only_the_tools_whose_inputs_hold_it"),
        word: Word::Succeeds,
        input: || Scene::OneInputEdited,
        expect: Expect::Value(std::borrow::Cow::Borrowed("lint reused, docs ran")),
    },
    Case {
        name: CaseName(
            "a_check_whose_store_cannot_be_saved_still_answers_and_keeps_the_previous_store",
        ),
        word: Word::Succeeds,
        input: || Scene::SaveRefused,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "2/0/0 Changed ran 2; store kept",
        )),
    },
    Case {
        name: CaseName(
            "a_check_in_which_a_tool_fails_keeps_the_failure_and_the_last_completed_run_for_reuse",
        ),
        word: Word::Succeeds,
        input: || Scene::FailedAfterWarm,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "failed true; failure kept true; reused after the edit is undone true",
        )),
    },
    Case {
        name: CaseName(
            "after_a_failing_check_up_reports_the_failed_tool_and_the_other_tools_last_completed_runs_are_reused",
        ),
        word: Word::Succeeds,
        input: || Scene::FailedBesideReused,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "check: lint reused, docs failed; up: 1 tool(s) failed in the last check, `docs` among them; again: lint reused, docs failed; lint ran 0",
        )),
    },
    Case {
        name: CaseName("a_place_with_no_product_above_it_is_refused"),
        word: Word::Refuses,
        input: || Scene::Nowhere,
        expect: Expect::Refusal(|error| {
            if matches!(error, CheckError::Manifest(_)) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("since_in_a_tree_with_no_commit_is_refused"),
        word: Word::Refuses,
        input: || Scene::Detached,
        expect: Expect::error(CheckError::NoHead),
    },
];

fn said(scenario: &Scenario, check: CheckRequest) -> Result<String, Failure> {
    let before = scenario.store.kept();
    let saves = scenario
        .store
        .saves
        .load(std::sync::atomic::Ordering::Relaxed);
    let report = match use_case(scenario).check(check) {
        Ok(report) => report,
        Err(error) => {
            assert_eq!(scenario.store.kept(), before);
            assert_eq!(
                scenario
                    .store
                    .saves
                    .load(std::sync::atomic::Ordering::Relaxed),
                saves
            );
            return Err(error);
        }
    };
    let reuse = match report.runs.first().map(|run| run.reuse) {
        Some(Reuse::Reused) => "reused",
        Some(Reuse::Ran) | None => "ran",
    };
    let outcome = &report.outcome;
    Ok(format!(
        "{}/{}/{} {:?} {reuse} {}",
        outcome.new.len(),
        outcome.accepted.len(),
        outcome.resolved.len(),
        report.verdict,
        scenario.tools.ran().len()
    ))
}

fn initial(scene: Input) -> Scenario {
    match scene {
        Scene::DualFailure => Scenario {
            repository: std::sync::Arc::new(
                crate::outbound::repository::fixtures::fake::RepositoryFake {
                    worktree: Err(
                        crate::outbound::repository::errors::RepositoryError::Failed {
                            reason: "worktree unavailable".into(),
                        },
                    ),
                    ..crate::outbound::repository::fixtures::fake::RepositoryFake::default()
                },
            ),
            ..scenario()
        },
        Scene::Unreadable => with_sarif(scenario(), SarifFake::default()),
        Scene::Detached => {
            let detached = scenario();
            Scenario {
                repository: std::sync::Arc::new(
                    crate::outbound::repository::fixtures::fake::RepositoryFake {
                        head: crate::domain::renames::types::Head::Detached,
                        ..crate::outbound::repository::fixtures::fake::RepositoryFake::default()
                    },
                ),
                ..detached
            }
        }
        Scene::RangeKept | Scene::RangeMoved => reading_a_range(),
        Scene::OneInputEdited | Scene::FailedBesideReused => two_tools(),
        Scene::ImprovedPlain | Scene::ImprovedFixing | Scene::ImprovedSince => improving(),
        Scene::First
        | Scene::Warm
        | Scene::Edited
        | Scene::Accepted
        | Scene::Since
        | Scene::Unread
        | Scene::SaveRefused
        | Scene::FailedAfterWarm
        | Scene::ManyRuns
        | Scene::Nowhere => scenario(),
    }
}

/// Runs the first check of a scene, which fills the cache the second one is judged by, so it must have answered.
fn warm(scenario: &Scenario) -> Result<(), Failure> {
    assert!(
        !said(scenario, request())?.is_empty(),
        "the first check answers"
    );
    Ok(())
}

/// A second check whose inputs did not move answers as the first did, its runs reused.
fn warm_as_cold(scenario: &Scenario) -> Result<String, Failure> {
    let mut cold = use_case(scenario).check(request())?;
    let warm = use_case(scenario).check(request())?;
    for run in &mut cold.runs {
        run.reuse = Reuse::Reused;
    }
    assert_eq!(cold, warm);
    said(scenario, request())
}

/// How many file contents a check reads when nothing moved since the one before it.
fn unread(scenario: &Scenario) -> Result<String, Failure> {
    warm(scenario)?;
    let before = scenario.sources.contents_read();
    let second = use_case(scenario).check(request())?;
    let reused = second.runs.iter().all(|run| run.reuse == Reuse::Reused);
    Ok(format!(
        "{}; {} contents read",
        if reused { "reused" } else { "ran" },
        scenario.sources.contents_read() - before
    ))
}

/// Which tools run again when a file only `docs` reads is edited.
fn one_input_edited(scenario: &Scenario) -> Result<String, Failure> {
    warm(scenario)?;
    scenario.sources.write("docs/guide.md", "# Guide, edited\n");
    let second = use_case(scenario).check(request())?;
    Ok(second
        .runs
        .iter()
        .map(|run| {
            let reuse = match run.reuse {
                Reuse::Reused => "reused",
                Reuse::Ran => "ran",
            };
            format!("{} {reuse}", run.run.tool.0)
        })
        .collect::<Vec<_>>()
        .join(", "))
}

/// Whether many checks in a row, the tool's file edited before every other one, leave the store keeping no more files
/// than the first check did and the lock holding no more operations: a check keeps one record a file it stamps, and
/// writes the lock only for a perf best it improves.
fn many_runs(scenario: &Scenario) -> Result<String, Failure> {
    warm(scenario)?;
    let files = scenario.store.kept().recorded.files.len();
    let operations = scenario.lock.held().len();
    for round in 1_u32..20 {
        let text = if round % 2 == 0 {
            "fn a() {}\n"
        } else {
            "fn a() {}\nfn b() {}\n"
        };
        scenario.sources.write("src/a.rs", text);
        let checked = use_case(scenario).check(request())?;
        if checked.runs.is_empty()
            || scenario.store.kept().recorded.files.len() > files
            || scenario.lock.held().len() > operations
        {
            return Ok(format!("grew at run {round}"));
        }
    }
    Ok("bounded".to_owned())
}

/// A check whose store cannot be saved, after an edit.
fn save_refused(scenario: Scenario) -> Result<String, Failure> {
    warm(&scenario)?;
    scenario.sources.write("src/a.rs", "fn a() {}\n");
    let refusing = Scenario {
        store: std::sync::Arc::new(
            crate::outbound::check_store::fixtures::fake::CheckStoreFake {
                stored: std::sync::Mutex::new(scenario.store.kept()),
                refusing: true,
                ..crate::outbound::check_store::fixtures::fake::CheckStoreFake::default()
            },
        ),
        ..scenario
    };
    let before = refusing.store.kept();
    let answer = said(&refusing, request())?;
    Ok(if refusing.store.kept() == before {
        format!("{answer}; store kept")
    } else {
        format!("{answer}; store rewritten")
    })
}

/// A check whose tool fails, after an edit: the failure is kept, and once the edit is undone the tool's last completed
/// run is reused.
fn failed_after_warm(scenario: Scenario) -> Result<String, Failure> {
    warm(&scenario)?;
    let completing = std::sync::Arc::clone(&scenario.sarif);
    scenario.sources.write("src/a.rs", "fn a() {}\n");
    let failing = with_sarif(scenario, SarifFake::default());
    let report = use_case(&failing).check(request())?;
    let failed = report
        .runs
        .iter()
        .any(|run| matches!(run.run.status, Status::Failed(_)));
    let recorded = match failing.store.kept().report {
        KeptReport::Kept(kept) => !failures(&kept).is_empty(),
        KeptReport::Nothing => false,
    };
    let restored = Scenario {
        sarif: completing,
        ..failing
    };
    restored.sources.write("src/a.rs", "fn a() {}\nfn b() {}\n");
    let again = use_case(&restored).check(request())?;
    let reused = again.runs.iter().all(|run| run.reuse == Reuse::Reused);
    Ok(format!(
        "failed {failed}; failure kept {recorded}; reused after the edit is undone {reused}"
    ))
}

/// Two tools checked, then a file only `docs` reads is edited and `docs` prints no SARIF log: what the check after
/// the edit did with each tool, what `up` says after it, what a second check did with each tool, and how often `lint`
/// ran across both. The failed run is recorded, so `up` names it; `lint`'s last completed run is reused, never re-run.
fn failed_beside_reused(scenario: Scenario) -> Result<String, Failure> {
    warm(&scenario)?;
    scenario.sources.write("docs/guide.md", "# Guide, edited\n");
    let failing = Scenario {
        tools: std::sync::Arc::new(
            crate::outbound::tools::fixtures::fake::ToolsFake::default()
                .answering("lint", 1, LOG)
                .answering("docs", 1, "docs-unreadable"),
        ),
        ..scenario
    };
    let each = |report: &super::types::CheckReport| {
        report
            .runs
            .iter()
            .map(|run| {
                let how = match (&run.run.status, run.reuse) {
                    (Status::Failed(_), _) => "failed",
                    (_, Reuse::Reused) => "reused",
                    (_, Reuse::Ran) => "ran",
                };
                format!("{} {how}", run.run.tool.0)
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let checked = each(&use_case(&failing).check(request())?);
    let refused = match up(&failing).up(up_request(Selectors::default())) {
        Ok(report) => format!("accepted {report:?}"),
        Err(error) => error.to_string(),
    };
    let again = each(&use_case(&failing).check(request())?);
    let lint = failing
        .tools
        .ran()
        .iter()
        .filter(|argv| argv.0.first().is_some_and(|program| &*program.0 == "lint"))
        .count();
    Ok(format!(
        "check: {checked}; up: {refused}; again: {again}; lint ran {lint}"
    ))
}

fn told(scene: Input) -> Result<String, Failure> {
    let scenario = initial(scene);
    match scene {
        Scene::DualFailure => {
            scenario.lock.append(&Operation {
                parents: vec![crate::domain::operations::types::OperationId(
                    "missing-parent".into(),
                )],
                covers: Vec::new(),
                accepted: Vec::new(),
                removed: Vec::new(),
                perf: Vec::new(),
                perf_removed: Vec::new(),
            })?;
            said(&scenario, request())
        }
        Scene::First | Scene::Unreadable => said(&scenario, request()),
        Scene::ImprovedPlain | Scene::ImprovedFixing | Scene::ImprovedSince => improved(
            if matches!(scene, Scene::ImprovedFixing) {
                Fixing::Fix
            } else {
                Fixing::Report
            },
            if matches!(scene, Scene::ImprovedSince) {
                Against::Since(Revision("origin/main".into()))
            } else {
                Against::Lock
            },
            &scenario,
        ),
        Scene::Warm => warm_as_cold(&scenario),
        Scene::RangeKept => {
            warm(&scenario)?;
            said(&scenario, request())
        }
        Scene::RangeMoved => {
            warm(&scenario)?;
            said(&after_a_commit(scenario), request())
        }
        Scene::Unread => unread(&scenario),
        Scene::OneInputEdited => one_input_edited(&scenario),
        Scene::SaveRefused => save_refused(scenario),
        Scene::FailedAfterWarm => failed_after_warm(scenario),
        Scene::FailedBesideReused => failed_beside_reused(scenario),
        Scene::ManyRuns => many_runs(&scenario),
        Scene::Edited => {
            warm(&scenario)?;
            scenario
                .sources
                .write("src/a.rs", "fn a() {}\nfn b() {}\n// more\n");
            said(&scenario, request())
        }
        Scene::Accepted => {
            let first = use_case(&scenario).check(request())?;
            let lock = std::sync::Arc::clone(&scenario.lock);
            let made = accepting(
                &[],
                &first.outcome.new,
                &Provenance::Unknown,
                &Day("2026-10-19".into()),
                Some(&Reason("why".into())),
                &|key| lock.id(key),
            );
            let operation = Operation {
                parents: Vec::new(),
                covers: Vec::new(),
                accepted: made.records,
                removed: Vec::new(),
                perf: Vec::new(),
                perf_removed: Vec::new(),
            };
            // The accepting operation's id is not needed: the check reads the lock back through the same fake.
            lock.append(&operation)?;
            said(&scenario, request())
        }
        Scene::Since | Scene::Detached => said(
            &scenario,
            CheckRequest {
                against: Against::Since(Revision("origin/main".into())),
                ..request()
            },
        ),
        Scene::Nowhere => said(
            &scenario,
            CheckRequest {
                start: ProductRoot("/elsewhere".into()),
                ..request()
            },
        ),
    }
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

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
