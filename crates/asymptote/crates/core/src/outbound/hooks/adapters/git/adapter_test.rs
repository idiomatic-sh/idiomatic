//! The git adapter of the hooks port: the contract over a repository made in a temporary directory, a directory that
//! is no repository, a hooks directory `core.hooksPath` moves, a linked worktree, and the mode a script is written
//! with.

use std::path::Path;
use std::process::Command;

use super::adapter::HooksGitAdapter;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::hooks::service::{ownership, script as script_of};
use crate::domain::hooks::types::{Ownership, ScriptText};
use crate::domain::manifest::types::{HookName, ProductRoot};
use crate::outbound::hooks::errors::HooksError;
use crate::outbound::hooks::fixtures::contract::check;
use crate::outbound::hooks::interface::HooksPort as _;
use crate::outbound::hooks::types::Installed;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The contract.
    Contract,
    /// A directory that is no repository.
    NotARepository,
    /// A repository whose `core.hooksPath` names another directory.
    HooksPath,
    /// A linked worktree of a repository.
    Worktree,
    /// A script is written executable.
    Executable,
    /// A hook that is a directory, which cannot be read as a script.
    Unreadable,
    /// A hooks directory that is a file, which a script cannot be written under.
    Unwritable,
    /// A repository whose configuration git cannot read.
    GitFails,
    /// A hook that is a symlink to nothing.
    DanglingLink,
    /// A hook that is a symlink to a script asymptote wrote.
    LinkToOurScript,
}

type Input = Question;
/// Whether the answer is the expected one.
type Value = bool;
type Failure = HooksError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_adapter_keeps_the_contract"),
        word: Word::Succeeds,
        input: || Question::Contract,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_script_goes_where_core_hooks_path_says"),
        word: Word::Succeeds,
        input: || Question::HooksPath,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_linked_worktree_shares_the_hooks_of_its_repository"),
        word: Word::Succeeds,
        input: || Question::Worktree,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_script_is_written_executable"),
        word: Word::Succeeds,
        input: || Question::Executable,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_dangling_symlink_is_a_foreign_hook"),
        word: Word::Succeeds,
        input: || Question::DanglingLink,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_symlink_to_a_script_asymptote_wrote_is_a_foreign_hook"),
        word: Word::Succeeds,
        input: || Question::LinkToOurScript,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_hook_that_cannot_be_read_is_refused"),
        word: Word::Refuses,
        input: || Question::Unreadable,
        expect: Expect::Refusal(|error| {
            if matches!(error, HooksError::Unreadable { path, .. } if path.ends_with("pre-commit"))
            {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_script_that_cannot_be_written_is_refused"),
        word: Word::Refuses,
        input: || Question::Unwritable,
        expect: Expect::Refusal(|error| {
            if matches!(error, HooksError::Unwritable { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_repository_git_cannot_read_is_refused_with_what_git_said"),
        word: Word::Refuses,
        input: || Question::GitFails,
        expect: Expect::Refusal(|error| {
            if matches!(error, HooksError::Git { reason } if !reason.is_empty()) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_directory_in_no_repository_is_refused"),
        word: Word::Refuses,
        input: || Question::NotARepository,
        expect: Expect::Refusal(|error| {
            if matches!(error, HooksError::NotARepository { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

fn git(directory: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(arguments)
        .current_dir(directory)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {arguments:?}: {output:?}");
}

fn root(directory: &Path) -> ProductRoot {
    ProductRoot(directory.display().to_string().into())
}

fn name() -> HookName {
    HookName("pre-commit".into())
}

fn script() -> ScriptText {
    ScriptText("#!/bin/sh\nexit 0\n".into())
}

/// The permission bits of the file at `path`.
#[cfg(unix)]
fn mode_of(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::metadata(path)
        .expect("the script")
        .permissions()
        .mode()
}

/// A file system with no executable bit reports every bit set.
#[cfg(not(unix))]
fn mode_of(_path: &Path) -> u32 {
    0o777
}

/// Makes `at` a symlink to `target`.
#[cfg(unix)]
fn link(target: &Path, at: &Path) {
    std::os::unix::fs::symlink(target, at).expect("a symlink");
}

/// Whether the hook at `path`, a symlink, is read as a foreign one. A file system without symlinks has none to read.
#[cfg(unix)]
fn foreign(path: &Path) -> Result<Value, Failure> {
    Ok(
        matches!(HooksGitAdapter.inspect(&root(path), &name())?, Installed::Script { text, .. } if ownership(&text) == Ownership::Foreign),
    )
}

/// A file system without symlinks has none to read.
#[cfg(not(unix))]
fn foreign(_path: &Path) -> Result<Value, Failure> {
    Ok(true)
}

fn subject(question: Input) -> Result<Value, Failure> {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path();
    match question {
        Question::Contract => {
            git(path, &["init", "-q"]);
            check(&HooksGitAdapter, &root(path));
            Ok(true)
        }
        Question::NotARepository => HooksGitAdapter.inspect(&root(path), &name()).map(|_| true),
        Question::HooksPath => {
            git(path, &["init", "-q"]);
            git(path, &["config", "core.hooksPath", "custom-hooks"]);
            HooksGitAdapter.install(&root(path), &name(), &script())?;
            Ok(path.join("custom-hooks/pre-commit").is_file())
        }
        Question::Worktree => {
            git(path, &["init", "-q"]);
            git(path, &["commit", "-q", "--allow-empty", "-m", "first"]);
            let linked = path.join("linked");
            git(path, &["worktree", "add", "-q", "linked"]);
            HooksGitAdapter.install(&root(&linked), &name(), &script())?;
            Ok(path.join(".git/hooks/pre-commit").is_file()
                && matches!(HooksGitAdapter.inspect(&root(path), &name())?, Installed::Script { text, .. } if text == script()))
        }
        Question::Executable => {
            git(path, &["init", "-q"]);
            HooksGitAdapter.install(&root(path), &name(), &script())?;
            Ok(mode_of(&path.join(".git/hooks/pre-commit")) & 0o111 == 0o111)
        }
        Question::DanglingLink => {
            git(path, &["init", "-q"]);
            #[cfg(unix)]
            link(&path.join("nowhere"), &path.join(".git/hooks/pre-commit"));
            foreign(path)
        }
        Question::LinkToOurScript => {
            git(path, &["init", "-q"]);
            let ours = path.join("ours");
            std::fs::write(&ours, script_of(&name()).0.as_bytes()).expect("a script");
            #[cfg(unix)]
            link(&ours, &path.join(".git/hooks/pre-commit"));
            foreign(path)
        }
        Question::Unreadable => {
            git(path, &["init", "-q"]);
            std::fs::create_dir_all(path.join(".git/hooks/pre-commit")).expect("a directory");
            HooksGitAdapter.inspect(&root(path), &name()).map(|_| true)
        }
        Question::Unwritable => {
            git(path, &["init", "-q"]);
            std::fs::remove_dir_all(path.join(".git/hooks")).expect("no hooks directory");
            std::fs::write(path.join(".git/hooks"), "a file").expect("a file");
            HooksGitAdapter
                .install(&root(path), &name(), &script())
                .map(|()| true)
        }
        Question::GitFails => {
            git(path, &["init", "-q"]);
            std::fs::write(path.join(".git/config"), "[core\n").expect("a broken configuration");
            HooksGitAdapter.inspect(&root(path), &name()).map(|_| true)
        }
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
