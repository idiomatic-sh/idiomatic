// ignore: idiomatic.arch/file-length - the check case matrix and its subject jointly specify complete reports, refusal context, and cache effects for one public workflow.
//! What the `check` use case promises, against the fakes of the ports it calls.

use std::sync::Arc;

use super::errors::CheckError;
use super::fixtures::StoreFake;
use super::fixtures::{FOUND, analysis, repository, request, selection, use_case};
use super::interface::CheckUseCase as _;
use super::types::{CheckRequest, Fixing};
use super::use_case::CheckUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::analysis::fixtures::fake::AnalysisFake;
use crate::outbound::repository::fixtures::fake::RepositoryFake;

/// What a row changes about the fixture.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The fixture as it is, its lock rewritten.
    Placed,
    /// Checked again with nothing changed: the run answers what the one before kept, and keeps nothing new.
    Twice,
    /// Checked again after the lock's text changed: a file the answer rests on changed, so the check runs again.
    Relocked,
    /// Checked again with `--fix` after the lock became text that is not a lock: an error, never a lock rewritten as if
    /// it were missing.
    CorruptLock,
    /// No lock, and no `--fix`.
    Unlocked,
    /// The engine fails.
    EngineFails,
    /// No workspace at the start.
    Nowhere,
    /// The root `tastemaker.toml` declares a variable nothing refers to.
    UnusedVariable,
    /// The rule's entry excludes the directory its finding is in.
    EntryExcluded,
    /// The workspace's `rules-exclude` names the directory the finding is in, so ryo is not handed its files.
    RulesExcluded,
    /// Checked again after a part was added: the walk finds it, and the check runs again.
    Added,
    /// Checked again with no file moved, by an engine that would fail: the last report is the answer.
    Unmoved,
    /// Checked again after a file's text changed, by an engine that fails: the lint call is made again.
    Moved,
    /// Checked again within one directory, by an engine that fails: a report of another scope is not the answer.
    Rescoped,
    /// The engine cannot run the rule, which governs files: the rule is refused with a `#query` error.
    Unrunnable,
    /// The engine cannot run the rule, which governs none of the files: it is left out, with no finding.
    Idle,
    /// A previously checked workspace loses its validation proofs.
    Stale,
    /// The root `tastemaker.toml` is text the manifest port cannot read.
    Broken,
    /// The nearest `tastemaker.toml` is a local one: it declares no `workspace`.
    Local,
    /// A member's local `tastemaker.toml` declares a `workspace`, which only the root file does.
    MemberWithWorkspace,
    /// Checked within a path that holds no file of the workspace.
    Nonexistent,
    /// Checked many times in a row, a file added before each.
    Many,
    /// Checked again with nothing changed: the kept answer, and no file read but the workspace's `tastemaker.toml`
    /// files and its lock.
    ReadsNothing,
    /// Checked again after one file's text changed: of the files the answer rests on, only that one's row moves.
    OneEdit,
    /// Checked again after a file's text changed, by an engine that fails: the store keeps the answer it held.
    FailedKeeps,
}

/// What a row changes about the fixture.
type Input = Scenario;

/// The rule ids found, sorted and joined, and whether the plan was kept.
type Value = String;
type Failure = CheckError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_placed_workspace_reports_its_stray_file_its_missing_count_and_its_rule"),
        word: Word::Succeeds,
        input: || Scenario::Placed,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_second_run_with_nothing_changed_answers_the_kept_answer"),
        word: Word::Succeeds,
        input: || Scenario::Twice,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_second_run_after_the_lock_changed_checks_again"),
        word: Word::Succeeds,
        input: || Scenario::Relocked,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_root_variable_nothing_refers_to_is_a_format_finding"),
        word: Word::Succeeds,
        input: || Scenario::UnusedVariable,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_exclude_of_a_rule_entry_is_handed_to_ryo"),
        word: Word::Succeeds,
        input: || Scenario::EntryExcluded,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_under_rules_exclude_is_not_handed_to_ryo"),
        word: Word::Succeeds,
        input: || Scenario::RulesExcluded,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_added_since_the_last_run_is_listed"),
        word: Word::Succeeds,
        input: || Scenario::Added,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("many_runs_in_a_row_leave_a_bounded_store"),
        word: Word::Succeeds,
        input: || Scenario::Many,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_run_with_no_file_moved_answers_the_last_report"),
        word: Word::Succeeds,
        input: || Scenario::Unmoved,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_run_after_a_file_moved_asks_the_engine_again"),
        word: Word::Fails,
        input: || Scenario::Moved,
        expect: Expect::Refusal(|error| selection(matches!(error, CheckError::Analysis(_)))),
    },
    Case {
        name: CaseName("a_run_of_another_scope_asks_the_engine_again"),
        word: Word::Fails,
        input: || Scenario::Rescoped,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(CheckError::Analysis(
                    crate::outbound::analysis::errors::AnalysisError::Engine {
                        reason: "the fake fails".into(),
                    },
                ))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_rule_the_engine_cannot_run_is_a_query_finding"),
        word: Word::Succeeds,
        input: || Scenario::Unrunnable,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "a_rule_entry_that_governs_no_file_is_a_coverage_error_and_its_unrunnable_query_no_finding",
        ),
        word: Word::Succeeds,
        input: || Scenario::Idle,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("check_refuses_when_the_validation_proofs_are_missing"),
        word: Word::Refuses,
        input: || Scenario::Stale,
        expect: Expect::Refusal(|error| selection(matches!(error, CheckError::Proof { .. }))),
    },
    Case {
        name: CaseName("a_missing_lock_without_fix_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Unlocked,
        expect: Expect::Refusal(|error| selection(matches!(error, CheckError::Lock { .. }))),
    },
    Case {
        name: CaseName("no_workspace_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Nowhere,
        expect: Expect::Refusal(|error| selection(matches!(error, CheckError::Repository(_)))),
    },
    Case {
        name: CaseName("an_engine_that_fails_fails_the_check"),
        word: Word::Fails,
        input: || Scenario::EngineFails,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(CheckError::Analysis(
                    crate::outbound::analysis::errors::AnalysisError::Engine {
                        reason: "the fake fails".into(),
                    },
                ))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_root_file_the_manifest_cannot_read_fails_the_check"),
        word: Word::Fails,
        input: || Scenario::Broken,
        expect: Expect::Refusal(|error| selection(matches!(error, CheckError::Manifest { .. }))),
    },
    Case {
        name: CaseName("a_local_tastemaker_toml_is_no_root_to_check_from"),
        word: Word::Refuses,
        input: || Scenario::Local,
        expect: Expect::Refusal(|error| selection(matches!(error, CheckError::NotARoot { .. }))),
    },
    Case {
        name: CaseName("a_member_file_that_declares_a_workspace_is_refused"),
        word: Word::Refuses,
        input: || Scenario::MemberWithWorkspace,
        expect: Expect::Refusal(|error| selection(matches!(error, CheckError::Resolution { .. }))),
    },
    Case {
        name: CaseName("a_path_that_holds_no_file_of_the_workspace_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Nonexistent,
        expect: Expect::Refusal(|error| {
            selection(matches!(error, CheckError::Within { path } if path == "nowhere"))
        }),
    },
    Case {
        name: CaseName("a_lock_that_does_not_read_is_refused_not_rewritten"),
        word: Word::Refuses,
        input: || Scenario::CorruptLock,
        expect: Expect::Refusal(|error| selection(matches!(error, CheckError::Manifest { .. }))),
    },
    Case {
        name: CaseName("an_unchanged_check_answers_the_kept_answer_reading_no_governed_file"),
        word: Word::Succeeds,
        input: || Scenario::ReadsNothing,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("one_edited_file_moves_only_its_own_row"),
        word: Word::Succeeds,
        input: || Scenario::OneEdit,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_failed_check_keeps_the_answer_the_store_held"),
        word: Word::Succeeds,
        input: || Scenario::FailedKeeps,
        expect: Expect::Value(String::new()),
    },
];

/// The lifecycle rows: what a row answers, or nothing for a row of the others.
fn lifecycle(scenario: Scenario) -> Option<Result<Value, Failure>> {
    use crate::outbound::check_store::interface::CheckStorePort as _;
    match scenario {
        Scenario::ReadsNothing | Scenario::OneEdit | Scenario::FailedKeeps => {}
        Scenario::Placed
        | Scenario::Twice
        | Scenario::Relocked
        | Scenario::CorruptLock
        | Scenario::Unlocked
        | Scenario::EngineFails
        | Scenario::Nowhere
        | Scenario::UnusedVariable
        | Scenario::EntryExcluded
        | Scenario::RulesExcluded
        | Scenario::Added
        | Scenario::Unmoved
        | Scenario::Moved
        | Scenario::Rescoped
        | Scenario::Unrunnable
        | Scenario::Idle
        | Scenario::Stale
        | Scenario::Broken
        | Scenario::Local
        | Scenario::MemberWithWorkspace
        | Scenario::Nonexistent
        | Scenario::Many => return None,
    }
    let store = Arc::new(StoreFake::default());
    let files = repository();
    let held = files.clone();
    let answer = (|| -> Result<Value, Failure> {
        let check = use_case(files, analysis(), &store)?;
        let _first = check.check(request(Fixing::Fix))?;
        let kept = store.answers.checked();
        held.reads
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        if !matches!(scenario, Scenario::ReadsNothing)
            && let Ok(mut texts) = held.files.lock()
        {
            drop(texts.insert(
                crate::domain::paths::types::RepoPath("src/a/errors.txt".into()),
                crate::domain::files::types::FileText(
                    "// edited, and longer than the text it replaces\n".into(),
                ),
            ));
        }
        let now = if matches!(scenario, Scenario::FailedKeeps) {
            let failing = AnalysisFake {
                fails: true,
                ..AnalysisFake::default()
            };
            let refused = use_case(held.clone(), failing, &store)?.check(request(Fixing::Report));
            if !matches!(refused, Err(CheckError::Analysis(_))) {
                return Ok(format!("the failing engine answered {refused:?}"));
            }
            store.answers.checked()
        } else {
            let _again = check.check(request(Fixing::Report))?;
            store.answers.checked()
        };
        let read = held
            .reads
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let moved = now
            .files
            .iter()
            .filter(|file| !kept.files.contains(file))
            .map(|file| file.path.0.to_string())
            .collect::<Vec<_>>();
        let only_tomls = read
            .iter()
            .all(|path| path.0.ends_with("tastemaker.toml") || path.0.ends_with("tastemaker.lock"));
        let holds = (matches!(scenario, Scenario::ReadsNothing) && now == kept && only_tomls)
            || (matches!(scenario, Scenario::OneEdit) && moved == ["src/a/errors.txt"])
            || (matches!(scenario, Scenario::FailedKeeps) && now == kept);
        Ok(if holds {
            String::new()
        } else {
            format!("read {read:?}, moved {moved:?}")
        })
    })();
    Some(answer)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    if let Some(answer) = lifecycle(scenario) {
        return answer;
    }
    if matches!(scenario, Scenario::Placed) {
        super::fixtures::serial_and_parallel_agree()?;
    }
    let store = Arc::new(StoreFake::default());
    let mut files = repository();
    if matches!(scenario, Scenario::Nowhere) {
        files.workspace = crate::outbound::repository::fixtures::fake::FakeWorkspace::Missing;
    }
    let edited = edited_workspace(scenario, &files);
    if let Some(text) = edited
        && let Ok(mut root) = files.files.lock()
    {
        drop(root.insert(
            crate::domain::paths::types::RepoPath("tastemaker.toml".into()),
            crate::domain::files::types::FileText(text.into()),
        ));
    }
    let engine = engine_for(scenario);
    let fixing = if matches!(scenario, Scenario::Unlocked) {
        Fixing::Report
    } else {
        Fixing::Fix
    };
    let held = files.clone();
    let check = use_case(files, engine, &store)?;
    if matches!(scenario, Scenario::Nonexistent) {
        let _checked = check.check(CheckRequest {
            within: vec![crate::domain::paths::types::RepoPath("nowhere".into())],
            ..request(fixing)
        })?;
        return Ok("a check within no file unexpectedly succeeded".to_owned());
    }
    let mut report = check.check(request(fixing))?;
    if matches!(scenario, Scenario::Added) {
        return added(&check, &held, &store, report.report.findings.len());
    }
    if matches!(scenario, Scenario::Many) {
        return many(&check, &held, &store);
    }
    if matches!(
        scenario,
        Scenario::Unmoved | Scenario::Moved | Scenario::Rescoped
    ) {
        report = rechecked(scenario, &check, &held, &store)?;
    }
    if matches!(scenario, Scenario::Stale) {
        if let Ok(mut rows) = store.proofs.tables.lock() {
            rows.clear();
        }
        let _refused = check.check(request(Fixing::Report))?;
        return Ok("a check without proofs unexpectedly succeeded".to_owned());
    }
    if matches!(scenario, Scenario::Relocked) {
        return relocked(&check, &held, &store);
    }
    if matches!(scenario, Scenario::CorruptLock) {
        if let Ok(mut held_files) = held.files.lock() {
            drop(held_files.insert(
                crate::domain::paths::types::RepoPath("tastemaker.lock".into()),
                crate::domain::files::types::FileText("not a lock [[".into()),
            ));
        }
        let _rewritten = check.check(request(Fixing::Fix))?;
        return Ok("a lock that does not read was rewritten".to_owned());
    }
    if matches!(scenario, Scenario::Twice) {
        // The first run rewrote its lock and kept its answer resting on it, so the second answers that.
        let before = kept(&store);
        report = check.check(request(Fixing::Report))?;
        if kept(&store) != before {
            return Ok("the second run checked again".to_owned());
        }
    }
    let mut found = report
        .report
        .findings
        .iter()
        .map(|finding| {
            crate::domain::findings::service::printed(&finding.rule)
                .0
                .to_string()
        })
        .collect::<Vec<_>>();
    found.sort();
    let found = found.join(" ");
    let expected = expected_rules(scenario);
    Ok(if found == expected {
        String::new()
    } else {
        found
    })
}

/// How many times the check kept its answer: a run that checks again keeps it again.
fn kept(store: &StoreFake) -> usize {
    *store
        .answers
        .check_saves
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Checks again after the lock's text changed: the kept answer rests on the lock before, so the check runs again.
fn relocked(
    check: &CheckUseCaseImpl,
    held: &RepositoryFake,
    store: &StoreFake,
) -> Result<Value, Failure> {
    let before = kept(store);
    let lock = crate::domain::paths::types::RepoPath("tastemaker.lock".into());
    if let Ok(mut files) = held.files.lock()
        && let Some(text) = files.get(&lock).cloned()
    {
        drop(files.insert(
            lock,
            crate::domain::files::types::FileText(format!("{}# relocked\n", text.0).into()),
        ));
    }
    let _report = check.check(request(Fixing::Report))?;
    Ok(if kept(store) == before {
        "the run after the lock changed answered the answer resting on the lock before".to_owned()
    } else {
        String::new()
    })
}

/// Checks again after a part was added to the files: the walk must find it, and the check run again.
fn added(
    check: &CheckUseCaseImpl,
    held: &RepositoryFake,
    store: &StoreFake,
    before: usize,
) -> Result<Value, Failure> {
    let planned = kept(store);
    if let Ok(mut files) = held.files.lock() {
        drop(files.insert(
            crate::domain::paths::types::RepoPath("src/c/notes.txt".into()),
            crate::domain::files::types::FileText("c\n".into()),
        ));
    }
    let report = check.check(request(Fixing::Report))?;
    Ok(
        if kept(store) != planned && report.report.findings.len() > before {
            String::new()
        } else {
            "the added part was not listed".to_owned()
        },
    )
}

/// Checks twelve times in a row, a file added before each: what the store keeps is the last run's, every file it rests
/// on named once, and validation's proofs as many as after the first run, never a copy for each run.
fn many(
    check: &CheckUseCaseImpl,
    held: &RepositoryFake,
    store: &StoreFake,
) -> Result<Value, Failure> {
    use crate::outbound::check_store::interface::CheckStorePort as _;
    let proofs = store.proofs.held();
    for run in 0_u8..12 {
        if let Ok(mut files) = held.files.lock() {
            drop(files.insert(
                crate::domain::paths::types::RepoPath(format!("src/c/note-{run}.txt").into()),
                crate::domain::files::types::FileText("c\n".into()),
            ));
        }
        drop(check.check(request(Fixing::Report))?);
    }
    let kept = store.answers.checked();
    let named = kept
        .files
        .iter()
        .map(|file| &file.path)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    Ok(
        if store.proofs.held() == proofs && named == kept.files.len() {
            String::new()
        } else {
            format!(
                "the store grew: {} proofs (from {proofs}), {} files kept, {named} of them named once",
                store.proofs.held(),
                kept.files.len()
            )
        },
    )
}

/// Checks again with nothing changed, or a file's text changed, by an engine that fails: the answer is the last
/// report unless the lint call must be made again.
fn rechecked(
    scenario: Scenario,
    check: &CheckUseCaseImpl,
    held: &RepositoryFake,
    store: &Arc<StoreFake>,
) -> Result<super::types::CheckReport, Failure> {
    let _first = check.check(request(Fixing::Report))?;
    if matches!(scenario, Scenario::Moved)
        && let Ok(mut files) = held.files.lock()
    {
        drop(files.insert(
            crate::domain::paths::types::RepoPath("src/a/errors.txt".into()),
            crate::domain::files::types::FileText(
                "// moved, and longer than the text it replaces\n".into(),
            ),
        ));
    }
    let failing = AnalysisFake {
        fails: true,
        ..AnalysisFake::default()
    };
    use_case(held.clone(), failing, store)?.check(CheckRequest {
        within: if matches!(scenario, Scenario::Rescoped) {
            vec![crate::domain::paths::types::RepoPath("src/a".into())]
        } else {
            Vec::new()
        },
        ..request(Fixing::Report)
    })
}

/// The root `tastemaker.toml` the row edits the fixture's to, none when it keeps it.
fn edited_workspace(scenario: Scenario, files: &RepositoryFake) -> Option<String> {
    match scenario {
        Scenario::UnusedVariable => Some(super::fixtures::WORKSPACE.replace(
            "closed = false\n",
            "closed = false\nvariables = { unused = [\"x\"] }\n",
        )),
        Scenario::EntryExcluded => Some(super::fixtures::WORKSPACE.replace(
            "rule = \"p/marker\", include = [\"**\"]",
            "rule = \"p/marker\", include = [\"**\"], exclude = [\"src/a\"]",
        )),
        Scenario::RulesExcluded => Some(super::fixtures::WORKSPACE.replace(
            "name = \"w\",",
            "name = \"w\", rules-exclude = [\"src/a/errors.txt\"],",
        )),
        Scenario::Idle => Some(super::fixtures::WORKSPACE.replace(
            "rule = \"p/marker\", include = [\"**\"]",
            "rule = \"p/marker\", include = [\"nowhere/**\"]",
        )),
        Scenario::Broken => Some(super::fixtures::BROKEN_WORKSPACE.to_owned()),
        Scenario::Local => Some("closed = false\n".to_owned()),
        Scenario::MemberWithWorkspace => Some(super::fixtures::member_workspace(files)),
        Scenario::Placed
        | Scenario::Twice
        | Scenario::Relocked
        | Scenario::CorruptLock
        | Scenario::Unmoved
        | Scenario::Moved
        | Scenario::Rescoped
        | Scenario::Added
        | Scenario::Unlocked
        | Scenario::Unrunnable
        | Scenario::EngineFails
        | Scenario::Nowhere
        | Scenario::Stale
        | Scenario::Nonexistent
        | Scenario::Many
        | Scenario::ReadsNothing
        | Scenario::OneEdit
        | Scenario::FailedKeeps => None,
    }
}

/// The engine the row runs the check over.
fn engine_for(scenario: Scenario) -> AnalysisFake {
    if matches!(scenario, Scenario::EngineFails) {
        AnalysisFake {
            fails: true,
            ..AnalysisFake::default()
        }
    } else if matches!(scenario, Scenario::Unrunnable | Scenario::Idle) {
        AnalysisFake {
            unrunnable: vec!["MARKER".into()],
            ..analysis()
        }
    } else {
        analysis()
    }
}

/// The rules the row expects the check to find, sorted and joined by a space.
fn expected_rules(scenario: Scenario) -> String {
    match scenario {
        Scenario::UnusedVariable => format!("{FOUND} tastemaker#format"),
        Scenario::EntryExcluded | Scenario::RulesExcluded => FOUND.replace("p/marker ", ""),
        Scenario::Unrunnable => FOUND.replace("p/marker ", "p/marker#query "),
        // An entry that reaches no file is a `#coverage` error; the rule the engine cannot run there has nothing to find.
        Scenario::Idle => FOUND.replace("p/marker ", "p/marker#coverage "),
        Scenario::Placed
        | Scenario::Twice
        | Scenario::Relocked
        | Scenario::CorruptLock
        | Scenario::Unmoved
        | Scenario::Moved
        | Scenario::Rescoped
        | Scenario::Added
        | Scenario::Unlocked
        | Scenario::EngineFails
        | Scenario::Nowhere
        | Scenario::Stale
        | Scenario::Broken
        | Scenario::Local
        | Scenario::MemberWithWorkspace
        | Scenario::Nonexistent
        | Scenario::Many
        | Scenario::ReadsNothing
        | Scenario::OneEdit
        | Scenario::FailedKeeps => FOUND.to_owned(),
    }
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
