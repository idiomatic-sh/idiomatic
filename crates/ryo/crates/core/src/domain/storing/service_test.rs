// the one service_test.rs a service has holds a row for each decision of the service, and each row its own store; a second test file for one service is a role no spec places
//! The `storing` service's cases, on values alone.

use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::reasons::types::ToolMessage;
use ryo_values::domain::descriptions::types::LanguageName;
use ryo_values::domain::files::types::{CorpusEntry, FileNumber, FileSize, Nanoseconds};
use ryo_values::domain::program::types::ColumnName;
use ryo_values::domain::records::types::Cell;
use ryo_values::domain::records::types::{
    FileOutcome, FindingRows, Handing, PlacedColumn, PlacedLine, PlacedRule, Ran, Record,
    RuleFrontier, Unreadable,
};
use ryo_values::domain::runs::types::{ManifestRefused, RuleId, SkipReason, Skipped};
use ryo_values::domain::stored_facts::types::{Digest, ReadFailure};
use ryo_values::domain::stores::types::{
    BucketsWanted, Crossing, FileAgain, FindingMade, FindingPlace, Stored, StoredFile,
    StoredFileNumber, StoredFinding, StoredFrontier, StoredHanded, StoredPlace, StoredRule,
    StoredUnknown, StoredUnread, StoredValue, UnknownText,
};

use super::service::{
    later_from, made_again, records_from, skipped_from, stored_of, unknowns_from, with_fresh,
};
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
    /// `made_again` for a file as it was, one whose bytes changed but not its interface, one whose interface changed but not its
    /// frontier, and one whose frontier of the first rule changed.
    MadeAgainFrontier,
    /// `made_again` for a file the store has no row of, a manifest, a file a rule reading across files found something in, one a
    /// rule could not decide in, and one whose notes changed; the second rule reads the project.
    MadeAgainHeld,
    /// What the store kept, made into records and the findings across files and kept again: the same tables.
    StoreRoundTrip,
    /// `records_from` and `later_from` with a rule the store kept that the run has not.
    UnknownRule,
    /// `with_fresh` of two kept records and two fresh, one of a path kept.
    WithFresh,
    /// `unknowns_from` with one of the two files it holds unknowns of read again.
    UnknownsFrom,
    /// Explicit skip outcomes are retained only for files not read again.
    SkippedFrom,
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

/// `record` with no frontier.
fn unbordered(mut record: Record) -> Record {
    if let FileOutcome::Read(ran) = &mut record.outcome {
        ran.frontier.clear();
    }
    record
}

/// What the rules `made_again` names come to as one digit: 1 for none, 9 for every one, else 2 and a bit for each rule.
fn again(made: &MadeAgain) -> usize {
    match made {
        MadeAgain::Nothing => 1,
        MadeAgain::All => 9,
        MadeAgain::These(rules) => 2 + rules.iter().map(|rule| 1_usize << rule.0).sum::<usize>(),
    }
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

/// `MadeAgainFrontier`, as one number.
fn frontier_changed() -> Value {
    let (ids, project) = two();
    let plain = plain();
    let made = |record: Record| again(&made_again(&plain, &[record], &[], &project, &ids));
    made(edited("a.rs", 10, 99)) * 1000
        + made(edited("a.rs", 12, 11)) * 100
        + made(edited("a.rs", 12, 12)) * 10
        + made(record("a.rs", 12, 12, 12, 6))
}

/// `MadeAgainHeld`, as one number.
fn held_changed() -> Value {
    let (ids, project) = two();
    let manifest = [FilePath("a.rs".into())];
    let plain = plain();
    let made = |stored: &Stored, record: Record, manifests: &[FilePath]| {
        again(&made_again(stored, &[record], manifests, &project, &ids))
    };
    made(&plain, edited("c.rs", 1, 1), &[]) * 10000
        + made(&plain, edited("a.rs", 12, 99), &manifest) * 1000
        + made(&plain, unbordered(record("b.rs", 23, 99, 22, 5)), &[]) * 100
        + made(&stored(), edited("a.rs", 12, 99), &[]) * 10
        + made(&plain, record("a.rs", 12, 99, 13, 5), &[])
}

/// Round-trip all stored metadata and retain each file's original word number.
fn stored_round_trip() -> Value {
    let mut kept = stored();
    kept.files[0].executable = ryo_values::domain::files::types::ExecutablePermission::PosixNotExecutable;
    kept.files[1].executable = ryo_values::domain::files::types::ExecutablePermission::PosixExecutable;
    let records = records_from(&kept, &ids()).unwrap_or_default();
    let later = later_from(&kept, &ids(), &[]).unwrap_or_default();
    let unknowns = unknowns_from(&kept, &[]);
    let again = stored_of(
        ToKeep {
            rules: kept.rules.clone(),
            records,
            later: &later,
            refused: &kept.refused,
            unknowns: &unknowns,
            skipped: &kept.skipped,
            consulted: &kept.consulted,
            consulted_absent: &kept.consulted_absent,
            project_observations: &kept.project_observations,
        },
        &kept,
        &StoredHanded::default(),
        &BucketsWanted::All,
        Vec::new(),
    );
    // Every file is used again, at its number: its words stay where the store keeps them.
    let numbers: Vec<FileAgain> = (0..kept.files.len())
        .map(|place| {
            let number = StoredFileNumber(u32::try_from(place).unwrap_or(u32::MAX));
            FileAgain {
                was: number,
                now: number,
            }
        })
        .collect();
    usize::from(again.stored == kept && again.kept.file_again == numbers)
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
        Scenario::MadeAgainFrontier => frontier_changed(),
        Scenario::MadeAgainHeld => held_changed(),
        Scenario::StoreRoundTrip => stored_round_trip(),
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
        name: CaseName(
            "only_the_rules_whose_frontier_of_a_changed_interface_changed_are_made_again",
        ),
        word: Word::Succeeds,
        input: || Scenario::MadeAgainFrontier,
        expect: Expect::Value(1113),
    },
    Case {
        name: CaseName(
            "a_new_file_makes_every_rule_again_a_manifest_or_notes_the_project_rules_a_finding_or_unknown_its_rule",
        ),
        word: Word::Succeeds,
        input: || Scenario::MadeAgainHeld,
        expect: Expect::Value(94334),
    },
    Case {
        name: CaseName("what_the_store_kept_used_again_and_kept_again_is_the_same_store"),
        word: Word::Succeeds,
        input: || Scenario::StoreRoundTrip,
        expect: Expect::Value(1),
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
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

#[test]
fn executable_changes_invalidate_early_store_and_equal_digest_routes_in_both_directions() {
    use ryo_values::domain::files::types::ExecutablePermission;
    let posix = |executable: bool| {
        if executable {
            ExecutablePermission::PosixExecutable
        } else {
            ExecutablePermission::PosixNotExecutable
        }
    };
    for (before, after) in [(false, true), (true, false)] {
        let mut kept = stored();
        let mut files = listed();
        kept.files[0].executable = posix(before);
        files[0].executable = posix(before);
        assert_eq!(
            super::service::as_kept(&kept.files[0], &files[0]),
            ryo_values::domain::records::types::Freshness::AsItWas
        );
        assert_eq!(decide(&kept, &rules(true), &files), Decision::Again);
        let mut fresh = record("a.rs", 10, 11, 12, 5);
        fresh.size = files[0].size;
        fresh.modified = files[0].modified;
        fresh.executable = posix(before);
        assert_eq!(
            made_again(&kept, &[fresh.clone()], &[], &[], &[]),
            MadeAgain::Nothing
        );
        files[0].executable = posix(after);
        fresh.executable = posix(after);
        assert_eq!(
            super::service::as_kept(&kept.files[0], &files[0]),
            ryo_values::domain::records::types::Freshness::Changed
        );
        assert_eq!(
            decide(&kept, &rules(true), &files),
            Decision::Check(vec![FileOrder(0)])
        );
        assert_eq!(
            made_again(&kept, &[fresh], &[], &[], &[]),
            MadeAgain::All,
            "equal byte/interface digests cannot hide changed input observations"
        );
    }
}

#[test]
fn unavailable_primary_cannot_reuse_a_cached_cross_file_interface() {
    use ryo_values::domain::files::types::{CorpusInput, InputReadFailure};
    for failed in [
        CorpusInput::Unreadable {
            path: FilePath("a.rs".into()),
            reason: InputReadFailure("missing".into()),
        },
        CorpusInput::ReadFailed {
            entry: entry("a.rs", 1, 5),
            reason: InputReadFailure("read denied".into()),
        },
    ] {
        let inputs = [failed, CorpusInput::Available(entry("b.rs", 2, 6))];
        assert_eq!(
            super::service::decide(&stored(), &rules(true), &inputs),
            Decision::Whole
        );
    }
}
