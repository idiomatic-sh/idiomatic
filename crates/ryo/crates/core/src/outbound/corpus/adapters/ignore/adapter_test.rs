// ignore: idiomatic.arch/file-length - the one test file the adapter template places for the ignore adapter: its walk, stamp, listing and skip scenarios share one table and one subject, and a split would be a file no adapter template places
//! The ignore adapter of the corpus port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::{CorpusIgnoreAdapter, agree};
use super::types::{BytesRead, Stability, Stamp};
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::fixtures::contract::check;
use crate::outbound::corpus::interface::CorpusPort as _;
use crate::outbound::corpus::types::SkippedDirectories;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::files::service::with_separator;
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusFile, CorpusListing, FileBytes, FileSize, Nanoseconds, NativeSeparator,
    PlatformPath,
};

/// The adapter that walks the root and is given nothing.
fn walking(root: RootPath) -> CorpusIgnoreAdapter {
    CorpusIgnoreAdapter::new(
        root,
        CorpusListing::Walked,
        Vec::new(),
        SkippedDirectories::default(),
    )
}

/// What the system said of a file of 9 bytes.
const STAMP: Stamp = Stamp {
    size: FileSize(9),
    written: Nanoseconds(10),
    changed: Nanoseconds(20),
};

/// The same file, written a moment later.
const WRITTEN_LATER: Stamp = Stamp {
    written: Nanoseconds(11),
    ..STAMP
};

/// The same file, its write time put back but its inode changed.
const CHANGED_LATER: Stamp = Stamp {
    changed: Nanoseconds(21),
    ..STAMP
};

/// What a row of the cases asks of the adapter.
#[derive(Clone, Copy)]
enum Scenario {
    /// Lists a directory with files, a subdirectory and a `.git` directory.
    Inventory,
    /// Writes paths as the engine does, on a platform that splits them with a backslash and on one that does not.
    Separator,
    /// Whether a read that saw the first stamp before and the second after, and brought in this many bytes, is stable.
    Stable(Stamp, Stamp, u64),
    /// Reads a sparse file of one byte more than the engine can number.
    TooLarge,
    /// Reads a file of nine bytes.
    Whole,
    /// Reads a file that is not on disk: the refusal names its path once.
    MissingFile,
    /// Lists a root that is not there.
    MissingRoot,
    /// Lists the two files the caller named, one of them not on disk, beside a file given as text.
    Listed,
    /// Reads a file given as text where the disk has a file of the same path.
    GivenOverDisk,
    /// Walks a repository with no `.gitignore` that holds the dependency, environment and build directories of three languages, at
    /// the root and nested, with the directories of the three declared as skipped.
    SkipsDeclared,
    /// The same repository, with only the directory of one language declared: the directories of the languages not linked are read.
    SkipsOfUnlinkedLanguages,
    /// Skips `./target`: the directory at that path below the root, not the ones of that name deeper.
    SkipsAPath,
    /// Walks a repository whose file `kept.rs` only the user's global git excludes file names: it is listed, since only the
    /// repository's own ignore files apply.
    GlobalExcludes,
}

/// What a row of the cases asks of the adapter.
type Input = Scenario;
/// A yes or no as a number, or how many bytes were read.
type Value = usize;
type Failure = CorpusError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Inventory => inventory(),
        Scenario::Separator => {
            let written = |text: &'static str, native: char| {
                with_separator(PlatformPath(text), NativeSeparator(native))
            };
            Ok(usize::from(
                written("src\\inbound\\lib.rs", '\\') == FilePath("src/inbound/lib.rs".into())
                    && written("src/inbound/lib.rs", '/') == FilePath("src/inbound/lib.rs".into())
                    && written("src/odd\\name.rs", '/') == FilePath("src/odd\\name.rs".into()),
            ))
        }
        Scenario::Stable(before, after, read) => Ok(usize::from(
            agree(&before, &after, BytesRead(read)) == Stability::Stable,
        )),
        Scenario::TooLarge => {
            let root = tempfile::tempdir().expect("a temporary directory");
            let file = std::fs::File::create(root.path().join("big.bin")).expect("a file");
            file.set_len(u64::from(u32::MAX) + 1)
                .expect("a sparse file");
            let entry = CorpusEntry {
                executable: Default::default(),
                path: FilePath("big.bin".into()),
                size: FileSize(u64::from(u32::MAX) + 1),
                modified: Nanoseconds(0),
            };
            walking(RootPath(root.path().to_str().expect("a utf-8 path").into()))
                .read(&entry, &mut FileBytes::default())
                .map(|()| 0)
        }
        Scenario::MissingRoot => walking(RootPath(
            "/this/root/is/missing/for/the/disk/adapter".into(),
        ))
        .inventory()
        .map(|entries| entries.0.len()),
        Scenario::Listed => listed(),
        Scenario::GlobalExcludes => global_excludes(),
        Scenario::SkipsDeclared => skipping(&["node_modules", ".venv", "target"], &["src/a.ts"]),
        Scenario::SkipsOfUnlinkedLanguages => skipping(
            &["node_modules"],
            &[
                ".venv/lib/y.py",
                "pkg/target/w.rs",
                "src/a.ts",
                "target/z.rs",
            ],
        ),
        Scenario::SkipsAPath => skipping(&["./target"], WITHOUT_ROOT_TARGET),
        Scenario::GivenOverDisk => {
            let root = tempfile::tempdir().expect("a temporary directory");
            std::fs::write(root.path().join("a.rs"), "fn a() {}").expect("a file");
            let adapter = CorpusIgnoreAdapter::new(
                RootPath(root.path().to_str().expect("a utf-8 path").into()),
                CorpusListing::Walked,
                vec![CorpusFile {
                    executable: Default::default(),
                    path: FilePath("a.rs".into()),
                    bytes: FileBytes(b"fn given() {}".to_vec()),
                }],
                SkippedDirectories::default(),
            );
            let entries = adapter.inventory()?.0;
            let mut bytes = FileBytes::default();
            adapter.read(known_entry(&entries[0]), &mut bytes)?;
            Ok(usize::from(
                entries.len() == 1 && bytes.0 == b"fn given() {}",
            ))
        }
        Scenario::MissingFile => {
            let root = tempfile::tempdir().expect("a temporary directory");
            let entry = CorpusEntry {
                executable: Default::default(),
                path: FilePath("gone.rs".into()),
                size: FileSize(9),
                modified: Nanoseconds(0),
            };
            let adapter = walking(RootPath(root.path().to_str().expect("a utf-8 path").into()));
            match adapter.read(&entry, &mut FileBytes::default()) {
                Err(error @ CorpusError::Unreadable { .. }) => {
                    Ok(error.to_string().matches("gone.rs").count())
                }
                other => other.map(|()| 0),
            }
        }
        Scenario::Whole => {
            let root = tempfile::tempdir().expect("a temporary directory");
            std::fs::write(root.path().join("a.rs"), "fn a() {}").expect("a file");
            let entry = CorpusEntry {
                executable: Default::default(),
                path: FilePath("a.rs".into()),
                size: FileSize(9),
                modified: Nanoseconds(0),
            };
            let mut bytes = FileBytes::default();
            walking(RootPath(root.path().to_str().expect("a utf-8 path").into()))
                .read(&entry, &mut bytes)?;
            Ok(bytes.0.len())
        }
    }
}

/// The walk lists the files under the root, sized and dated, and leaves out the repository's own directory.
fn inventory() -> Result<Value, Failure> {
    let root = tempfile::tempdir().expect("a temporary directory");
    std::fs::create_dir_all(root.path().join("sub")).expect("a subdirectory");
    std::fs::create_dir_all(root.path().join(".git")).expect("a git directory");
    std::fs::write(root.path().join("b.rs"), "fn b() {}").expect("a file");
    std::fs::write(root.path().join("a.rs"), "fn a() {}").expect("a file");
    std::fs::write(root.path().join("sub/c.txt"), "c").expect("a file");
    std::fs::write(root.path().join(".git/config"), "x").expect("a file");
    let mut entries = walking(RootPath(root.path().to_str().expect("a utf-8 path").into()))
        .inventory()?
        .0;
    entries.sort_by(|left, right| {
        ryo_values::domain::files::service::input_path(left)
            .cmp(ryo_values::domain::files::service::input_path(right))
    });
    let found: Vec<(&str, u64)> = entries
        .iter()
        .map(|entry| (&*known_entry(entry).path.0, known_entry(entry).size.0))
        .collect();
    // A file the system dates carries the date, so an edit shows without a read.
    let dated = entries
        .iter()
        .all(|entry| known_entry(entry).modified.0 > 0);
    Ok(usize::from(
        found == [("a.rs", 9), ("b.rs", 9), ("sub/c.txt", 1)] && dated,
    ))
}

/// What the walk lists when only `./target` is skipped.
const WITHOUT_ROOT_TARGET: &[&str] = &[
    ".venv/lib/y.py",
    "node_modules/x.js",
    "pkg/node_modules/v.js",
    "pkg/target/w.rs",
    "src/a.ts",
];

/// A walk of a repository with no `.gitignore`, holding `node_modules/x.js`, `pkg/node_modules/v.js`, `.venv/lib/y.py`,
/// `target/z.rs`, `pkg/target/w.rs` and `src/a.ts`, lists the files `expected` names when the directories named `skipped` are
/// skipped.
fn skipping(skipped: &[&str], expected: &[&str]) -> Result<Value, Failure> {
    let root = tempfile::tempdir().expect("a temporary directory");
    for file in [
        "node_modules/x.js",
        "pkg/node_modules/v.js",
        ".venv/lib/y.py",
        "target/z.rs",
        "pkg/target/w.rs",
        "src/a.ts",
    ] {
        let path = root.path().join(file);
        std::fs::create_dir_all(path.parent().expect("a directory")).expect("directories");
        std::fs::write(path, "x").expect("a file");
    }
    let adapter = CorpusIgnoreAdapter::new(
        RootPath(root.path().to_str().expect("a utf-8 path").into()),
        CorpusListing::Walked,
        Vec::new(),
        SkippedDirectories(skipped.iter().map(|name| (*name).into()).collect()),
    );
    let mut entries = adapter.inventory()?.0;
    entries.sort_by(|left, right| {
        ryo_values::domain::files::service::input_path(left)
            .cmp(ryo_values::domain::files::service::input_path(right))
    });
    let found: Vec<&str> = entries
        .iter()
        .map(|entry| &*ryo_values::domain::files::service::input_path(entry).0)
        .collect();
    Ok(usize::from(found == expected))
}

/// The name of the repository the global excludes case walks: the argument that names it makes a run of this test binary
/// the walking one.
const WALKED: &str = "global-excludes-repository";

/// A repository whose `kept.rs` only a global excludes file names, walked by a run of this test binary whose home and
/// configuration directory hold that file: the user's real git configuration is never touched, and the walk still lists
/// `kept.rs`. The run is this binary again, given the repository as an argument, so the variables are set on the child
/// process and never in this one.
fn global_excludes() -> Result<Value, Failure> {
    if let Some(repository) = std::env::args().find(|argument| argument.ends_with(WALKED)) {
        let entries = walking(RootPath(repository.into())).inventory()?.0;
        return Ok(usize::from(entries.iter().any(|entry| {
            &*ryo_values::domain::files::service::input_path(entry).0 == "kept.rs"
        })));
    }
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let repository = scratch.path().join(WALKED);
    let configuration = scratch.path().join("configuration");
    std::fs::create_dir_all(repository.join(".git")).expect("a repository");
    std::fs::create_dir_all(configuration.join("git")).expect("a configuration directory");
    std::fs::write(repository.join("kept.rs"), "fn a() {}").expect("a file");
    std::fs::write(configuration.join("git/ignore"), "kept.rs\n").expect("a global excludes file");
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
    let walked = std::process::Command::new(std::env::current_exe().expect("this test binary"))
        .args([&format!("{test}::succeeds"), "--exact", "--test-threads=1"])
        .arg(&repository)
        .env("HOME", &configuration)
        .env("XDG_CONFIG_HOME", &configuration)
        .output()
        .expect("this test binary runs");
    Ok(usize::from(walked.status.success()))
}

/// A listing lists only the named files that are there, and the given ones.
fn listed() -> Result<Value, Failure> {
    let root = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(root.path().join("a.rs"), "fn a() {}").expect("a file");
    std::fs::write(root.path().join("unlisted.rs"), "fn u() {}").expect("a file");
    let adapter = CorpusIgnoreAdapter::new(
        RootPath(root.path().to_str().expect("a utf-8 path").into()),
        CorpusListing::Listed(vec![FilePath("a.rs".into()), FilePath("gone.rs".into())]),
        vec![CorpusFile {
            executable: Default::default(),
            path: FilePath("given/b.rs".into()),
            bytes: FileBytes(b"fn b() {}".to_vec()),
        }],
        SkippedDirectories::default(),
    );
    let mut entries = adapter.inventory()?.0;
    entries.sort_by(|left, right| {
        ryo_values::domain::files::service::input_path(left)
            .cmp(ryo_values::domain::files::service::input_path(right))
    });
    let found: Vec<(&str, u64)> = entries
        .iter()
        .filter_map(|input| match input {
            ryo_values::domain::files::types::CorpusInput::Available(entry) => {
                Some((&*entry.path.0, entry.size.0))
            }
            _ => None,
        })
        .collect();
    let missing = entries
        .iter()
        .filter(|input| {
            matches!(input,
        ryo_values::domain::files::types::CorpusInput::Unreadable { path, reason }
            if path.0.as_ref() == "gone.rs" && !reason.0.is_empty())
        })
        .count();
    Ok(usize::from(
        found == [("a.rs", 9), ("given/b.rs", 9)] && missing == 1 && entries.len() == 3,
    ))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_directory_a_language_declares_skipped_is_not_read_at_any_depth"),
        word: Word::Succeeds,
        input: || Scenario::SkipsDeclared,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_directory_named_like_the_skip_of_a_language_not_linked_is_read"),
        word: Word::Succeeds,
        input: || Scenario::SkipsOfUnlinkedLanguages,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_skip_written_as_a_path_skips_only_the_directory_at_that_path"),
        word: Word::Succeeds,
        input: || Scenario::SkipsAPath,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_listing_lists_only_the_named_files_that_are_there_and_the_given_ones"),
        word: Word::Succeeds,
        input: || Scenario::Listed,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_file_given_as_text_is_read_in_place_of_the_disks"),
        word: Word::Succeeds,
        input: || Scenario::GivenOverDisk,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_file_only_the_global_git_excludes_name_is_listed"),
        word: Word::Succeeds,
        input: || Scenario::GlobalExcludes,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("files_are_listed_with_their_sizes_and_dates_and_git_is_skipped"),
        word: Word::Succeeds,
        input: || Scenario::Inventory,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_windows_path_is_written_with_the_engines_separator"),
        word: Word::Succeeds,
        input: || Scenario::Separator,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName(
            "a_read_is_stable_when_the_file_says_the_same_after_and_the_bytes_are_its_size",
        ),
        word: Word::Succeeds,
        input: || Scenario::Stable(STAMP, STAMP, 9),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_read_is_not_stable_when_the_file_was_written_during_it"),
        word: Word::Succeeds,
        input: || Scenario::Stable(STAMP, WRITTEN_LATER, 9),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName(
            "a_read_is_not_stable_when_the_inode_changed_though_the_write_time_was_restored",
        ),
        word: Word::Succeeds,
        input: || Scenario::Stable(STAMP, CHANGED_LATER, 9),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_read_is_not_stable_when_the_bytes_are_not_the_size_it_said"),
        word: Word::Succeeds,
        input: || Scenario::Stable(STAMP, STAMP, 5),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_file_that_is_there_is_read_whole"),
        word: Word::Succeeds,
        input: || Scenario::Whole,
        expect: Expect::Value(9),
    },
    Case {
        name: CaseName("a_file_that_is_not_there_is_refused_naming_its_path_once"),
        word: Word::Succeeds,
        input: || Scenario::MissingFile,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_root_that_is_not_there_is_refused"),
        word: Word::Refuses,
        input: || Scenario::MissingRoot,
        expect: Expect::Refusal(|error| selects(matches!(error, CorpusError::Unreadable { .. }))),
    },
    Case {
        name: CaseName("a_file_the_engine_cannot_number_is_refused_before_it_is_read"),
        word: Word::Refuses,
        input: || Scenario::TooLarge,
        expect: Expect::Refusal(|error| selects(matches!(error, CorpusError::TooLarge { .. }))),
    },
];

#[test]
fn succeeds() {
    let root = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(root.path().join("a.rs"), "fn a() {}").expect("a file");
    check(&walking(RootPath(
        root.path().to_str().expect("a utf-8 path").into(),
    )));
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[cfg(unix)]
#[test]
fn inventory_and_stat_observe_each_posix_execute_bit_without_changing_bytes_or_mtime() {
    use ryo_values::domain::files::types::ExecutablePermission;
    use std::os::unix::fs::PermissionsExt as _;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("case.py");
    std::fs::write(&path, b"print(1)\n").unwrap();
    let adapter = walking(RootPath(root.path().to_str().unwrap().into()));
    let file = FilePath("case.py".into());
    let crate::outbound::corpus::types::PathStat::Found(original) = adapter.stat(&file).unwrap()
    else {
        panic!("file is present");
    };
    for (mode, expected) in [
        (0o644, false),
        (0o744, true),
        (0o654, true),
        (0o645, true),
        (0o644, false),
    ] {
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        let crate::outbound::corpus::types::PathStat::Found(observed) =
            adapter.stat(&file).unwrap()
        else {
            panic!("file remains present");
        };
        assert_eq!(observed.executable, ExecutablePermission::Posix(expected));
        assert_eq!(observed.size, original.size);
        assert_eq!(observed.modified, original.modified);
        assert_eq!(
            known_entry(&adapter.inventory().unwrap().0[0]).executable,
            observed.executable
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"print(1)\n");
    }
}

/// Tests that require physical metadata explicitly reject unavailable input fixtures.
fn known_entry(
    input: &ryo_values::domain::files::types::CorpusInput,
) -> &ryo_values::domain::files::types::CorpusEntry {
    let ryo_values::domain::files::types::CorpusInput::Available(entry) = input else {
        panic!("this fixture requires observed file metadata");
    };
    entry
}
