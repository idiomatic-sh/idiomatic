//! Budget, content, grammar identity and current-source occurrence regressions.

use super::fixtures::{
    bounds_reject_large_keys_and_full_memos,
    memo_admission_prefers_source_insertion_order_when_the_memo_is_full,
    ordered_lookup_releases_large_dense_and_hash_storage_on_reset,
    parser_identity_and_exact_content_prevent_stale_reuse,
    the_memo_refuses_at_the_entry_bound_and_releases_oversized_capacity,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `bounds_reject_large_keys_and_full_memos` scenario.
    BoundsRejectLargeKeysAndFullMemos,
    /// The `parser_identity_and_exact_content_prevent_stale_reuse` scenario.
    ParserIdentityAndExactContentPreventStaleReuse,
    /// The `memo_admission_prefers_source_insertion_order_when_the_memo_is_full` scenario.
    MemoAdmissionPrefersSourceInsertionOrderWhenTheMemoIsFull,
    /// The `ordered_lookup_releases_large_dense_and_hash_storage_on_reset` scenario.
    OrderedLookupReleasesLargeDenseAndHashStorageOnReset,
    /// The `the_memo_refuses_at_the_entry_bound_and_releases_oversized_capacity` scenario.
    TheMemoRefusesAtTheEntryBoundAndReleasesOversizedCapacity,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// A scenario is never refused.
type Failure = ();

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("bounds_reject_large_keys_and_full_memos"),
        word: Word::Succeeds,
        input: || Given::BoundsRejectLargeKeysAndFullMemos,
        expect: Expect::Value("Some(Taken { key: MemoKey(\"small\"), value: 7 })|true|true|true"),
    },
    Case {
        name: CaseName("parser_identity_and_exact_content_prevent_stale_reuse"),
        word: Word::Succeeds,
        input: || Given::ParserIdentityAndExactContentPreventStaleReuse,
        expect: Expect::Value(
            "None|None|None|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true|true",
        ),
    },
    Case {
        name: CaseName("memo_admission_prefers_source_insertion_order_when_the_memo_is_full"),
        word: Word::Succeeds,
        input: || Given::MemoAdmissionPrefersSourceInsertionOrderWhenTheMemoIsFull,
        expect: Expect::Value(
            "Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|true|true|true|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|true|true|true|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|true|true|true|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|true|true|true|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|true|true|true|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|true|true|true|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|true|true|true|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|true|true|true|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|true|true|true|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|true|true|true|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|true|true|true|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|true|true|true|true",
        ),
    },
    Case {
        name: CaseName("ordered_lookup_releases_large_dense_and_hash_storage_on_reset"),
        word: Word::Succeeds,
        input: || Given::OrderedLookupReleasesLargeDenseAndHashStorageOnReset,
        expect: Expect::Value(
            "Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|Granted|true|true|0",
        ),
    },
    Case {
        name: CaseName("the_memo_refuses_at_the_entry_bound_and_releases_oversized_capacity"),
        word: Word::Succeeds,
        input: || Given::TheMemoRefusesAtTheEntryBoundAndReleasesOversizedCapacity,
        expect: Expect::Value(
            "EntryCount(64)|EntryCount(64)|true|Some(Taken { key: MemoKey(\"held-0\"), value: 0 })|EntryCount(63)|true|EntryCount(0)|true",
        ),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::BoundsRejectLargeKeysAndFullMemos => bounds_reject_large_keys_and_full_memos(),
        Given::ParserIdentityAndExactContentPreventStaleReuse => {
            parser_identity_and_exact_content_prevent_stale_reuse()
        }
        Given::MemoAdmissionPrefersSourceInsertionOrderWhenTheMemoIsFull => {
            memo_admission_prefers_source_insertion_order_when_the_memo_is_full()
        }
        Given::OrderedLookupReleasesLargeDenseAndHashStorageOnReset => {
            ordered_lookup_releases_large_dense_and_hash_storage_on_reset()
        }
        Given::TheMemoRefusesAtTheEntryBoundAndReleasesOversizedCapacity => {
            the_memo_refuses_at_the_entry_bound_and_releases_oversized_capacity()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
