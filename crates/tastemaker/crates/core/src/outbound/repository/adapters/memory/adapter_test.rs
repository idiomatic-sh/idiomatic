//! The memory adapter of the repository port: the contract over a scratch directory, then each tree walked, found and read
//! as the disk adapter walks, finds and reads the same tree written to a disk.

use super::adapter::RepositoryMemoryAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::{
    Declarer, DirectoryName, FileText, MachinePath, SkippedDirectory,
};
use crate::domain::paths::types::RepoPath;
use crate::outbound::repository::adapters::disk::adapter::RepositoryDiskAdapter;
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::fixtures::contract::check;
use crate::outbound::repository::interface::{Repository, RepositoryPort as _};

/// The directories the linked languages declare skipped in every tree below.
fn skipped() -> Vec<SkippedDirectory> {
    vec![SkippedDirectory {
        name: DirectoryName("node_modules".into()),
        by: vec![Declarer("javascript".into())],
    }]
}

/// The adapter, `node_modules` declared skipped.
fn memory() -> RepositoryMemoryAdapter {
    RepositoryMemoryAdapter {
        skipped: skipped(),
        store: DirectoryName(".tastemaker_cache".into()),
        trees: std::sync::Arc::default(),
    }
}

/// Where the memory adapter's project is: the root every row chooses.
fn root() -> MachinePath {
    MachinePath("/memory-adapter-test/project".into())
}

/// A tree a row writes to both adapters: each file by its path and its text.
type Tree = &'static [(&'static str, &'static str)];

/// What a row asks of a tree written to memory and to a disk.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The tree is walked from `under`: the memory adapter must answer what the disk adapter answers.
    Walked(Tree, &'static str),
    /// The workspace is looked for from `start`: the memory adapter must find what the disk adapter finds.
    Found(Tree, &'static str),
    /// A file of the tree is read.
    Read(Tree, &'static str),
    /// A file that is not in the tree is read: its refusal says what the disk's says.
    ReadMissing,
    /// A path that names no scratch directory is walked: nothing is there.
    NoScratch,
    /// More scratch directories are asked for than the adapter may hold at once: the next is refused.
    TreesWithinTheirBound,
}

/// A repository with ignore files at the root and below, a negation, a directory pattern, a `.ignore` overriding a
/// `.gitignore`, the store, a nested `.git` and a skipped directory.
const IGNORED: Tree = &[
    (".git/HEAD", "ref: refs/heads/main\n"),
    (".gitignore", "*.log\nbuild/\n!keep.log\n/rooted.txt\n"),
    ("a.log", "x"),
    ("keep.log", "x"),
    ("rooted.txt", "x"),
    ("src/rooted.txt", "x"),
    ("src/main.rs", "fn main() {}\n"),
    ("src/build/out.rs", "x"),
    ("build/out.rs", "x"),
    ("docs/.gitignore", "draft.md\n"),
    ("docs/draft.md", "x"),
    ("docs/final.md", "x"),
    ("docs/.ignore", "!draft.md\nfinal.md\n"),
    ("docs/deep/draft.md", "x"),
    ("node_modules/x.js", "x"),
    ("web/node_modules/y.js", "x"),
    (".tastemaker_cache/check.store", "x"),
    ("only-ignored/b.log", "x"),
    ("tastemaker.toml", "version = 3\n"),
    ("pkg/tastemaker.toml", "version = 3\n"),
    ("pkg/.gitignore", "*.rs\n"),
    ("pkg/lib.rs", "x"),
    ("nested/.git/HEAD", "ref: refs/heads/main\n"),
    ("nested/.gitignore", "*.txt\n"),
    ("nested/a.txt", "x"),
    ("nested/a.log", "x"),
];

/// A plain repository: no ignore file.
const PLAIN: Tree = &[
    (".git/HEAD", "ref: refs/heads/main\n"),
    ("tastemaker.toml", "version = 3\n"),
    ("src/a.rs", "x"),
    ("src/b/c.rs", "x"),
    ("README.md", "x"),
];

/// A tree with no `.git`: no repository.
const NO_GIT: Tree = &[("tastemaker.toml", "version = 3\n"), ("a.rs", "x")];

/// A repository with no `tastemaker.toml`.
const NO_WORKSPACE: Tree = &[(".git/HEAD", "ref: refs/heads/main\n"), ("a.rs", "x")];

type Input = Scenario;
/// What the memory adapter answered, when the disk adapter answered the same; else both answers.
type Value = String;
type Failure = RepositoryError;
/// One tree twice: the disk adapter, the temporary directory it is bound to, the memory adapter's scratch directory, and
/// the temporary directory itself.
type Both = (
    RepositoryDiskAdapter,
    std::sync::Arc<dyn Repository>,
    Box<dyn Repository>,
    tempfile::TempDir,
);

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("scratch_directories_stay_within_their_bound_and_the_next_is_refused"),
        word: Word::Refuses,
        input: || Scenario::TreesWithinTheirBound,
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::Write { reason, .. } if reason.contains("as many scratch"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_tree_with_ignore_files_is_walked_from_its_root_as_on_a_disk"),
        word: Word::Succeeds,
        input: || Scenario::Walked(IGNORED, ""),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_tree_with_ignore_files_is_walked_from_below_its_root_as_on_a_disk"),
        word: Word::Succeeds,
        input: || Scenario::Walked(IGNORED, "docs"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_root_inside_a_skipped_directory_is_walked_as_on_a_disk"),
        word: Word::Succeeds,
        input: || Scenario::Walked(IGNORED, "web/node_modules"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_walked_is_the_walk_as_on_a_disk"),
        word: Word::Succeeds,
        input: || Scenario::Walked(PLAIN, "src/a.rs"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_that_is_not_there_is_walked_as_on_a_disk"),
        word: Word::Succeeds,
        input: || Scenario::Walked(PLAIN, "absent"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_plain_tree_is_walked_as_on_a_disk"),
        word: Word::Succeeds,
        input: || Scenario::Walked(PLAIN, ""),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_workspace_at_the_root_is_found_as_on_a_disk"),
        word: Word::Succeeds,
        input: || Scenario::Found(IGNORED, ""),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_nearest_workspace_above_a_start_is_found_as_on_a_disk"),
        word: Word::Succeeds,
        input: || Scenario::Found(IGNORED, "pkg"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_root_workspace_is_found_from_a_directory_without_one_as_on_a_disk"),
        word: Word::Succeeds,
        input: || Scenario::Found(IGNORED, "src"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_written_is_read_back"),
        word: Word::Succeeds,
        input: || Scenario::Read(PLAIN, "src/b/c.rs"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_that_names_no_scratch_directory_holds_nothing"),
        word: Word::Succeeds,
        input: || Scenario::NoScratch,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_tree_without_git_is_in_no_repository"),
        word: Word::Refuses,
        input: || Scenario::Found(NO_GIT, ""),
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::NoRepository { .. }) {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_repository_without_a_tastemaker_toml_has_no_workspace"),
        word: Word::Refuses,
        input: || Scenario::Found(NO_WORKSPACE, ""),
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::NoWorkspace { .. }) {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_file_that_is_not_there_is_refused_as_on_a_disk"),
        word: Word::Refuses,
        input: || Scenario::ReadMissing,
        expect: Expect::Refusal(|error| {
            // A missing file in a directory that is there: Windows says otherwise of a path whose directory is missing.
            let on_disk = std::fs::read_to_string(
                std::env::temp_dir().join("tastemaker-memory-adapter-never.txt"),
            )
            .err()
            .map(|failure| failure.to_string());
            if matches!(error, RepositoryError::Read { path, reason } if path == "never.txt" && Some(reason) == on_disk.as_ref())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// Writes `tree` into `repository`.
fn write(repository: &dyn Repository, tree: Tree) -> Result<(), Failure> {
    for (path, text) in tree {
        repository.write(&RepoPath((*path).into()), &FileText((*text).into()))?;
    }
    Ok(())
}

/// `tree` written to a temporary directory and to a scratch directory of `adapter`: the disk adapter over the first,
/// the temporary directory bound, the scratch directory, and the temporary directory, each removed when it is dropped.
fn both(adapter: &RepositoryMemoryAdapter, tree: Tree) -> Result<Both, Failure> {
    let directory = tempfile::tempdir().map_err(|error| RepositoryError::Write {
        path: String::new(),
        reason: error.to_string(),
    })?;
    let on_disk = MachinePath(directory.path().display().to_string().into());
    let disk = RepositoryDiskAdapter {
        skipped: skipped(),
        store: DirectoryName(".tastemaker_cache".into()),
    };
    let on_disk = disk.open(&on_disk);
    write(&*on_disk, tree)?;
    let scratch = adapter.project(&root())?;
    write(&*scratch, tree)?;
    Ok((disk, on_disk, scratch, directory))
}

/// Nothing when the two answers are one, else both.
fn same<T>(in_memory: &T, on_disk: &T) -> Value
where
    T: std::fmt::Debug + PartialEq,
{
    if in_memory == on_disk {
        String::new()
    } else {
        format!("in memory {in_memory:?}, on disk {on_disk:?}")
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let adapter = memory();
    match scenario {
        Scenario::Walked(tree, under) => {
            let (_disk, on_disk, scratch, _directory) = both(&adapter, tree)?;
            let under = RepoPath(under.into());
            Ok(same(&scratch.walk(&under)?, &on_disk.walk(&under)?))
        }
        Scenario::Found(tree, start) => {
            let (disk, on_disk, _scratch, _directory) = both(&adapter, tree)?;
            let on_disk = on_disk.root().clone();
            let at = |root: &MachinePath| {
                MachinePath(if start.is_empty() {
                    root.0.clone()
                } else {
                    format!("{}/{start}", root.0).into()
                })
            };
            let found = adapter.find(&at(&root()))?;
            let on_disk_found = disk.find(&at(&on_disk))?;
            Ok(same(
                &(
                    found.workspace().clone(),
                    found.start().clone(),
                    *found.root() == root(),
                ),
                &(
                    on_disk_found.workspace().clone(),
                    on_disk_found.start().clone(),
                    true,
                ),
            ))
        }
        Scenario::Read(tree, path) => {
            let (_disk, on_disk, scratch, _directory) = both(&adapter, tree)?;
            let path = RepoPath(path.into());
            Ok(same(&scratch.read(&path)?, &on_disk.read(&path)?))
        }
        Scenario::ReadMissing => {
            let scratch = adapter.project(&root())?;
            scratch
                .read(&RepoPath("never.txt".into()))
                .map(|text| text.0.into_string())
        }
        Scenario::TreesWithinTheirBound => {
            use super::constants::TREES;
            use super::types::Trees;
            let kept: Vec<_> = (0..TREES)
                .map(|place| adapter.project(&MachinePath(format!("/scratch/{place}").into())))
                .collect::<Result<_, _>>()?;
            let Trees(held) = &*adapter.trees;
            let within = held.lock().map_or(0, |held| held.len()) == kept.len();
            let over = adapter.project(&MachinePath("/scratch/over".into()));
            if !within {
                return Ok("Trees holds more than its bound".to_owned());
            }
            over.map(|_| String::new())
        }
        Scenario::NoScratch => {
            let walk = adapter
                .open(&MachinePath("/nowhere".into()))
                .walk(&RepoPath::default())?;
            Ok(same(
                &walk,
                &crate::outbound::repository::types::Walk::default(),
            ))
        }
    }
}

#[test]
fn succeeds() {
    let adapter = memory();
    let scratch = adapter.project(&root()).expect("a scratch directory");
    scratch
        .write(
            &RepoPath(".git/HEAD".into()),
            &FileText("ref: refs/heads/main\n".into()),
        )
        .expect("a repository");
    check(&adapter, &root());
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
