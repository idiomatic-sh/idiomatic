//! The ignore adapter of the corpus port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::agree;
use super::fixtures::{inventory, listed, permissions, reading, walking::walking};
use super::types::{BytesRead, Stability, Stamp};
use crate::cases::{Case, CaseName, Expect, Word, run, Matched, selects};
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::fixtures::contract::check;
use crate::outbound::corpus::interface::CorpusPort as _;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::files::service::with_separator;
use ryo_values::domain::files::types::{FileSize, Nanoseconds, NativeSeparator, PlatformPath};

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
    /// Changes the execute bits of a file and reads them back through the stat and the inventory.
    ExecuteBits,
}

/// What a row of the cases asks of the adapter.
type Input = Scenario;
/// A yes or no as a number, or how many bytes were read.
type Value = usize;
type Failure = CorpusError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Inventory => inventory::inventory(),
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
        Scenario::TooLarge => reading::too_large(),
        Scenario::MissingRoot => walking(RootPath(
            "/this/root/is/missing/for/the/disk/adapter".into(),
        ))
        .inventory()
        .map(|entries| entries.0.len()),
        Scenario::Listed => listed::listed(),
        Scenario::GlobalExcludes => global_excludes(),
        Scenario::SkipsDeclared => {
            inventory::skipping(&["node_modules", ".venv", "target"], &["src/a.ts"])
        }
        Scenario::SkipsOfUnlinkedLanguages => inventory::skipping(
            &["node_modules"],
            &[
                ".venv/lib/y.py",
                "pkg/target/w.rs",
                "src/a.ts",
                "target/z.rs",
            ],
        ),
        Scenario::SkipsAPath => {
            inventory::skipping(&["./target"], inventory::WITHOUT_ROOT_TARGET)
        }
        Scenario::GivenOverDisk => reading::given_over_disk(),
        Scenario::MissingFile => reading::missing_file(),
        Scenario::Whole => reading::whole(),
        Scenario::ExecuteBits => permissions::execute_bits(),
    }
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
        name: CaseName("each_posix_execute_bit_is_observed_without_changing_bytes_or_mtime"),
        word: Word::Succeeds,
        input: || Scenario::ExecuteBits,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_root_that_is_not_there_is_refused"),
        word: Word::Refuses,
        input: || Scenario::MissingRoot,
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, CorpusError::Unreadable { .. })))),
    },
    Case {
        name: CaseName("a_file_the_engine_cannot_number_is_refused_before_it_is_read"),
        word: Word::Refuses,
        input: || Scenario::TooLarge,
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, CorpusError::TooLarge { .. })))),
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

