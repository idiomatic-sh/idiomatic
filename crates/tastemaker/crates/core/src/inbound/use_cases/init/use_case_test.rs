//! Initialization preserves user files and makes a usable local package.
use super::{
    errors::InitError, interface::InitUseCase as _, types::InitRequest, use_case::InitUseCaseImpl,
};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::files::types::MachinePath;
use crate::domain::paths::types::RepoPath;
use crate::domain::writing::types::Writing;
use crate::outbound::manifest::interface::ManifestPort as _;
use crate::outbound::repository::{fixtures::fake::RepositoryFake, interface::RepositoryPort as _};
use std::sync::Arc;
/// What the project holds before `init`.
#[derive(Clone, Copy, Debug)]
enum Project {
    /// Nothing, or the one file the row names.
    Bare,
    /// A README, sources in two directories and a dotfile directory: the closed workspace places each.
    Holding,
    /// Nothing, in no git repository.
    Unversioned,
}
type Input = (Writing, Option<&'static str>, bool, Project);
type Value = usize;
type Failure = InitError;
/// What a holding project's `init` must write: the root, closed by default and so saying nothing of it, an entry per top-level path, its name written as a TOML
/// string whatever it holds (a name that spells one of the entry's placeholders included, never substituted again), the
/// copied template and the lock.
const HOLDING_WRITES: [&str; 13] = [
    concat!("tastemaker = \"", env!("CARGO_PKG_VERSION"), "\""),
    "version = 1",
    "workspace = { name = \"local\"",
    "file = \"README.md\"",
    "file = \"src/{*src}\"",
    "file = \".github/{*github}\"",
    "file = \"a\\\"b.txt\"",
    "file = \"data[1]/{*data-1}\"",
    // A backslash and a wildcard are the name: escaped for the path pattern, then each backslash escaped for TOML.
    "file = \"a\\\\\\\\b.txt\"",
    "file = \"x\\\\*y/{*x-y}\"",
    "file = \"<capture>/{*capture}\"",
    "unchecked = \"0.1.0\"",
    "content = ''",
];
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_holding_project_is_closed_over_every_top_level_path"),
        word: Word::Succeeds,
        input: || (Writing::Write, None, false, Project::Holding),
        expect: Expect::Value(HOLDING_WRITES.len()),
    },
    Case {
        name: CaseName("fresh"),
        word: Word::Succeeds,
        input: || (Writing::Write, None, false, Project::Bare),
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("dry_run"),
        word: Word::Succeeds,
        input: || (Writing::DryRun, None, false, Project::Bare),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("workspace_exists"),
        word: Word::Refuses,
        input: || {
            (
                Writing::Write,
                Some("tastemaker.toml"),
                false,
                Project::Bare,
            )
        },
        expect: Expect::Refusal(|error| match error {
            InitError::Exists { file } if file == "tastemaker.toml" => Selection::Matches,
            InitError::Exists { .. } | InitError::Repository(_) => Selection::Differs,
        }),
    },
    Case {
        name: CaseName("a_project_in_no_git_repository_is_refused_before_a_write"),
        word: Word::Refuses,
        input: || (Writing::Write, None, false, Project::Unversioned),
        expect: Expect::Refusal(|error| match error {
            InitError::Repository(
                crate::outbound::repository::errors::RepositoryError::NoRepository { workspace },
            ) if workspace == "/project" => Selection::Matches,
            InitError::Exists { .. } | InitError::Repository(_) => Selection::Differs,
        }),
    },
    Case {
        name: CaseName("package_exists"),
        word: Word::Refuses,
        input: || {
            (
                Writing::Write,
                Some("tastemaker/package.toml"),
                false,
                Project::Bare,
            )
        },
        expect: Expect::Refusal(|error| match error {
            InitError::Exists { file } if file == "tastemaker/package.toml" => Selection::Matches,
            InitError::Exists { .. } | InitError::Repository(_) => Selection::Differs,
        }),
    },
    Case {
        name: CaseName("a_failed_first_write_prevents_configuration_publication"),
        word: Word::Fails,
        input: || (Writing::Write, None, true, Project::Bare),
        expect: Expect::Refusal(|error| match error {
            InitError::Repository(
                crate::outbound::repository::errors::RepositoryError::Write { path, reason },
            ) if path == "tastemaker/package.toml" && reason == "injected write failure" => {
                Selection::Matches
            }
            InitError::Exists { .. } | InitError::Repository(_) => Selection::Differs,
        }),
    },
];
/// How many of [`HOLDING_WRITES`] `init` writes into a project that already holds files.
fn holding() -> Result<Value, Failure> {
    let repository = RepositoryFake::of(&[
        ("README.md", "# project"),
        ("src/main.py", "print()"),
        ("src/lib/util.py", "x = 1"),
        (".github/workflows/ci.yml", "on: push"),
        ("a\"b.txt", "a quote in its name"),
        ("data[1]/rows.csv", "a bracket in its directory's name"),
        ("a\\b.txt", "a backslash in its name"),
        ("x*y/rows.csv", "a wildcard in its directory's name"),
        (
            "<capture>/notes.txt",
            "a directory named as the entry's own placeholder",
        ),
        // A nested workspace: its tree is no tree of the project's workspace, so no entry names it.
        (
            "nested/tastemaker.toml",
            "workspace = { name = \"nested\" }\n",
        ),
        ("nested/src/lib.py", "x = 2"),
        // A local file declares no workspace: its directory is governed, and an entry names it.
        ("local/tastemaker.toml", "rules = []\n"),
        ("local/lib.py", "x = 3"),
    ]);
    let target = MachinePath("/project".into());
    let report = InitUseCaseImpl::new(
        Arc::new(repository.clone()),
        Arc::new(crate::outbound::manifest::factory::toml()),
    )
    .init(InitRequest {
        target: target.clone(),
        writing: Writing::Write,
    })?;
    let written = report
        .written
        .iter()
        .map(|path| {
            repository
                .open(&target)
                .read(path)
                .map(|text| text.0.into_string())
        })
        .collect::<Result<Vec<_>, _>>()?
        .concat();
    // A nested workspace is named by no entry; a directory with only a local file is.
    if written.contains("nested") || !written.contains("file = \"local/{*local}\"") {
        return Ok(0);
    }
    // The workspace file reads as a workspace: a name is never pasted into it raw.
    let workspace = repository
        .open(&target)
        .read(&RepoPath("tastemaker.toml".into()))?;
    // A root is closed by default, so the file says nothing of it.
    if workspace.0.contains("closed") {
        return Ok(0);
    }
    let reads = crate::outbound::manifest::factory::toml()
        .workspace_file(&crate::outbound::manifest::types::DefinitionText(
            workspace.0,
        ))
        .ok()
        .is_some_and(|read| matches!(read.role, crate::domain::definitions::types::Role::Root(_)));
    Ok(usize::from(reads)
        * HOLDING_WRITES
            .iter()
            .filter(|expected| written.contains(*expected))
            .count())
}
fn subject((writing, existing, fail_write, project): Input) -> Result<Value, Failure> {
    if let Project::Holding = project {
        return holding();
    }
    let files: Vec<_> = existing
        .into_iter()
        .map(|path| (path, "user content"))
        .collect();
    let mut repository = RepositoryFake::of(&files);
    if let Project::Unversioned = project {
        repository.workspace =
            crate::outbound::repository::fixtures::fake::FakeWorkspace::Unversioned;
    }
    if fail_write {
        let _: bool = repository
            .write_failures
            .insert(RepoPath("tastemaker/package.toml".into()));
    }
    let target = MachinePath("/project".into());
    let result = InitUseCaseImpl::new(
        Arc::new(repository.clone()),
        Arc::new(crate::outbound::manifest::factory::toml()),
    )
    .init(InitRequest {
        target: target.clone(),
        writing,
    });
    if let Some(path) = existing {
        assert_eq!(
            repository
                .open(&target)
                .read(&RepoPath(path.into()))?
                .0
                .as_ref(),
            "user content"
        );
        assert_eq!(
            repository.files.lock().expect("fake repository lock").len(),
            1
        );
    }
    if fail_write || matches!(project, Project::Unversioned) {
        assert_eq!(
            repository.files.lock().expect("fake repository lock").len(),
            0
        );
    }
    let report = result?;
    assert_eq!(
        report.written,
        vec![
            RepoPath("tastemaker/package.toml".into()),
            RepoPath("tastemaker/templates/unchecked/0.1.0/template.toml".into()),
            RepoPath("tastemaker.lock".into()),
            RepoPath("tastemaker.toml".into())
        ]
    );
    Ok(repository.files.lock().expect("fake repository lock").len())
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
