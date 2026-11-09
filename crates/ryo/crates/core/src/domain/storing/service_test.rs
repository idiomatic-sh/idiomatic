// the one service_test.rs a service has holds a row for each decision of the service, and each row its own store; a second test file for one service is a role no spec places
//! The `storing` service's cases, on values alone.

use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::reasons::types::ToolMessage;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::descriptions::types::LanguageName;
use ryo_values::domain::files::types::{CorpusEntry, FileNumber, FileSize, Nanoseconds};
use ryo_values::domain::records::types::{
    FileOutcome, Handing, Ran, Record, RuleFrontier, Unreadable,
};
use ryo_values::domain::row_texts::types::Cell;
use ryo_values::domain::row_texts::types::{FindingRows, PlacedColumn, PlacedLine, PlacedRule};
use ryo_values::domain::runs::types::{ManifestRefused, RuleId, SkipReason, Skipped};
use ryo_values::domain::stored_facts::types::{Digest, ReadFailure};
use ryo_values::domain::stored_findings::types::{
    FindingMade, FindingPlace, StoredFileNumber, StoredFinding, StoredPlace, StoredUnknown,
    StoredUnread, StoredValue, UnknownText,
};
use ryo_values::domain::stores::types::{
    BucketsWanted, Crossing, FileAgain, Stored, StoredFile, StoredFrontier, StoredHanded,
    StoredRule,
};

use super::service::{made_again, records_from, stored_of};
use crate::domain::reporting::service::{later_from, skipped_from, unknowns_from, with_gone};
use crate::domain::workers::service::with_fresh;

#[cfg(test)]
mod scenarios_test;
use super::types::{Decision, ToKeep};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::workers::types::FileOrder;
use ryo_values::domain::stores::types::MadeAgain;

/// What a row of the cases does; a number packs the answers of the calls it names, first call in the highest digits.
#[derive(Clone, Copy)]
enum Scenario {
    /// `decide` over a store that holds nothing and one kept for another rule, and over one kept for other files, which makes the
    /// cross-file steps again from what the files handed.
    DecideWhole,
    /// `decide` over the files as the store kept them, and over one of them changed while no rule reads across files.
    DecideAgain,
    /// `decide` over one file changed, and over one whose system time is unknown, while a rule reads across files.
    DecideCheck,
    /// `records_from` and `later_from` with a rule the store kept that the run has not.
    UnknownRule,
    /// `with_fresh` of two kept records and two fresh, one of a path kept.
    WithFresh,
    /// `unknowns_from` with one of the two files it holds unknowns of read again.
    UnknownsFrom,
    /// Explicit skip outcomes are retained only for files not read again.
    SkippedFrom,
    /// A file the store kept and the run no longer lists leaves no unknown and no skip of its own behind; a file still listed is
    /// never gone.
    GoneFile,
}

/// What a row of the cases does.
type Input = Scenario;
/// A yes or no as a number, or what a function counts or picks.
type Value = usize;
type Failure = ();

/// The rule of the store, which reads across files when `across` says so.
fn rules(across: bool) -> Vec<StoredRule> {
    vec![StoredRule {
        rule: RuleId("r".into()),
        digest: Digest([1; 16]),
        crossing: if across {
            Crossing::Across
        } else {
            Crossing::Within
        },
    }]
}

/// The file at `path` as the system lists it.
fn entry(path: &str, size: u64, modified: u64) -> CorpusEntry {
    CorpusEntry {
        executable: Default::default(),
        path: FilePath(path.into()),
        size: FileSize(size),
        modified: Nanoseconds(modified),
    }
}

/// Existing store fixtures have observed metadata; preserve their decision assertions through the input sum type.
fn decide(stored: &Stored, rules: &[StoredRule], files: &[CorpusEntry]) -> Decision {
    let inputs: Vec<_> = files
        .iter()
        .cloned()
        .map(ryo_values::domain::files::types::CorpusInput::Available)
        .collect();
    super::service::decide(stored, rules, &inputs)
}

/// The two files of the store, as the system lists them while they are as they were.
fn listed() -> Vec<CorpusEntry> {
    vec![entry("a.rs", 1, 5), entry("b.rs", 2, 6)]
}

/// A finding of the rule `r` with one column, in the file `file` or across files.
fn finding(file: Option<u32>, subject: &str, line: u32) -> StoredFinding {
    StoredFinding {
        rule: RuleId("r".into()),
        made: file.map_or(FindingMade::AcrossFiles, |at| {
            FindingMade::InFile(StoredFileNumber(at))
        }),
        place: if line == 0 {
            FindingPlace::Unplaced
        } else {
            FindingPlace::At(StoredPlace {
                line: PlacedLine(line),
                column: PlacedColumn(3),
            })
        },
        subject: Some(Cell::File(subject.into())).into(),
        key: vec![Some(Cell::Text("k".into())).into()],
        values: vec![StoredValue {
            column: ColumnName("name".into()),
            value: Some(Cell::Text("main".into())).into(),
        }],
    }
}

/// A store of two files: a finding in `a.rs`, `b.rs` read by no language, a finding across files about `b.rs`, a manifest refused,
/// what a rule could not decide in `a.rs` and a configuration above the root the project's lookup read.
fn stored() -> Stored {
    let file = |path: &str, size: u64, modified: u64, tag: u8| StoredFile {
        executable: Default::default(),
        path: FilePath(path.into()),
        digest: Digest([tag; 16]),
        interface: Digest([tag + 1; 16]),
        noted: Digest([tag + 2; 16]),
        size: FileSize(size),
        modified: Nanoseconds(modified),
    };
    Stored {
        rules: rules(true),
        files: vec![file("a.rs", 1, 5, 10), file("b.rs", 2, 6, 20)],
        findings: vec![finding(Some(0), "a.rs", 2), finding(None, "b.rs", 0)],
        unread: vec![StoredUnread {
            file: StoredFileNumber(1),
            reason: Unreadable::Failed(ReadFailure("it failed".into())),
        }],
        refused: vec![ManifestRefused {
            language: LanguageName("rust".into()),
            path: FilePath("Cargo.toml".into()),
            reason: ToolMessage("broken".into()),
        }],
        unknowns: vec![StoredUnknown {
            rule: RuleId("r".into()),
            path: FilePath("a.rs".into()),
            message: UnknownText("partial".into()),
            cause: ryo_values::domain::runs::types::UnknownCause::Undecided,
        }],
        frontiers: vec![StoredFrontier {
            file: StoredFileNumber(0),
            rule: PlacedRule(0),
            digest: Digest([5; 16]),
        }],
        consulted: vec![file("../.cargo/config.toml", 3, 7, 30)],
        ..Stored::default()
    }
}

/// The same store, with nothing the rules could not decide.
fn plain() -> Stored {
    Stored {
        unknowns: Vec::new(),
        skipped: Vec::new(),
        ..stored()
    }
}

/// The ids of the run's rules.
fn ids() -> Vec<RuleId> {
    vec![RuleId("r".into())]
}

/// A record of `path` with the digest, the interface and the notes given, read with no finding; its frontier of the first rule has
/// the digest `frontier`.
fn record(path: &str, digest: u8, interface: u8, noted: u8, frontier: u8) -> Record {
    Record {
        executable: Default::default(),
        path: FilePath(path.into()),
        size: FileSize(9),
        modified: Nanoseconds(9),
        digest: Digest([digest; 16]),
        number: FileNumber(0),
        outcome: FileOutcome::Read(Ran {
            rows: FindingRows::default(),
            number: FileNumber(0),
            interface: Digest([interface; 16]),
            handed: Handing::Kept,
            frontier: vec![RuleFrontier {
                rule: PlacedRule(0),
                digest: Digest([frontier; 16]),
            }],
            noted: Digest([noted; 16]),
        }),
    }
}

/// A record of `path` as `record` makes it, with the notes and frontier as the store kept those of `a.rs`.
fn edited(path: &str, digest: u8, interface: u8) -> Record {
    record(path, digest, interface, 12, 5)
}

/// The ids of two rules, the second of which reads the project, and its place.
fn two() -> (Vec<RuleId>, Vec<PlacedRule>) {
    (
        vec![RuleId("r".into()), RuleId("s".into())],
        vec![PlacedRule(1)],
    )
}

/// 1 when `decision` is `expected`.
fn is(decision: &Decision, expected: &Decision) -> usize {
    usize::from(decision == expected)
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::DecideWhole => {
            let other_rules = vec![StoredRule {
                digest: Digest([7; 16]),
                ..rules(true)[0].clone()
            }];
            let mut more = listed();
            more.push(entry("c.rs", 3, 7));
            is(
                &decide(&Stored::default(), &rules(true), &listed()),
                &Decision::Whole,
            ) * 100
                + is(
                    &decide(&stored(), &other_rules, &listed()),
                    &Decision::Whole,
                ) * 10
                + is(&decide(&stored(), &rules(true), &more), &Decision::Cross)
        }
        Scenario::DecideAgain => {
            let within = Stored {
                rules: rules(false),
                ..stored()
            };
            let edited = vec![entry("a.rs", 1, 8), entry("b.rs", 2, 6)];
            is(
                &decide(&stored(), &rules(true), &listed()),
                &Decision::Again,
            ) * 10
                + is(&decide(&within, &rules(false), &edited), &Decision::Again)
        }
        Scenario::DecideCheck => {
            let edited = vec![entry("a.rs", 1, 8), entry("b.rs", 2, 6)];
            let unknown_time = vec![entry("a.rs", 1, 5), entry("b.rs", 2, 0)];
            is(
                &decide(&stored(), &rules(true), &edited),
                &Decision::Check(vec![FileOrder(0)]),
            ) * 10
                + is(
                    &decide(&stored(), &rules(true), &unknown_time),
                    &Decision::Check(vec![FileOrder(1)]),
                )
        }
        Scenario::UnknownRule => {
            let other = [RuleId("s".into())];
            usize::from(records_from(&stored(), &other).is_none()) * 10
                + usize::from(later_from(&stored(), &other, &[]).is_none())
        }
        Scenario::WithFresh => {
            let merged = with_fresh(
                vec![edited("a.rs", 1, 1), edited("c.rs", 1, 1)],
                vec![edited("b.rs", 2, 2), edited("c.rs", 3, 3)],
            );
            let digests: Vec<u8> = merged.iter().map(|each| each.digest.0[0]).collect();
            let paths: Vec<&str> = merged.iter().map(|each| &*each.path.0).collect();
            usize::from(paths == ["a.rs", "b.rs", "c.rs"]) * 10 + usize::from(digests == [1, 2, 3])
        }
        Scenario::UnknownsFrom => {
            let mut kept = stored();
            kept.unknowns.push(StoredUnknown {
                path: FilePath("b.rs".into()),
                ..kept.unknowns[0].clone()
            });
            let left = unknowns_from(&kept, &[FilePath("a.rs".into())]);
            left.len() * 10 + usize::from(left.first().is_some_and(|each| &*each.path.0 == "b.rs"))
        }
        Scenario::SkippedFrom => {
            let mut kept = stored();
            kept.skipped.push(Skipped {
                rule: RuleId("syntax-only".into()),
                path: FilePath("a.rs".into()),
                reason: SkipReason::SyntaxRejected,
            });
            let reused = skipped_from(&kept, &[]);
            let refreshed = skipped_from(&kept, &[FilePath("a.rs".into())]);
            usize::from(reused == kept.skipped) * 10 + refreshed.len()
        }
        Scenario::GoneFile => {
            let mut kept = stored();
            kept.unknowns.push(StoredUnknown {
                path: FilePath("b.rs".into()),
                ..kept.unknowns[0].clone()
            });
            kept.skipped.push(Skipped {
                rule: RuleId("syntax-only".into()),
                path: FilePath("a.rs".into()),
                reason: SkipReason::SyntaxRejected,
            });
            let (a, b) = (FilePath("a.rs".into()), FilePath("b.rs".into()));
            let none_gone = with_gone(Vec::new(), &kept, &[&a, &b]).is_empty();
            // a.rs is no longer listed: its unknown and its skip go, b.rs keeps its unknown.
            let read_again = with_gone(Vec::new(), &kept, &[&b]);
            let left = unknowns_from(&kept, &read_again);
            usize::from(none_gone) * 1000
                + left.len() * 100
                + usize::from(left.first().is_some_and(|each| each.path == b)) * 10
                + skipped_from(&kept, &read_again).len()
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "everything_is_made_again_when_nothing_is_kept_or_it_was_kept_for_other_rules_and_other_files_cross_again",
        ),
        word: Word::Succeeds,
        input: || Scenario::DecideWhole,
        expect: Expect::Value(111),
    },
    Case {
        name: CaseName(
            "what_was_kept_is_used_again_when_nothing_changed_or_no_rule_reads_across_files",
        ),
        word: Word::Succeeds,
        input: || Scenario::DecideAgain,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName(
            "a_changed_file_or_one_whose_time_is_unknown_is_checked_when_a_rule_reads_across_files",
        ),
        word: Word::Succeeds,
        input: || Scenario::DecideCheck,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("a_store_kept_for_a_rule_the_run_has_not_is_used_for_nothing"),
        word: Word::Succeeds,
        input: || Scenario::UnknownRule,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("fresh_records_take_the_place_of_kept_ones_and_join_them_by_path"),
        word: Word::Succeeds,
        input: || Scenario::WithFresh,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("what_the_rules_could_not_decide_in_a_file_read_again_is_not_kept"),
        word: Word::Succeeds,
        input: || Scenario::UnknownsFrom,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("syntax_rejection_skips_are_replayed_until_their_file_is_read_again"),
        word: Word::Succeeds,
        input: || Scenario::SkippedFrom,
        expect: Expect::Value(10),
    },
    Case {
        name: CaseName("a_file_no_longer_listed_leaves_no_unknown_or_skip_behind"),
        word: Word::Succeeds,
        input: || Scenario::GoneFile,
        expect: Expect::Value(1110),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
