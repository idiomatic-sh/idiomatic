//! Independent demand, exact raw envelopes and retained per-token parser identities.

use super::fixtures::{
    raw_format_cache_is_lazy_shared_and_reset_independently,
    raw_format_envelopes_borrow_exact_bodies,
    raw_format_is_not_demanded_by_decoded_or_brace_readers,
    raw_format_malformed_segments_are_atomic_and_success_order_is_flattened,
    raw_format_missing_token_text_is_not_an_empty_valid_body,
    raw_format_unavailable_segment_withholds_precise_global_order,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `raw_format_envelopes_borrow_exact_bodies` scenario.
    RawFormatEnvelopesBorrowExactBodies,
    /// The `raw_format_cache_is_lazy_shared_and_reset_independently` scenario.
    RawFormatCacheIsLazySharedAndResetIndependently,
    /// The `raw_format_is_not_demanded_by_decoded_or_brace_readers` scenario.
    RawFormatIsNotDemandedByDecodedOrBraceReaders,
    /// The `raw_format_malformed_segments_are_atomic_and_success_order_is_flattened` scenario.
    RawFormatMalformedSegmentsAreAtomicAndSuccessOrderIsFlattened,
    /// The `raw_format_unavailable_segment_withholds_precise_global_order` scenario.
    RawFormatUnavailableSegmentWithholdsPreciseGlobalOrder,
    /// The `raw_format_missing_token_text_is_not_an_empty_valid_body` scenario.
    RawFormatMissingTokenTextIsNotAnEmptyValidBody,
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
        name: CaseName("raw_format_envelopes_borrow_exact_bodies"),
        word: Word::Succeeds,
        input: || Given::RawFormatEnvelopesBorrowExactBodies,
        expect: Expect::Value(
            "Some(\"\")|Some(\"\")|Some(\"%d\")|Some(\"%s\")|Some(\"%c\")|Some(\"%x\")|Some(\"%d\\r\\n%s\")|Some(\"\\\\x25d\")|Some(\"é\\\\\\n%d\")|None|None|None|None|None|None|None|None",
        ),
    },
    Case {
        name: CaseName("raw_format_cache_is_lazy_shared_and_reset_independently"),
        word: Word::Succeeds,
        input: || Given::RawFormatCacheIsLazySharedAndResetIndependently,
        expect: Expect::Value(
            "Fresh|0|2|2|2|3|true|Some(FormatResultIndex(1))|true|Fresh|Fresh|2|2|Fresh|true|true|4|3",
        ),
    },
    Case {
        name: CaseName("raw_format_is_not_demanded_by_decoded_or_brace_readers"),
        word: Word::Succeeds,
        input: || Given::RawFormatIsNotDemandedByDecodedOrBraceReaders,
        expect: Expect::Value("Fresh|Fresh"),
    },
    Case {
        name: CaseName("raw_format_malformed_segments_are_atomic_and_success_order_is_flattened"),
        word: Word::Succeeds,
        input: || Given::RawFormatMalformedSegmentsAreAtomicAndSuccessOrderIsFlattened,
        expect: Expect::Value("Complete|3|true|1|2|4|true"),
    },
    Case {
        name: CaseName("raw_format_unavailable_segment_withholds_precise_global_order"),
        word: Word::Succeeds,
        input: || Given::RawFormatUnavailableSegmentWithholdsPreciseGlobalOrder,
        expect: Expect::Value("Partial|true|true|true"),
    },
    Case {
        name: CaseName("raw_format_missing_token_text_is_not_an_empty_valid_body"),
        word: Word::Succeeds,
        input: || Given::RawFormatMissingTokenTextIsNotAnEmptyValidBody,
        expect: Expect::Value("Partial|true|true|true"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::RawFormatEnvelopesBorrowExactBodies => raw_format_envelopes_borrow_exact_bodies(),
        Given::RawFormatCacheIsLazySharedAndResetIndependently => {
            raw_format_cache_is_lazy_shared_and_reset_independently()
        }
        Given::RawFormatIsNotDemandedByDecodedOrBraceReaders => {
            raw_format_is_not_demanded_by_decoded_or_brace_readers()
        }
        Given::RawFormatMalformedSegmentsAreAtomicAndSuccessOrderIsFlattened => {
            raw_format_malformed_segments_are_atomic_and_success_order_is_flattened()
        }
        Given::RawFormatUnavailableSegmentWithholdsPreciseGlobalOrder => {
            raw_format_unavailable_segment_withholds_precise_global_order()
        }
        Given::RawFormatMissingTokenTextIsNotAnEmptyValidBody => {
            raw_format_missing_token_text_is_not_an_empty_valid_body()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
