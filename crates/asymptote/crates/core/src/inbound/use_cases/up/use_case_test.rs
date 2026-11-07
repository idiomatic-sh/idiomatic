//! What the `up` use case promises, against the fakes of the ports it calls, after a check of the scenario.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::errors::UpError;
use super::fixtures::{at, manifest_missing};
use super::interface::UpUseCase as _;
use super::types::UpRequest;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::judgement::types::Verdict;
use crate::domain::operations::types::Revision;
use crate::domain::ratchet::types::Writing;
use crate::domain::renames::types::Head;
use crate::domain::selection::types::{PathPattern, Selectors};
use crate::domain::violations::types::{RepoPath, RuleId};
use crate::inbound::use_cases::check::fixtures::{
    LOG, ROOT, Scenario, request, scenario, up, up_request, use_case, with_sarif,
};
use crate::inbound::use_cases::check::interface::CheckUseCase as _;
use crate::outbound::manifest::fixtures::fake::ManifestFake;
use crate::outbound::repository::fixtures::fake::RepositoryFake;
use crate::outbound::sarif::fixtures::fake::{SarifFake, reading, result};
use crate::outbound::sources::fixtures::fake::SourcesFake;

/// What `up` is asked, and what happens before.
#[derive(Clone, Copy, Debug)]
enum Scene {
    /// No selector.
    Everything,
    /// A rule nothing new is of.
    OtherRule,
    /// The location of one of the two.
    At,
    /// A dry run.
    DryRun,
    /// A location nothing new is at.
    NothingAt,
    /// No check ran first.
    NoReport,
    /// The injected manifest cannot find the requested product.
    ManifestMissing,
    /// The file the check fingerprinted changed since.
    Moved,
    /// The working tree moved to another commit.
    HeadMoved,
    /// The check's tool failed.
    ToolFailed,
    /// A new violation in a file that cannot be read.
    Unkeyed,
    /// A `--path` that is no glob.
    BadGlob,
    /// A new violation at an absolute path, outside the product.
    Outside,
    /// A new violation at a path that leads outside the product through a link.
    Linked,
    /// `asymptote.toml` changed since the check.
    Reconfigured,
    /// 100 rounds, each a check, an edit of the violation on line 2 and an `up` of the new one.
    Rounds,
}

/// `recorded left operations verdict`, the verdict a check after it gives.
type Input = Scene;
type Value = std::borrow::Cow<'static, str>;
type Failure = UpError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_lock_stays_bounded_over_100_ups"),
        word: Word::Succeeds,
        input: || Scene::Rounds,
        expect: Expect::Value(std::borrow::Cow::Borrowed("at most 63 operations")),
    },
    Case {
        name: CaseName("a_failed_manifest_lookup_has_no_ratchet_effects"),
        word: Word::Fails,
        input: || Scene::ManifestMissing,
        expect: Expect::Refusal(|error| match error {
            UpError::Manifest(crate::outbound::manifest::errors::ManifestError::Missing {
                start,
            }) if start == "/missing" => Selection::Matches,
            UpError::Manifest(_)
            | UpError::Lock(_)
            | UpError::Fold(_)
            | UpError::Repository(_)
            | UpError::Matching(_)
            | UpError::NoReport
            | UpError::HeadMoved
            | UpError::ManifestMoved
            | UpError::Moved { .. }
            | UpError::ToolsFailed { .. }
            | UpError::NothingAt { .. }
            | UpError::Outside { .. }
            | UpError::Unkeyed { .. } => Selection::Differs,
        }),
    },
    Case {
        name: CaseName("with_no_selector_every_new_violation_is_recorded"),
        word: Word::Succeeds,
        input: || Scene::Everything,
        expect: Expect::Value(std::borrow::Cow::Borrowed("2 0 1 Pass")),
    },
    Case {
        name: CaseName("a_rule_selector_records_only_that_rule"),
        word: Word::Succeeds,
        input: || Scene::OtherRule,
        expect: Expect::Value(std::borrow::Cow::Borrowed("0 2 0 Changed")),
    },
    Case {
        name: CaseName("an_at_selector_records_the_violation_at_it_and_the_rest_still_fails"),
        word: Word::Succeeds,
        input: || Scene::At,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 1 1 Changed")),
    },
    Case {
        name: CaseName("a_dry_run_writes_nothing"),
        word: Word::Succeeds,
        input: || Scene::DryRun,
        expect: Expect::Value(std::borrow::Cow::Borrowed("2 0 0 Changed")),
    },
    Case {
        name: CaseName("an_at_no_new_violation_is_at_is_refused_with_the_nearest"),
        word: Word::Refuses,
        input: || Scene::NothingAt,
        expect: Expect::Refusal(|error| {
            if matches!(error, UpError::NothingAt { nearest, .. } if nearest.len() == 2) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("up_with_no_check_before_it_is_refused"),
        word: Word::Refuses,
        input: || Scene::NoReport,
        expect: Expect::error(UpError::NoReport),
    },
    Case {
        name: CaseName("a_file_changed_since_the_check_is_refused"),
        word: Word::Refuses,
        input: || Scene::Moved,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(UpError::Moved {
                    first: "src/a.rs".into(),
                    count: 1,
                })
            {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("another_commit_than_the_checks_is_refused"),
        word: Word::Refuses,
        input: || Scene::HeadMoved,
        expect: Expect::error(UpError::HeadMoved),
    },
    Case {
        name: CaseName("a_check_whose_tool_failed_is_refused"),
        word: Word::Refuses,
        input: || Scene::ToolFailed,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(UpError::ToolsFailed {
                    first: "lint".into(),
                    count: 1,
                })
            {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_new_violation_with_no_fingerprint_cannot_be_recorded"),
        word: Word::Refuses,
        input: || Scene::Unkeyed,
        expect: Expect::error(UpError::Unkeyed { count: 1 }),
    },
    Case {
        name: CaseName("a_path_that_is_no_glob_is_refused"),
        word: Word::Refuses,
        input: || Scene::BadGlob,
        expect: Expect::Refusal(|error| {
            if matches!(error, UpError::Matching(_)) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_finding_outside_the_workspace_is_refused"),
        word: Word::Refuses,
        input: || Scene::Outside,
        expect: Expect::Refusal(|error| {
            if matches!(error, UpError::Outside { path } if path == "/elsewhere") {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("up_refuses_when_the_configuration_changed_since_the_check"),
        word: Word::Refuses,
        input: || Scene::Reconfigured,
        expect: Expect::error(UpError::ManifestMoved),
    },
    Case {
        name: CaseName("a_finding_through_a_link_out_of_the_workspace_is_refused"),
        word: Word::Refuses,
        input: || Scene::Linked,
        expect: Expect::Refusal(|error| {
            if matches!(error, UpError::Outside { path } if path == "/elsewhere") {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

/// The scenario `scene` checks first.
fn first(scene: Scene) -> Scenario {
    match scene {
        Scene::ToolFailed => with_sarif(scenario(), SarifFake::default()),
        Scene::Unkeyed => with_sarif(scenario(), reading(LOG, vec![result("src/gone.rs", 1)])),
        Scene::Outside => with_sarif(scenario(), reading(LOG, vec![result("/elsewhere", 1)])),
        Scene::Linked => Scenario {
            sources: Arc::new(SourcesFake {
                outside: [(RepoPath("out".into()), RepoPath("/elsewhere".into()))].into(),
                ..SourcesFake::holding(&[("out", "fn a() {}\n")])
            }),
            ..with_sarif(scenario(), reading(LOG, vec![result("out", 1)]))
        },
        Scene::Everything
        | Scene::OtherRule
        | Scene::At
        | Scene::DryRun
        | Scene::NothingAt
        | Scene::NoReport
        | Scene::ManifestMissing
        | Scene::Moved
        | Scene::HeadMoved
        | Scene::Reconfigured
        | Scene::Rounds
        | Scene::BadGlob => scenario(),
    }
}

fn told(scene: Input) -> Result<String, Failure> {
    let start = first(scene);
    if !matches!(scene, Scene::NoReport | Scene::ManifestMissing) {
        // This check leaves the report `up` reads, so it must succeed.
        if let Err(error) = use_case(&start).check(request()) {
            panic!("the check before up fails: {error:?}");
        }
    }
    let scenario = match scene {
        Scene::HeadMoved => Scenario {
            repository: Arc::new(RepositoryFake {
                head: Head::Commit(Revision("other".into())),
                ..RepositoryFake::default()
            }),
            ..start
        },
        Scene::Moved => {
            start.sources.write("src/a.rs", "changed\n");
            start
        }
        Scene::Reconfigured => {
            // The manifest's file is edited, and reads as the edit says.
            start.sources.write("asymptote.toml", "[tools.other]\n");
            Scenario {
                manifest: Arc::new(ManifestFake::of(ROOT, &["lint", "other"])),
                ..start
            }
        }
        Scene::Everything
        | Scene::OtherRule
        | Scene::At
        | Scene::DryRun
        | Scene::NothingAt
        | Scene::NoReport
        | Scene::ManifestMissing
        | Scene::ToolFailed
        | Scene::Unkeyed
        | Scene::Outside
        | Scene::Linked
        | Scene::Rounds
        | Scene::BadGlob => start,
    };
    let selectors = match scene {
        Scene::OtherRule => Selectors {
            rules: vec![RuleId("other".into())],
            ..Selectors::default()
        },
        Scene::At => Selectors {
            at: vec![at(2)],
            ..Selectors::default()
        },
        Scene::NothingAt => Selectors {
            at: vec![at(9)],
            ..Selectors::default()
        },
        Scene::BadGlob => Selectors {
            paths: vec![PathPattern("a[".into())],
            ..Selectors::default()
        },
        Scene::Everything
        | Scene::DryRun
        | Scene::NoReport
        | Scene::ManifestMissing
        | Scene::Moved
        | Scene::HeadMoved
        | Scene::ToolFailed
        | Scene::Outside
        | Scene::Linked
        | Scene::Reconfigured
        | Scene::Rounds
        | Scene::Unkeyed => Selectors::default(),
    };
    let writing = if matches!(scene, Scene::DryRun) {
        Writing::DryRun
    } else {
        Writing::Write
    };
    let before = scenario.lock.held();
    let saved = scenario.store.saves.load(Ordering::Relaxed);
    let answer = up(&scenario).up(UpRequest {
        writing,
        ..up_request(selectors)
    });
    if answer.is_err() {
        assert_eq!(scenario.lock.held(), before);
        assert_eq!(scenario.store.saves.load(Ordering::Relaxed), saved);
    }
    let report = answer?;
    let after = use_case(&scenario)
        .check(request())
        .map_or(Verdict::Undetermined, |checked| checked.verdict);
    Ok(format!(
        "{} {} {} {after:?}",
        report.recorded.len(),
        report.left.len(),
        scenario.lock.held().len()
    ))
}

/// 100 rounds over one product, each a check, an edit of the violation on line 2 and an `up` that records the new one:
/// how many operations the lock held at most after any of them.
fn rounds() -> Result<String, UpError> {
    let product = scenario();
    let mut most = 0;
    for round in 0..100_u32 {
        product
            .sources
            .write("src/a.rs", &format!("fn a() {{}}\n// round {round}\n"));
        if let Err(error) = use_case(&product).check(request()) {
            panic!("each round's check runs: {error:?}");
        }
        drop(up(&product).up(up_request(Selectors::default()))?);
        most = most.max(product.lock.held().len());
    }
    Ok(format!("at most {most} operations"))
}

fn subject(input: Input) -> Result<Value, Failure> {
    if matches!(input, Scene::Rounds) {
        return rounds().map(std::borrow::Cow::Owned);
    }
    if matches!(input, Scene::ManifestMissing) {
        return manifest_missing().map(std::borrow::Cow::Owned);
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
