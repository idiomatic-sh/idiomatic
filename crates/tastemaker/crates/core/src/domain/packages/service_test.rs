//! What the `packages` service answers about releases, versions and the lock.

use std::collections::BTreeMap;

use super::service::{
    kind_of, latest, lock, lock_status, release_file, release_id, strays, unread, version_number,
};
use super::types::KindWord;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::packages::types::{
    DefinitionKind, DefinitionName, Lock, LockStatus, Ownership, PackageManifest, PackageName,
    PackageRoot, Reach, Release, ResolvedPackage, SourcePath, VersionText,
};

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    SharedFile,
    OwnedFile,
    OwnedId,
    Latest,
    NotAVersion,
    UnknownKind,
    LockCurrent,
    LockStale,
    LockMissing,
    Changes,
    DowngradeChanges,
    UnversionedChanges,
    /// A file a package holds outside its manifest, its releases, its nested packages and its versions' directories.
    Strays,
    /// Versions on disk, some pinned by a lock, some the latest of their definition.
    Unread,
}

type Input = Question;
type Value = String;
type Failure = ();

fn release(ownership: Ownership) -> Release {
    Release {
        package: PackageName("idiomatic.arch".into()),
        kind: DefinitionKind::Rule,
        name: DefinitionName("file-length".into()),
        version: VersionText("0.3.0".into()),
        ownership,
    }
}

fn made() -> Lock {
    let mut packages = BTreeMap::new();
    drop(packages.insert(
        PackageName("idiomatic.arch".into()),
        ResolvedPackage {
            directory: PackageRoot("/arch".into()),
            pin: crate::domain::packages::types::PackageSource::Path(SourcePath("../arch".into())),
            manifest: PackageManifest {
                name: PackageName("idiomatic.arch".into()),
                dependencies: Vec::new(),
            },
            reach: Reach::Direct,
        },
    ));
    let used = [
        release(Ownership::Shared),
        release(Ownership::Owned(DefinitionName("hex".into()))),
    ];
    lock(&[PackageName("idiomatic.arch".into())], &packages, &used)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_version_no_lock_reads_and_not_the_latest_is_unread"),
        word: Word::Succeeds,
        input: || Question::Unread,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_outside_every_version_directory_is_a_stray"),
        word: Word::Succeeds,
        input: || Question::Strays,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_changelog_contains_only_crossed_versions"),
        word: Word::Succeeds,
        input: || Question::Changes,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_downgrade_reports_crossed_versions"),
        word: Word::Succeeds,
        input: || Question::DowngradeChanges,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_unversioned_changelog_is_not_a_version_range"),
        word: Word::Succeeds,
        input: || Question::UnversionedChanges,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_shared_release_sits_under_its_kind_name_and_version"),
        word: Word::Succeeds,
        input: || Question::SharedFile,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_owned_release_sits_under_its_spec_version"),
        word: Word::Succeeds,
        input: || Question::OwnedFile,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_owned_release_is_named_by_its_spec"),
        word: Word::Succeeds,
        input: || Question::OwnedId,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_latest_version_is_the_highest_offered"),
        word: Word::Succeeds,
        input: || Question::Latest,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_lock_made_again_is_current"),
        word: Word::Succeeds,
        input: || Question::LockCurrent,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_lock_that_differs_is_stale"),
        word: Word::Succeeds,
        input: || Question::LockStale,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("no_lock_is_missing"),
        word: Word::Succeeds,
        input: || Question::LockMissing,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_version_of_two_numbers_is_refused"),
        word: Word::Refuses,
        input: || Question::NotAVersion,
        expect: Expect::error(()),
    },
    Case {
        name: CaseName("an_unknown_kind_word_is_refused"),
        word: Word::Refuses,
        input: || Question::UnknownKind,
        expect: Expect::error(()),
    },
];

fn check(answer: &str, expected: &str) -> String {
    if answer == expected {
        String::new()
    } else {
        answer.to_owned()
    }
}

/// Which versions on disk no lock reads and are not their definition's latest, against what should be.
fn unread_answer() -> String {
    let on_disk = |kind: DefinitionKind, name: &str, version: &str| Release {
        package: PackageName("idiomatic.arch".into()),
        kind,
        name: DefinitionName(name.into()),
        version: VersionText(version.into()),
        ownership: Ownership::Shared,
    };
    let disk = [
        on_disk(DefinitionKind::Rule, "file-length", "0.1.0"),
        on_disk(DefinitionKind::Rule, "file-length", "0.2.0"),
        on_disk(DefinitionKind::Rule, "file-length", "0.10.0"),
        on_disk(DefinitionKind::Spec, "file-length", "0.2.0"),
        on_disk(DefinitionKind::Spec, "crate", "0.1.0"),
        on_disk(DefinitionKind::Spec, "crate", "0.2.0"),
    ];
    // The lock pins the rule's first version and the spec's: the rule's second is read by no one and is not its
    // latest; a spec of the same name is a definition of its own, and the latest of each is offered.
    let mut pins = made();
    pins.packages[0].releases = vec![
        crate::domain::packages::types::LockedRelease {
            kind: DefinitionKind::Rule,
            name: DefinitionName("file-length".into()),
            version: VersionText("0.1.0".into()),
        },
        crate::domain::packages::types::LockedRelease {
            kind: DefinitionKind::Spec,
            name: DefinitionName("crate".into()),
            version: VersionText("0.1.0".into()),
        },
    ];
    check(
        &unread(&disk, &[pins])
            .iter()
            .map(|release| {
                format!(
                    "{:?} {} {}",
                    release.kind, release.name.0, release.version.0
                )
            })
            .collect::<Vec<_>>()
            .join(","),
        "Rule file-length 0.2.0",
    )
}

/// The changes between two versions a CHANGELOG gives, against what should be: none across a downgrade or from notes
/// with no version headings.
fn changes_answer(question: Question) -> String {
    let text = if matches!(question, Question::UnversionedChanges) {
        "unversioned notes"
    } else {
        "# part\n\n## 0.3.0\nFuture.\n## 0.2.0\nNew.\n## 0.1.0\nOld.\n"
    };
    let (from, to) = if matches!(question, Question::DowngradeChanges) {
        ("0.2.0", "0.1.0")
    } else {
        ("0.1.0", "0.2.0")
    };
    let selected = super::service::changes(
        &super::types::ChangeLog(text.into()),
        &VersionText(from.into()),
        &VersionText(to.into()),
    );
    let expected = if matches!(question, Question::UnversionedChanges) {
        ""
    } else {
        "## 0.2.0\nNew."
    };
    check(&selected.0, expected)
}

fn subject(question: Input) -> Result<Value, Failure> {
    Ok(match question {
        Question::SharedFile => check(
            &release_file(&release(Ownership::Shared)).0,
            "rules/file-length/0.3.0/rule.toml",
        ),
        Question::OwnedFile => check(
            &release_file(&release(Ownership::Owned(DefinitionName("hex".into())))).0,
            "specs/hex/0.3.0/rules/file-length/rule.toml",
        ),
        Question::OwnedId => check(
            &release_id(&release(Ownership::Owned(DefinitionName("hex".into())))).0,
            "idiomatic.arch/hex/file-length",
        ),
        Question::Latest => check(
            &latest(
                &VersionText("0.3.0".into()),
                &[
                    VersionText("0.10.0".into()),
                    VersionText("0.9.1".into()),
                    VersionText("x".into()),
                ],
            )
            .0,
            "0.10.0",
        ),
        Question::NotAVersion => {
            version_number(&VersionText("1.2".into()))
                .map(drop)
                .ok_or(())?;
            String::new()
        }
        Question::UnknownKind => {
            kind_of(KindWord("module")).map(drop).ok_or(())?;
            String::new()
        }
        Question::LockCurrent => {
            let lock = made();
            check(
                &format!("{:?}", lock_status(Some(&lock), &made())),
                &format!("{:?}", LockStatus::Current),
            )
        }
        Question::LockStale => {
            let mut prior = made();
            prior.packages.clear();
            check(
                &format!("{:?}", lock_status(Some(&prior), &made())),
                &format!("{:?}", LockStatus::Stale),
            )
        }
        Question::Changes | Question::DowngradeChanges | Question::UnversionedChanges => {
            changes_answer(question)
        }
        Question::LockMissing => check(
            &format!("{:?}", lock_status(None, &made())),
            &format!("{:?}", LockStatus::Missing),
        ),
        Question::Unread => unread_answer(),
        Question::Strays => check(
            &strays(
                &[
                    "package.toml",
                    "releases.lock",
                    "stray.txt",
                    "rules/a/CHANGELOG.md",
                    "rules/a/0.1.0/rule.toml",
                    "specs/s/0.2.0/rules/r/rule.toml",
                    "templates/t/notes.md",
                    "benchmarks/package.toml",
                    "benchmarks/rules/b/0.1.0/rule.toml",
                ]
                .map(|file| super::types::ReleaseFile(file.to_owned())),
            )
            .iter()
            .map(|file| &*file.0)
            .collect::<Vec<_>>()
            .join(","),
            "releases.lock,stray.txt,rules/a/CHANGELOG.md,templates/t/notes.md",
        ),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
