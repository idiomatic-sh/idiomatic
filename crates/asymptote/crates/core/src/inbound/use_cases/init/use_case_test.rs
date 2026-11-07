//! What the `init` use case promises, against the fakes of the ports it calls.

use std::sync::Arc;

use super::errors::InitError;
use super::interface::InitUseCase as _;
use super::types::{InitPorts, InitRequest};
use super::use_case::InitUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::hooks::service::script;
use crate::domain::manifest::types::{HookName, ProductRoot};
use crate::domain::ratchet::types::Writing;
use crate::outbound::hooks::errors::HooksError;
use crate::outbound::hooks::fixtures::fake::{HooksFake, Reliability, Toplevel};
use crate::outbound::lock::fixtures::fake::LockFake;
use crate::outbound::manifest::fixtures::fake::ManifestFake;

/// What `init` is asked, and what is there before.
#[derive(Clone, Copy, Debug)]
enum Scene {
    /// A repository with no configuration and no hook.
    Fresh,
    /// A product with a configuration and no hook.
    Configured,
    /// A repository where `init` already ran.
    Rerun,
    /// A configuration that lists no `pre-commit`, over a script `init` wrote for it.
    Unlisted,
    /// A script of another's where `pre-commit` goes.
    Foreign,
    /// A script of another's at a hook that is not listed.
    ForeignUnlisted,
    /// A dry run.
    DryRun,
    /// A repository with no configuration, where `init` runs in a directory below its top level.
    Subdirectory,
    /// A directory in no repository.
    NotARepository,
}

/// `configuration installed removed scripts manifests locks`, the scripts and manifests held after.
type Input = Scene;
type Value = std::borrow::Cow<'static, str>;
type Failure = InitError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_fresh_repository_gets_a_configuration_an_empty_lock_and_the_default_hook",
        ),
        word: Word::Succeeds,
        input: || Scene::Fresh,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "Created pre-commit - pre-commit 1 1 /repo",
        )),
    },
    Case {
        name: CaseName("a_configuration_that_is_there_is_left_alone"),
        word: Word::Succeeds,
        input: || Scene::Configured,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "Kept pre-commit - pre-commit 0 0 -",
        )),
    },
    Case {
        name: CaseName("a_rerun_rewrites_its_own_script_and_changes_nothing_else"),
        word: Word::Succeeds,
        input: || Scene::Rerun,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "Kept pre-commit - pre-commit 0 0 -",
        )),
    },
    Case {
        name: CaseName("the_configuration_is_written_at_the_top_level_of_the_repository"),
        word: Word::Succeeds,
        input: || Scene::Subdirectory,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "Created pre-commit - pre-commit 1 1 /repo",
        )),
    },
    Case {
        name: CaseName("a_dry_run_writes_nothing"),
        word: Word::Succeeds,
        input: || Scene::DryRun,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Created pre-commit - - 0 0 -")),
    },
    Case {
        name: CaseName("a_foreign_script_at_a_hook_not_listed_is_never_touched"),
        word: Word::Succeeds,
        input: || Scene::ForeignUnlisted,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "Created pre-commit - commit-msg,pre-commit 1 1 /repo",
        )),
    },
    Case {
        name: CaseName("a_script_of_its_own_at_a_hook_no_longer_listed_is_removed"),
        word: Word::Succeeds,
        input: || Scene::Unlisted,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "Kept pre-push pre-commit pre-push 0 0 -",
        )),
    },
    Case {
        name: CaseName("a_foreign_script_at_a_listed_hook_is_refused_before_anything_is_written"),
        word: Word::Refuses,
        input: || Scene::Foreign,
        expect: Expect::Refusal(|error| {
            if matches!(error, InitError::Foreign { hook, file } if hook == "pre-commit" && file.ends_with("/pre-commit"))
            {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_directory_in_no_repository_is_refused_before_anything_is_written"),
        word: Word::Refuses,
        input: || Scene::NotARepository,
        expect: Expect::Refusal(|error| {
            if matches!(error, InitError::Hooks(HooksError::NotARepository { .. })) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

const FOREIGN: &str = "#!/bin/sh\necho theirs\n";

/// The product's fakes for a scene.
fn ports(scene: Scene) -> (Arc<ManifestFake>, Arc<LockFake>, Arc<HooksFake>) {
    let pre_commit = script(&HookName("pre-commit".into())).0;
    let manifest = match scene {
        Scene::Fresh
        | Scene::DryRun
        | Scene::NotARepository
        | Scene::ForeignUnlisted
        | Scene::Subdirectory => ManifestFake::of("/elsewhere", &[]),
        Scene::Configured | Scene::Rerun | Scene::Foreign => ManifestFake::of("/repo", &[]),
        Scene::Unlisted => {
            let mut fake = ManifestFake::of("/repo", &[]);
            fake.manifest.hooks = vec![crate::domain::manifest::types::Hook {
                name: HookName("pre-push".into()),
                groups: vec![crate::domain::manifest::types::Group("lint".into())],
            }];
            fake
        }
    };
    let hooks = match scene {
        Scene::Rerun | Scene::Unlisted => HooksFake::default().holding("pre-commit", &pre_commit),
        Scene::Foreign => HooksFake::default().holding("pre-commit", FOREIGN),
        Scene::ForeignUnlisted => HooksFake::default().holding("commit-msg", FOREIGN),
        Scene::NotARepository => HooksFake {
            reliability: Reliability::Fails(HooksError::NotARepository {
                path: "/repo".to_owned(),
            }),
            ..HooksFake::default()
        },
        Scene::Subdirectory => HooksFake {
            top: Toplevel::At(ProductRoot("/repo".into())),
            ..HooksFake::default()
        },
        Scene::Fresh | Scene::Configured | Scene::DryRun => HooksFake::default(),
    };
    (
        Arc::new(manifest),
        Arc::new(LockFake::default()),
        Arc::new(hooks),
    )
}

fn subject(scene: Input) -> Result<Value, Failure> {
    let (manifest, lock, hooks) = ports(scene);
    let init = InitUseCaseImpl::new(InitPorts {
        manifest: Arc::<ManifestFake>::clone(&manifest),
        lock: Arc::<LockFake>::clone(&lock),
        hooks: Arc::<HooksFake>::clone(&hooks),
    });
    let report = init.init(InitRequest {
        start: ProductRoot(
            if matches!(scene, Scene::Subdirectory) {
                "/repo/packages/a"
            } else {
                "/repo"
            }
            .into(),
        ),
        writing: if matches!(scene, Scene::DryRun) {
            Writing::DryRun
        } else {
            Writing::Write
        },
    })?;
    let names = |names: &[HookName]| {
        if names.is_empty() {
            "-".to_owned()
        } else {
            names
                .iter()
                .map(|name| &*name.0)
                .collect::<Vec<_>>()
                .join(",")
        }
    };
    let held: Vec<HookName> = hooks.held().into_keys().collect();
    let (written, at) = {
        let written = manifest
            .written
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (
            written.len(),
            written
                .first()
                .map_or_else(|| "-".to_owned(), |(root, _)| root.0.to_string()),
        )
    };
    Ok(std::borrow::Cow::Owned(format!(
        "{:?} {} {} {} {written} {} {at}",
        report.configuration,
        names(&report.installed),
        names(&report.removed),
        names(&held),
        lock.held().len(),
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
