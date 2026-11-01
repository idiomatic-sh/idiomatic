// ignore: idiomatic.arch/file-length - the adapter spec places one adapter_test.rs per adapter and no second one: its case table holds every answer of the repository port over a temporary repository (the walk, its skips, the workspace search, reads and writes, and a path whose metadata cannot be read), each a scene of its own.
//! The disk adapter of the repository port: the contract over a temporary repository, then the search for a workspace.

use super::adapter::RepositoryDiskAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::{
    Declarer, DirectoryName, FileText, MachinePath, SkippedDirectory,
};
use crate::domain::paths::types::RepoPath;
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::fixtures::contract::check;
use crate::outbound::repository::interface::RepositoryPort as _;

/// The adapter with no language linked.
fn disk() -> RepositoryDiskAdapter {
    RepositoryDiskAdapter {
        skipped: Vec::new(),
        store: DirectoryName(".tastemaker_cache".into()),
    }
}

/// What a row asks of a temporary directory.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The workspace is looked for, the directory holding `.git` and a `tastemaker.toml` as said.
    Found(bool, bool),
    /// The workspace is looked for from a repository holding no `tastemaker.toml`, inside a directory that holds one.
    AboveRepository,
    /// The repository is looked for from a directory below the root, the root holding `.git` as said.
    Repository(bool),
    /// A file never written is read.
    ReadMissing,
    /// A path whose metadata cannot be read (a link to itself) is asked for, then read: it is present, and its read refused.
    Looped,
    /// A tree holding a name that is not UTF-8 is listed: the walk is refused with its path, never leaves it out unseen.
    NotUtf8,
    /// A file is written below a file.
    WriteBelowFile,
    /// A repository with no `.gitignore` is listed, the directories of the named languages declared skipped: nothing when the files found
    /// are the ones named, and the files found when they are not.
    Lists(&'static [(&'static str, &'static str)], &'static str),
    /// The same repository with `node_modules` declared skipped is listed from the directory named, which is read all the same: only a
    /// directory the walk comes upon is skipped, never the one a root or an include names.
    Under(&'static str, &'static str),
    /// A repository whose `kept.txt` only the user's global git excludes file names is listed: `kept.txt` is found, since only
    /// the repository's own ignore files apply.
    GlobalExcludes,
}

/// The repository `Lists` walks: dependency, environment and build directories at the root and nested, and one source file.
const FILES: [&str; 6] = [
    "node_modules/x.js",
    "pkg/node_modules/v.js",
    ".venv/lib/y.py",
    "target/z.txt",
    "pkg/target/w.txt",
    "src/a.ts",
];

type Input = Scenario;
/// The workspace's directory relative to the repository.
type Value = String;
type Failure = RepositoryError;

/// The name of the repository the global excludes case lists: the argument that names it makes a run of this test binary
/// the listing one.
const LISTED: &str = "global-excludes-repository";

/// A repository whose `kept.txt` only a global excludes file names, listed by a run of this test binary whose home and
/// configuration directory hold that file: the user's real git configuration is never touched, and the listing still
/// holds `kept.txt`. The run is this binary again, given the repository as an argument, so the variables are set on the
/// child process and never in this one. Nothing when it holds `kept.txt`, and what was said otherwise.
fn global_excludes() -> Result<Value, Failure> {
    if let Some(repository) = std::env::args().find(|argument| argument.ends_with(LISTED)) {
        let files = disk()
            .open(&MachinePath(repository.into()))
            .list(&RepoPath::default())?;
        return Ok(if files.iter().any(|path| &*path.0 == "kept.txt") {
            String::new()
        } else {
            "kept.txt is not listed".to_owned()
        });
    }
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let repository = scratch.path().join(LISTED);
    let configuration = scratch.path().join("configuration");
    std::fs::create_dir_all(repository.join(".git")).expect("a repository");
    std::fs::create_dir_all(configuration.join("git")).expect("a configuration directory");
    std::fs::write(repository.join("kept.txt"), "fn a() {}").expect("a file");
    std::fs::write(configuration.join("git/ignore"), "kept.txt\n").expect("a global excludes file");
    std::fs::write(
        configuration.join(".gitconfig"),
        format!(
            "[core]\n\texcludesFile = {}\n",
            configuration.join("git/ignore").display()
        ),
    )
    .expect("a global git configuration");
    let test = module_path!()
        .split_once("::")
        .map_or(module_path!(), |(_, test)| test);
    let listed = std::process::Command::new(std::env::current_exe().expect("this test binary"))
        .args([&format!("{test}::succeeds"), "--exact", "--test-threads=1"])
        .arg(&repository)
        .env("HOME", &configuration)
        .env("XDG_CONFIG_HOME", &configuration)
        .output()
        .expect("this test binary runs");
    Ok(if listed.status.success() {
        String::new()
    } else {
        String::from_utf8_lossy(&listed.stdout).into_owned()
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_only_the_global_git_excludes_name_is_listed"),
        word: Word::Succeeds,
        input: || Scenario::GlobalExcludes,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_directories_the_linked_languages_declare_are_not_listed_at_any_depth"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lists(
                &[
                    ("node_modules", "javascript"),
                    (".venv", "python"),
                    ("target", "rust"),
                ],
                "src/a.ts",
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_root_inside_a_skipped_directory_is_read"),
        word: Word::Succeeds,
        input: || Scenario::Under("node_modules", "node_modules/x.js"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_root_inside_a_nested_skipped_directory_is_read"),
        word: Word::Succeeds,
        input: || Scenario::Under("pkg/node_modules", "pkg/node_modules/v.js"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_directory_named_like_the_skip_of_a_language_not_linked_is_listed"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lists(
                &[("node_modules", "javascript")],
                ".venv/lib/y.py,pkg/target/w.txt,src/a.ts,target/z.txt",
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_workspace_in_a_repository_is_found"),
        word: Word::Succeeds,
        input: || Scenario::Found(true, true),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("no_tastemaker_toml_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Found(true, false),
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::NoWorkspace {start} if std::path::Path::new(start).is_absolute() && std::path::Path::new(start).file_name().is_some_and(|name| name.to_string_lossy().starts_with(".tmp")))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_tastemaker_toml_above_the_repository_is_never_read"),
        word: Word::Refuses,
        input: || Scenario::AboveRepository,
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::NoWorkspace {start} if std::path::Path::new(start).ends_with("repository"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("the_repository_above_a_directory_is_found"),
        word: Word::Succeeds,
        input: || Scenario::Repository(true),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_directory_in_no_repository_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Repository(false),
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::NoRepository {workspace} if std::path::Path::new(workspace).ends_with("below"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("no_repository_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Found(false, true),
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::NoRepository {workspace} if std::path::Path::new(workspace).is_absolute() && std::path::Path::new(workspace).file_name().is_some_and(|name| name.to_string_lossy().starts_with(".tmp")))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_name_that_is_not_utf8_is_refused_never_dropped"),
        word: Word::Refuses,
        input: || Scenario::NotUtf8,
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::Read { path, reason } if path.contains("bad") && reason.contains("UTF-8"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_file_never_written_is_not_read"),
        word: Word::Refuses,
        input: || Scenario::ReadMissing,
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::Read {path,reason} if path == "never.txt" && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_path_whose_metadata_cannot_be_read_is_present_and_its_read_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Looped,
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::Read { path, reason } if path == "tastemaker.lock" && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_file_below_a_file_is_not_written"),
        word: Word::Fails,
        input: || Scenario::WriteBelowFile,
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::Write {path,reason} if path == "a.txt/b.txt" && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// The repository failure carrying `error`'s text.
fn failure(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::Write {
        path: String::new(),
        reason: error.to_string(),
    }
}

/// Writes `bad\xff.txt`, a name that is not UTF-8, into `directory`.
#[cfg(all(unix, not(target_os = "macos")))]
fn not_utf8(directory: &std::path::Path) -> Result<(), RepositoryError> {
    use std::os::unix::ffi::OsStrExt as _;
    std::fs::write(
        directory.join(std::ffi::OsStr::from_bytes(b"bad\xff.txt")),
        "x",
    )
    .map_err(failure)
}

/// A system whose names are UTF-16, or macOS, whose file system takes only UTF-8 names, cannot write a name that is not UTF-8:
/// the walk's refusal is the one it would give.
#[cfg(any(not(unix), target_os = "macos"))]
fn not_utf8(directory: &std::path::Path) -> Result<(), RepositoryError> {
    Err(RepositoryError::Read {
        path: directory.join("bad").display().to_string(),
        reason: "the name is not UTF-8".to_owned(),
    })
}

/// Makes `tastemaker.lock` in `directory` a link to itself, whose metadata cannot be read.
#[cfg(unix)]
fn looped(directory: &std::path::Path) -> Result<(), RepositoryError> {
    std::os::unix::fs::symlink("tastemaker.lock", directory.join("tastemaker.lock"))
        .map_err(failure)
}

/// A system that makes links only with a privilege cannot be relied on to make one: the read's refusal is the one it would give.
#[cfg(not(unix))]
fn looped(_directory: &std::path::Path) -> Result<(), RepositoryError> {
    Err(RepositoryError::Read {
        path: "tastemaker.lock".to_owned(),
        reason: "too many levels of symbolic links".to_owned(),
    })
}

/// Asks whether a lock that is a link to itself is there, then reads it: present, and its read refused.
fn looped_read(directory: &std::path::Path, root: &MachinePath) -> Result<Value, Failure> {
    looped(directory)?;
    let lock = RepoPath("tastemaker.lock".into());
    if disk().open(root).exists(&lock) == crate::outbound::repository::types::Presence::Absent {
        return Ok("a lock whose metadata cannot be read is absent".to_owned());
    }
    let text = disk().open(root).read(&lock)?;
    Ok(text.0.into_string())
}

/// What `adapter` lists beneath `under` once every file of [`FILES`] is written in `directory`, against `expected`.
fn listed(
    directory: &std::path::Path,
    root: &MachinePath,
    adapter: &RepositoryDiskAdapter,
    under: &str,
    expected: &str,
) -> Result<Value, Failure> {
    for file in FILES {
        let path = directory.join(file);
        std::fs::create_dir_all(path.parent().expect("a directory")).map_err(failure)?;
        std::fs::write(path, "x").map_err(failure)?;
    }
    let files = adapter.open(root).list(&RepoPath(under.into()))?;
    let found = files
        .iter()
        .map(|path| &*path.0)
        .collect::<Vec<_>>()
        .join(",");
    Ok(if found == expected {
        String::new()
    } else {
        found
    })
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let directory = tempfile::tempdir().map_err(|error| RepositoryError::Read {
        path: String::new(),
        reason: error.to_string(),
    })?;
    let root = MachinePath(directory.path().display().to_string().into());
    let (git, workspace) = match scenario {
        Scenario::Found(git, workspace) => (git, workspace),
        Scenario::Repository(git) => {
            if git {
                std::fs::create_dir_all(directory.path().join(".git")).map_err(failure)?;
            }
            let below = directory.path().join("below");
            std::fs::create_dir_all(&below).map_err(failure)?;
            let found = disk().repository(&MachinePath(below.display().to_string().into()))?;
            let expected = std::fs::canonicalize(directory.path()).map_err(failure)?;
            // The same place, however each spells it (Windows' canonical form is verbatim, the adapter's is not).
            return Ok(
                if std::fs::canonicalize(&*found.0).ok() == Some(expected.clone()) {
                    String::new()
                } else {
                    format!("found {}", found.0)
                },
            );
        }
        Scenario::AboveRepository => {
            std::fs::write(directory.path().join("tastemaker.toml"), "version = 2\n")
                .map_err(failure)?;
            let repository = directory.path().join("repository");
            std::fs::create_dir_all(repository.join(".git")).map_err(failure)?;
            let found = disk().find(&MachinePath(repository.display().to_string().into()))?;
            return Ok(found.workspace().0.clone().into_string());
        }
        Scenario::NotUtf8 => {
            std::fs::create_dir_all(directory.path().join(".git")).map_err(failure)?;
            not_utf8(directory.path())?;
            let files = disk().open(&root).list(&RepoPath::default())?;
            return Ok(format!("{} files listed", files.len()));
        }
        Scenario::Looped => return looped_read(directory.path(), &root),
        Scenario::ReadMissing => {
            let text = disk().open(&root).read(&RepoPath("never.txt".into()))?;
            return Ok(text.0.into_string());
        }
        Scenario::Lists(declared, expected) => {
            let adapter = RepositoryDiskAdapter {
                skipped: declared
                    .iter()
                    .map(|(name, by)| SkippedDirectory {
                        name: DirectoryName((*name).into()),
                        by: vec![Declarer((*by).into())],
                    })
                    .collect(),
                store: DirectoryName(".tastemaker_cache".into()),
            };
            return listed(directory.path(), &root, &adapter, "", expected);
        }
        Scenario::Under(under, expected) => {
            let adapter = RepositoryDiskAdapter {
                skipped: vec![SkippedDirectory {
                    name: DirectoryName("node_modules".into()),
                    by: vec![Declarer("javascript".into())],
                }],
                store: DirectoryName(".tastemaker_cache".into()),
            };
            return listed(directory.path(), &root, &adapter, under, expected);
        }
        Scenario::GlobalExcludes => return global_excludes(),
        Scenario::WriteBelowFile => {
            let text = FileText("text".into());
            let repository = disk().open(&root);
            repository.write(&RepoPath("a.txt".into()), &text)?;
            repository.write(&RepoPath("a.txt/b.txt".into()), &text)?;
            return Ok(String::new());
        }
    };
    if git {
        std::fs::create_dir_all(directory.path().join(".git")).map_err(failure)?;
    }
    if workspace {
        std::fs::write(directory.path().join("tastemaker.toml"), "version = 2\n")
            .map_err(failure)?;
    }
    let found = disk().find(&root)?;
    Ok(found.workspace().0.clone().into_string())
}

#[test]
fn succeeds() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    std::fs::create_dir_all(directory.path().join(".git")).expect("a .git directory");
    check(
        &disk(),
        &MachinePath(directory.path().display().to_string().into()),
    );
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
