//! The path adapter of the packages port: the contract over a temporary package, then its lock pins.

use super::adapter::PackagesPathAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::packages::types::{
    Dependency, GitSource, PackageName, PackageRoot, PackageSource, SourcePath,
};
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::packages::fixtures::contract::check;
use crate::outbound::packages::interface::PackagesPort as _;
use crate::outbound::packages::types::HeldFiles;

/// What a row asks of a temporary package directory.
#[derive(Clone, Debug)]
enum Scenario {
    /// The source written is located, its file's paths read from the base.
    Locate(PackageSource, &'static str),
    /// A file the package does not hold is read.
    ReadMissing,
    /// The versions under a path that is a file, not a directory, are listed: the listing fails, it is not empty.
    VersionsOfAFile,
    /// More packages are held than the adapter may: the one held longest is let go.
    HeldWithinTheirBound,
    /// More packages are held than the adapter may, by several threads at once: the bound holds when they are done.
    HeldAtOnce,
}

type Input = Scenario;
/// The pin the lock records.
type Value = String;
type Failure = PackagesError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_listing_that_fails_is_refused_never_empty"),
        word: Word::Refuses,
        input: || Scenario::VersionsOfAFile,
        expect: Expect::Refusal(|actual| {
            if matches!(actual, PackagesError::Read { path, .. } if path.ends_with("rules/f")) {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("held_packages_held_at_once_stay_within_their_bound"),
        word: Word::Succeeds,
        input: || Scenario::HeldAtOnce,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("held_packages_stay_within_their_bound"),
        word: Word::Succeeds,
        input: || Scenario::HeldWithinTheirBound,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_is_pinned_as_written_from_the_workspace"),
        word: Word::Succeeds,
        input: || Scenario::Locate(PackageSource::Path(SourcePath("pkg".into())), ""),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_git_source_is_refused_until_the_registry"),
        word: Word::Refuses,
        input: || {
            Scenario::Locate(
                PackageSource::Git(GitSource {
                    url: crate::domain::packages::types::RepositoryUrl(
                        "https://example.invalid/p".into(),
                    ),
                    revision: crate::domain::packages::types::Revision("1".into()),
                }),
                "",
            )
        },
        expect: Expect::Refusal(|actual| {
            if actual
                == &(PackagesError::Unsupported {
                    package: "p".into(),
                    source_kind: "git".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_missing_directory_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Locate(PackageSource::Path(SourcePath("absent".into())), ""),
        expect: Expect::Refusal(|error| {
            if matches!(error, PackagesError::Missing {package,path} if package == "p" && std::path::Path::new(path).ends_with("absent"))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_file_the_package_does_not_hold_is_not_read"),
        word: Word::Refuses,
        input: || Scenario::ReadMissing,
        expect: Expect::Refusal(|error| {
            if matches!(error, PackagesError::Read {path,reason} if path.ends_with("/absent.toml") && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let directory = tempfile::tempdir().map_err(|error| PackagesError::Read {
        path: String::new(),
        reason: error.to_string(),
    })?;
    std::fs::create_dir_all(directory.path().join("pkg")).map_err(|error| PackagesError::Read {
        path: String::new(),
        reason: error.to_string(),
    })?;
    let root = PackageRoot(directory.path().display().to_string().into());
    let (source, base) = match scenario {
        Scenario::HeldWithinTheirBound => {
            use super::constants::HELD_PACKAGES;
            use super::types::HeldPackages;
            let adapter = PackagesPathAdapter::default();
            // Past the numbers whose text sorts before the older ones' (`100000` before `65536`): the one let go is the oldest held.
            let mut last = PackageRoot(String::new().into());
            for _ in 0..=(HELD_PACKAGES * 3) {
                last = adapter.hold(HeldFiles::default());
            }
            let HeldPackages(held) = &adapter.held;
            let kept = held.lock().map_or(0, |held| held.len());
            let last_held = held.lock().is_ok_and(|held| held.contains_key(&last));
            return Ok(if kept == HELD_PACKAGES && last_held {
                String::new()
            } else {
                format!(
                    "HeldPackages holds {kept}, bound {HELD_PACKAGES}, the last held is there: {last_held}"
                )
            });
        }
        Scenario::HeldAtOnce => {
            use super::constants::HELD_PACKAGES;
            use super::types::HeldPackages;
            let adapter = PackagesPathAdapter::default();
            std::thread::scope(|threads| {
                for _ in 0_u8..8_u8 {
                    let _handle = threads.spawn(|| {
                        for _ in 0..HELD_PACKAGES / 2 {
                            drop(adapter.hold(HeldFiles::default()));
                        }
                    });
                }
            });
            let HeldPackages(held) = &adapter.held;
            let kept = held.lock().map_or(0, |held| held.len());
            return Ok(if kept == HELD_PACKAGES {
                String::new()
            } else {
                format!("HeldPackages holds {kept}, bound {HELD_PACKAGES}")
            });
        }
        Scenario::Locate(source, base) => (source, base),
        Scenario::VersionsOfAFile => {
            std::fs::create_dir_all(directory.path().join("rules")).map_err(|error| {
                PackagesError::Read {
                    path: String::new(),
                    reason: error.to_string(),
                }
            })?;
            std::fs::write(directory.path().join("rules/f"), "").map_err(|error| {
                PackagesError::Read {
                    path: String::new(),
                    reason: error.to_string(),
                }
            })?;
            let versions = PackagesPathAdapter::default().versions(
                &root,
                &crate::outbound::packages::types::PackageFile("rules/f".into()),
            )?;
            return Ok(format!("{} versions", versions.0.len()));
        }
        Scenario::ReadMissing => {
            let text = PackagesPathAdapter::default().read(
                &root,
                &crate::outbound::packages::types::PackageFile("absent.toml".into()),
            )?;
            return Ok(text.0.into_string());
        }
    };
    let located = PackagesPathAdapter::default().locate(
        &Dependency {
            package: PackageName("p".into()),
            source,
        },
        &root,
        &SourcePath(base.into()),
    )?;
    Ok(
        if located.pin == PackageSource::Path(SourcePath("pkg".into())) {
            String::new()
        } else {
            format!("{:?}", located.pin)
        },
    )
}

#[test]
fn succeeds() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    std::fs::create_dir_all(directory.path().join("rules/r/0.1.0")).expect("a version directory");
    std::fs::write(
        directory.path().join("rules/r/0.1.0/rule.toml"),
        "version = 2\n",
    )
    .expect("a rule");
    check(
        &PackagesPathAdapter::default(),
        &PackageRoot(directory.path().display().to_string().into()),
    );
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
