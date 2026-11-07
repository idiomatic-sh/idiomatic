// ignore: idiomatic.arch/file-length - the use-case spec places one use_case_test.rs per use case and no second one: its case table proves every promise of down over the fakes, the lock's bound over 200 operations among them, and the rows share one scenario, one setup and one runner.
//! What the `down` use case promises, against the fakes of the ports it calls: after a check, an `up`, a fix and a
//! check again.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::errors::DownError;
use super::interface::DownUseCase as _;
use super::types::DownRequest;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::judgement::types::Verdict;
use crate::domain::ratchet::types::Writing;
use crate::domain::selection::types::Selectors;
use crate::inbound::use_cases::check::fixtures::{
    LOG, ROOT, Scenario, down, down_request, request, scenario, up, up_request, use_case,
    with_sarif,
};
use crate::inbound::use_cases::check::interface::CheckUseCase as _;
use crate::inbound::use_cases::up::interface::UpUseCase as _;
use crate::outbound::manifest::fixtures::fake::ManifestFake;
use crate::outbound::sarif::fixtures::fake::{SarifFake, reading, result};

/// What happens between the `up` of both violations and `down`.
#[derive(Clone, Copy, Debug)]
enum Scene {
    /// One of them is fixed.
    Fixed,
    /// Nothing is fixed.
    Nothing,
    /// One is fixed, and `down` is a dry run.
    DryRun,
    /// The tool fails in the check after the fix.
    ToolFailed,
    /// No check ran.
    NoReport,
    /// The injected manifest cannot find the requested product.
    ManifestMissing,
    /// The working tree moved to another commit since the check.
    HeadMoved,
    /// The file the check fingerprinted changed since.
    Moved,
    /// The violations are fixed, the check is clean, and the file is restored: a file no tool lists as an input, which
    /// only the records the check judged name.
    Restored,
    /// `asymptote.toml` changed since the check.
    Reconfigured,
    /// The code of both violations moved to another file, git recording no rename.
    CodeMoved,
    /// The code of one of them moved to another file, the other staying.
    OneMoved,
    /// Two violations of the same code in one file, one of them moved to another file.
    CodeMovedPartly,
    /// 100 rounds of an `up` and a `down`, a line edited between them: the most operations the lock held.
    Rounds,
}

/// `dropped lowered operations verdict`, the verdict a check after it gives; for code that moved, the paths the lock
/// then holds its records at.
type Input = Scene;
type Value = std::borrow::Cow<'static, str>;
type Failure = DownError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_lock_stays_bounded_over_200_ups_and_downs"),
        word: Word::Succeeds,
        input: || Scene::Rounds,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "at most 63 operations, 200 written",
        )),
    },
    Case {
        name: CaseName("a_failed_manifest_lookup_has_no_ratchet_effects"),
        word: Word::Fails,
        input: || Scene::ManifestMissing,
        expect: Expect::Refusal(|error| match error {
            DownError::Manifest(crate::outbound::manifest::errors::ManifestError::Missing {
                start,
            }) if start == "/missing" => crate::cases::Selection::Matches,
            DownError::Manifest(_)
            | DownError::Lock(_)
            | DownError::Fold(_)
            | DownError::Repository(_)
            | DownError::NoReport
            | DownError::HeadMoved
            | DownError::ManifestMoved
            | DownError::Moved { .. }
            | DownError::ToolsFailed { .. } => crate::cases::Selection::Differs,
        }),
    },
    Case {
        name: CaseName("a_fixed_violation_is_drained_from_the_lock"),
        word: Word::Succeeds,
        input: || Scene::Fixed,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 0 2 Pass")),
    },
    Case {
        name: CaseName("with_nothing_fixed_nothing_is_written"),
        word: Word::Succeeds,
        input: || Scene::Nothing,
        expect: Expect::Value(std::borrow::Cow::Borrowed("0 0 1 Pass")),
    },
    Case {
        name: CaseName("a_dry_run_says_what_it_would_drain_and_writes_nothing"),
        word: Word::Succeeds,
        input: || Scene::DryRun,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 0 1 Changed")),
    },
    Case {
        name: CaseName("a_check_whose_tool_failed_is_refused"),
        word: Word::Refuses,
        input: || Scene::ToolFailed,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(DownError::ToolsFailed {
                    first: "lint".into(),
                    count: 1,
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("down_with_no_check_before_it_is_refused"),
        word: Word::Refuses,
        input: || Scene::NoReport,
        expect: Expect::error(DownError::NoReport),
    },
    Case {
        name: CaseName("another_commit_than_the_checks_is_refused"),
        word: Word::Refuses,
        input: || Scene::HeadMoved,
        expect: Expect::error(DownError::HeadMoved),
    },
    Case {
        name: CaseName("a_file_changed_since_the_check_is_refused"),
        word: Word::Refuses,
        input: || Scene::Moved,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(DownError::Moved {
                    first: "src/a.rs".into(),
                    count: 1,
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("down_refuses_when_a_fixed_file_is_restored_after_a_clean_check"),
        word: Word::Refuses,
        input: || Scene::Restored,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(DownError::Moved {
                    first: "src/a.rs".into(),
                    count: 1,
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("down_refuses_when_the_configuration_changed_since_the_check"),
        word: Word::Refuses,
        input: || Scene::Reconfigured,
        expect: Expect::error(DownError::ManifestMoved),
    },
    Case {
        name: CaseName("a_record_whose_code_moved_to_another_file_is_moved_there"),
        word: Word::Succeeds,
        input: || Scene::CodeMoved,
        expect: Expect::Value(std::borrow::Cow::Borrowed("0 0 2 Pass src/b.rs src/b.rs")),
    },
    Case {
        name: CaseName("of_two_records_only_the_one_whose_code_moved_is_moved"),
        word: Word::Succeeds,
        input: || Scene::OneMoved,
        expect: Expect::Value(std::borrow::Cow::Borrowed("0 0 2 Pass src/a.rs src/b.rs")),
    },
    Case {
        name: CaseName("a_record_whose_code_partly_moved_keeps_what_stayed_and_moves_the_rest"),
        word: Word::Succeeds,
        input: || Scene::CodeMovedPartly,
        expect: Expect::Value(std::borrow::Cow::Borrowed("0 0 2 Pass src/a.rs src/b.rs")),
    },
];

/// The scenario, its tool listing no inputs.
fn unlisted() -> Scenario {
    let mut manifest = ManifestFake::of(ROOT, &["lint"]);
    for tool in &mut manifest.manifest.tools {
        tool.inputs.clear();
    }
    Scenario {
        manifest: Arc::new(manifest),
        ..scenario()
    }
}

/// The scenario once its code moved: `src/a.rs` and `src/b.rs` holding `in_a` and `in_b`, the tool reporting `reported`.
fn moved(first: Scenario, in_a: &str, in_b: &str, reported: &[(&str, u32)]) -> Scenario {
    first.sources.write("src/a.rs", in_a);
    first.sources.write("src/b.rs", in_b);
    let results = reported
        .iter()
        .map(|(path, line)| result(path, *line))
        .collect();
    with_sarif(first, reading(LOG, results))
}

/// The path of every violation the lock accepts, in order.
fn held_at(product: &Scenario) -> Result<String, Failure> {
    let folded =
        crate::domain::operations::service::fold(&product.lock.held()).map_err(DownError::Fold)?;
    let mut paths: Vec<&str> = folded
        .records
        .iter()
        .flat_map(|record| {
            std::iter::repeat_n(
                &*record.key.path.0,
                usize::try_from(record.count.0).unwrap_or(usize::MAX),
            )
        })
        .collect();
    paths.sort_unstable();
    Ok(paths.join(" "))
}

/// The product after what changed between the `up` and the check before `down`.
fn changed(scene: Scene, first: Scenario) -> Scenario {
    match scene {
        Scene::Fixed | Scene::DryRun => {
            with_sarif(first, reading(LOG, vec![result("src/a.rs", 1)]))
        }
        Scene::ToolFailed => with_sarif(first, SarifFake::default()),
        Scene::Restored => with_sarif(first, reading(LOG, Vec::new())),
        Scene::CodeMoved => moved(
            first,
            "",
            "fn a() {}\nfn b() {}\n",
            &[("src/b.rs", 1), ("src/b.rs", 2)],
        ),
        Scene::OneMoved => moved(
            first,
            "fn b() {}\n",
            "fn a() {}\n",
            &[("src/a.rs", 1), ("src/b.rs", 1)],
        ),
        Scene::CodeMovedPartly => moved(
            first,
            "fn a() {}\n",
            "fn a() {}\n",
            &[("src/a.rs", 1), ("src/b.rs", 1)],
        ),
        Scene::Nothing
        | Scene::NoReport
        | Scene::ManifestMissing
        | Scene::HeadMoved
        | Scene::Moved
        | Scene::Reconfigured
        | Scene::Rounds => first,
    }
}

fn told(scene: Input) -> Result<String, Failure> {
    let first = if matches!(scene, Scene::Restored) {
        unlisted()
    } else {
        scenario()
    };
    if matches!(scene, Scene::CodeMovedPartly) {
        first.sources.write("src/a.rs", "fn a() {}\nfn a() {}\n");
    }
    if !matches!(scene, Scene::NoReport | Scene::ManifestMissing) {
        // The setup leaves a report and a lock behind, so its runs must succeed.
        if let Err(error) = use_case(&first).check(request()) {
            panic!("the setup check fails: {error:?}");
        }
        if let Err(error) = up(&first).up(up_request(Selectors::default())) {
            panic!("the setup up fails: {error:?}");
        }
    }
    let after = changed(scene, first);
    if matches!(
        scene,
        Scene::Fixed | Scene::DryRun | Scene::ToolFailed | Scene::Restored
    ) {
        after.sources.write("src/a.rs", "fn a() {}\n// fixed\n");
    }
    if !matches!(scene, Scene::NoReport | Scene::ManifestMissing) {
        // This check leaves the report `down` reads, so it must succeed.
        if let Err(error) = use_case(&after).check(request()) {
            panic!("the check before down fails: {error:?}");
        }
    }
    let after = match scene {
        Scene::HeadMoved => Scenario {
            repository: Arc::new(
                crate::outbound::repository::fixtures::fake::RepositoryFake {
                    head: crate::domain::renames::types::Head::Commit(
                        crate::domain::operations::types::Revision("other".into()),
                    ),
                    ..crate::outbound::repository::fixtures::fake::RepositoryFake::default()
                },
            ),
            ..after
        },
        Scene::Moved => {
            after.sources.write("src/a.rs", "changed\n");
            after
        }
        Scene::Restored => {
            after.sources.write("src/a.rs", "fn a() {}\nfn b() {}\n");
            after
        }
        Scene::Reconfigured => {
            // The manifest's file is edited, and reads as the edit says.
            after.sources.write("asymptote.toml", "[tools.other]\n");
            Scenario {
                manifest: Arc::new(ManifestFake::of(ROOT, &["lint", "other"])),
                ..after
            }
        }
        Scene::Fixed
        | Scene::Nothing
        | Scene::DryRun
        | Scene::ToolFailed
        | Scene::NoReport
        | Scene::ManifestMissing
        | Scene::CodeMoved
        | Scene::OneMoved
        | Scene::CodeMovedPartly
        | Scene::Rounds => after,
    };
    let writing = if matches!(scene, Scene::DryRun) {
        Writing::DryRun
    } else {
        Writing::Write
    };
    let before = after.lock.held();
    let saved = after.store.saves.load(Ordering::Relaxed);
    let answer = down(&after).down(DownRequest {
        writing,
        ..down_request()
    });
    if answer.is_err() {
        assert_eq!(after.lock.held(), before);
        assert_eq!(after.store.saves.load(Ordering::Relaxed), saved);
    }
    let report = answer?;
    let verdict = use_case(&after)
        .check(request())
        .map_or(Verdict::Undetermined, |checked| checked.verdict);
    let said = format!(
        "{} {} {} {verdict:?}",
        report.dropped.len(),
        report.lowered.len(),
        after.lock.held().len()
    );
    if matches!(
        scene,
        Scene::CodeMoved | Scene::OneMoved | Scene::CodeMovedPartly
    ) {
        Ok(format!("{said} {}", held_at(&after)?))
    } else {
        Ok(said)
    }
}

/// 100 rounds over one product, each a check, an `up`, an edit of the violation on line 2, a check and a `down`: how
/// many operations the lock held at most after any of them, and how many were written.
fn rounds() -> Result<String, Failure> {
    let product = scenario();
    let mut most = 0;
    let mut written = 0;
    for round in 0..100_u32 {
        if let Err(error) = use_case(&product).check(request()) {
            panic!("each round's first check runs: {error:?}");
        }
        let before = product.lock.appended.load(Ordering::Relaxed);
        if let Err(error) = up(&product).up(up_request(Selectors::default())) {
            panic!("each round's up runs: {error:?}");
        }
        most = most.max(product.lock.held().len());
        product
            .sources
            .write("src/a.rs", &format!("fn a() {{}}\n// round {round}\n"));
        if let Err(error) = use_case(&product).check(request()) {
            panic!("each round's second check runs: {error:?}");
        }
        drop(down(&product).down(down_request())?);
        most = most.max(product.lock.held().len());
        // A snapshot is one more operation appended.
        written += product
            .lock
            .appended
            .load(Ordering::Relaxed)
            .saturating_sub(before)
            .min(2);
    }
    Ok(format!("at most {most} operations, {written} written"))
}

fn subject(input: Input) -> Result<Value, Failure> {
    if matches!(input, Scene::Rounds) {
        return rounds().map(std::borrow::Cow::Owned);
    }
    if matches!(input, Scene::ManifestMissing) {
        let product = scenario();
        let before = product.lock.held();
        let saves = product.store.saves.load(Ordering::Relaxed);
        let result = down(&product).down(DownRequest {
            start: crate::domain::manifest::types::ProductRoot("/missing".into()),
            ..down_request()
        });
        assert_eq!(product.lock.held(), before);
        assert_eq!(product.store.saves.load(Ordering::Relaxed), saves);
        return result.map(|_| std::borrow::Cow::Borrowed("unexpected success"));
    }

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
