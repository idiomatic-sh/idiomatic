//! The cases of `made_again` over a changed frontier and held findings, the store's round trip, and the executable and
//! availability routes of what the store kept.
use super::*;

/// What a row of these cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// `made_again` for a file as it was, one whose bytes changed but not its interface, one whose interface changed but not its
    /// frontier, and one whose frontier of the first rule changed.
    MadeAgainFrontier,
    /// `made_again` for a file the store has no row of, a manifest, a file a rule reading across files found something in, one a
    /// rule could not decide in, and one whose notes changed; the second rule reads the project.
    MadeAgainHeld,
    /// What the store kept, made into records and the findings across files and kept again: the same tables.
    StoreRoundTrip,
    /// executable changes invalidate early store and equal digest routes in both directions.
    ExecutableChangesInvalidateEarlyStoreAndEqualDigestRoutesInBothDirections,
    /// unavailable primary cannot reuse a cached cross file interface.
    UnavailablePrimaryCannotReuseACachedCrossFileInterface,
}

type Input = Scenario;
type Value = usize;
type Failure = ();

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::MadeAgainFrontier => frontier_changed(),
        Scenario::MadeAgainHeld => held_changed(),
        Scenario::StoreRoundTrip => stored_round_trip(),
        Scenario::ExecutableChangesInvalidateEarlyStoreAndEqualDigestRoutesInBothDirections => executable_changes_invalidate_early_store_and_equal_digest_routes_in_both_directions(),
        Scenario::UnavailablePrimaryCannotReuseACachedCrossFileInterface => unavailable_primary_cannot_reuse_a_cached_cross_file_interface(),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
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
        name: CaseName("executable_changes_invalidate_early_store_and_equal_digest_routes_in_both_directions"),
        word: Word::Succeeds,
        input: || Scenario::ExecutableChangesInvalidateEarlyStoreAndEqualDigestRoutesInBothDirections,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("unavailable_primary_cannot_reuse_a_cached_cross_file_interface"),
        word: Word::Succeeds,
        input: || Scenario::UnavailablePrimaryCannotReuseACachedCrossFileInterface,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
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
    kept.files[0].executable =
        ryo_values::domain::files::types::ExecutablePermission::PosixNotExecutable;
    kept.files[1].executable =
        ryo_values::domain::files::types::ExecutablePermission::PosixExecutable;
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

fn executable_changes_invalidate_early_store_and_equal_digest_routes_in_both_directions() -> Value {
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
    1
}

fn unavailable_primary_cannot_reuse_a_cached_cross_file_interface() -> Value {
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
    1
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
