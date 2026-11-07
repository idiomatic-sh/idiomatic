//! What every `packages` adapter must pass, over a package holding `rules/r/0.1.0/rule.toml`.

// Error locations are contractual; diagnostic wording belongs to each implementation.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::{FileText, Stamp};
use crate::domain::packages::types::{
    Dependency, PackageName, PackageRoot, PackageSource, SourcePath, VersionText,
};
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::packages::interface::PackagesPort;
use crate::outbound::packages::types::{HeldFiles, PackageFile, PackagedFile};

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// The rule's file is read.
    Reads,
    /// The rule's file is offered, and nothing of it is kept.
    Offers,
    /// A file the package does not hold is not offered.
    NotOffered,
    /// The rule's versions are listed.
    Versions,
    /// A file the package does not hold is refused.
    Missing,
    /// The files beneath a directory of the package are listed relative to it.
    Files,
    /// A package written by its path is found there, and the lock pins it.
    Locates,
    /// A file the package holds has a stamp, and one it does not has the empty stamp.
    Stamps,
    /// A package the run made, held in memory, is read and listed at the directory it is held at, as any package is.
    Holds,
}

/// What a row asks.
type Input = Question;
/// Whether the answer is the one the contract expects.
type Value = bool;
/// The port's own failure.
type Failure = PackagesError;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_definition_file_is_read"),
        word: Word::Succeeds,
        input: || Question::Reads,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_definition_file_is_offered"),
        word: Word::Succeeds,
        input: || Question::Offers,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_the_package_does_not_hold_is_not_offered"),
        word: Word::Refuses,
        input: || Question::NotOffered,
        expect: Expect::Refusal(|error| {
            if matches!(error, PackagesError::Read {path,reason} if std::path::Path::new(path).ends_with("rules/absent/0.1.0/rule.toml") && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("the_versions_of_a_definition_are_listed"),
        word: Word::Succeeds,
        input: || Question::Versions,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_the_package_does_not_hold_is_refused"),
        word: Word::Refuses,
        input: || Question::Missing,
        expect: Expect::Refusal(|error| {
            if matches!(error, PackagesError::Read {path,reason} if std::path::Path::new(path).ends_with("rules/absent/0.1.0/rule.toml") && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("the_files_beneath_a_directory_are_listed_relative_to_it"),
        word: Word::Succeeds,
        input: || Question::Files,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_package_written_by_its_path_is_found_and_pinned"),
        word: Word::Succeeds,
        input: || Question::Locates,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_held_has_a_stamp_and_one_not_held_the_empty_stamp"),
        word: Word::Succeeds,
        input: || Question::Stamps,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_package_held_in_memory_is_read_and_listed_where_it_is_held"),
        word: Word::Succeeds,
        input: || Question::Holds,
        expect: Expect::Value(true),
    },
];

/// Whether `adapter` answers `question` as the contract expects, over the package at `directory`.
fn answer(
    adapter: &impl PackagesPort,
    directory: &PackageRoot,
    question: Question,
) -> Result<Value, Failure> {
    match question {
        Question::Reads => Ok(!adapter
            .read(directory, &PackageFile("rules/r/0.1.0/rule.toml".into()))?
            .0
            .is_empty()),
        Question::Offers => adapter
            .offers(directory, &PackageFile("rules/r/0.1.0/rule.toml".into()))
            .map(|()| true),
        Question::NotOffered => adapter
            .offers(
                directory,
                &PackageFile("rules/absent/0.1.0/rule.toml".into()),
            )
            .map(|()| true),
        Question::Versions => Ok(adapter
            .versions(directory, &PackageFile("rules/r".into()))?
            .0
            == vec![VersionText("0.1.0".into())]),
        Question::Missing => adapter
            .read(
                directory,
                &PackageFile("rules/absent/0.1.0/rule.toml".into()),
            )
            .map(|_| true),
        Question::Files => Ok(adapter.files(directory, &PackageFile("rules".into()))?.0
            == vec![PackageFile("r/0.1.0/rule.toml".into())]),
        Question::Locates => {
            let located = adapter.locate(
                &Dependency {
                    package: PackageName("p".into()),
                    source: PackageSource::Path(SourcePath(".".into())),
                },
                directory,
                &SourcePath(Box::default()),
            )?;
            Ok(matches!(located.pin, PackageSource::Path(_)))
        }
        Question::Stamps => {
            let packaged = |file: &str| PackagedFile {
                directory: directory.clone(),
                file: PackageFile(file.into()),
            };
            let stamps = adapter.stamps(&[
                packaged("rules/r/0.1.0/rule.toml"),
                packaged("rules/absent/0.1.0/rule.toml"),
            ]);
            Ok(match stamps.0.as_slice() {
                [present, absent] => *present != Stamp::default() && *absent == Stamp::default(),
                _ => false,
            })
        }
        Question::Holds => {
            let rule = PackageFile("rules/h/0.1.0/rule.toml".into());
            let held = adapter.hold(HeldFiles(std::collections::BTreeMap::from([(
                rule.clone(),
                FileText("held".into()),
            )])));
            Ok(held != *directory
                && &*adapter.read(&held, &rule)?.0 == "held"
                && adapter.files(&held, &PackageFile("rules".into()))?.0
                    == vec![PackageFile("h/0.1.0/rule.toml".into())])
        }
    }
}

/// Runs the contract's rows against one implementation, over the package at `directory`.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl PackagesPort, directory: &PackageRoot) {
    run(CASES, Word::Succeeds, |question| {
        answer(adapter, directory, question)
    });
    run(CASES, Word::Refuses, |question| {
        answer(adapter, directory, question)
    });
}
