//! An occurrence's absolute span: its place past the base, refused past what a span's offset holds.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::source_matches::service::absolute_span;
use crate::domain::source_matches::types::{OccurrenceByte, OccurrenceSpan, SourceBase};
use ryo_relations::domain::relations::types::{Offset, Span};
/// An occurrence at `start..end` of the text searched from `base`.
type Input = (usize, usize, usize);
type Value = Span;
type Failure = PortableOperatorFault;

/// The absolute span of an occurrence: the one conversion the text_map stage's search makes of each whole and captured span.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_occurrence_s_span_is_its_place_past_the_base"),
        word: Word::Succeeds,
        input: || (10, 3, 4),
        expect: Expect::Value(Span {
            start: Offset(13),
            end: Offset(14),
        }),
    },
    Case {
        name: CaseName("an_occurrence_ending_at_the_last_offset_a_span_holds_is_placed"),
        word: Word::Succeeds,
        input: || (4_294_967_293, 1, 2),
        expect: Expect::Value(Span {
            start: Offset(u32::MAX - 1),
            end: Offset(u32::MAX),
        }),
    },
    Case {
        name: CaseName("an_occurrence_ending_past_what_a_span_holds_refuses"),
        word: Word::Refuses,
        input: || (4_294_967_294, 1, 2),
        expect: Expect::Error(PortableOperatorFault::InvalidSourceMatchSpan),
    },
    Case {
        name: CaseName("an_occurrence_past_the_platform_s_offsets_refuses"),
        word: Word::Refuses,
        input: || (usize::MAX, 0, 1),
        expect: Expect::Error(PortableOperatorFault::InvalidSourceMatchSpan),
    },
];

fn subject((base, start, end): Input) -> Result<Value, Failure> {
    absolute_span(
        SourceBase(base),
        OccurrenceSpan {
            start: OccurrenceByte(start),
            end: OccurrenceByte(end),
        },
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
