//! Public inbound acceptance: actual semantic dispatch, borrowed outputs, faults and delivery order.
use super::{interface::*, use_case::OperatorsUseCaseImpl};
use ryo_relations::domain::relations::types::{CellKind, Integer, Offset, Span, Value as Cell};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::borrow::Cow;

mod groups;
mod ordered;
#[cfg(test)]
mod ordered_existence;
#[cfg(test)]
mod positions;
mod rows;
mod text;

#[derive(Clone, Copy)]
enum Scenario {
    Project,
    Append,
    MissingProjectColumn,
    SelectRepeated,
    SelectRejected,
    SelectOriginal,
    SelectMissing,
    Words,
    WordsEmpty,
    CaptureUtf8,
    CaptureSlash,
    RankFirst,
    RankAbsent,
    ReplaceLiteral,
    CountOutput,
    DigestOutput,
    AggregateMissing,
    OrderedPairs,
    OrderedMissing,
    OrderedInterrupted,
    SourceLines,
    SourceMissingCapture,
    SourceMissingOptions,
    SourceInterrupted,
    SourceOrdinary,
    SourceOrdinaryInterrupted,
    SourceNoLine,
    SourceSpanOnly,
    CountGroups,
    DistinctGroups,
    CountOperandFirst,
    DigestGroups,
    HashValue,
    CheckCount,
    CheckDigest,
    CheckStreamCountOrder,
    CheckCollectedCountOrder,
    CheckStreamDigestOrder,
    StreamCount,
    StreamDigest,
    StreamEarlyFault,
    StreamStorageFailure,
    FinishDigest,
}
type Input = Scenario;
#[derive(Debug, PartialEq)]
enum Observation {
    Cells(Cow<'static, [Cell]>),
    Selected(bool, Cow<'static, [Cell]>),
    Text(Cow<'static, str>),
    Rank(Option<i64>),
    Groups(Cow<'static, [(usize, usize)]>),
    Kinds(Cow<'static, [CellKind]>),
    Hashes {
        same_text: bool,
        different_numbers: bool,
    },
}
type Value = Observation;
type Failure = PortableOperatorFault;
const ROW: [Cell; 2] = [Cell::Number(Integer(7)), Cell::Number(Integer(9))];
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("project_preserves_repeated_plan_column"),
        word: Word::Succeeds,
        input: || Scenario::Project,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[ROW[1], ROW[0], ROW[1]]))),
    },
    Case {
        name: CaseName("append_preserves_row_and_suffix_order"),
        word: Word::Succeeds,
        input: || Scenario::Append,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[ROW[0], ROW[1], ROW[0]]))),
    },
    Case {
        name: CaseName("project_reports_absent_column"),
        word: Word::Refuses,
        input: || Scenario::MissingProjectColumn,
        expect: Expect::Error(Failure::NoSuchColumn { index: 2 }),
    },
    Case {
        name: CaseName("select_dispatch_preserves_repeated_kept_columns"),
        word: Word::Succeeds,
        input: || Scenario::SelectRepeated,
        expect: Expect::Value(Observation::Selected(
            true,
            Cow::Borrowed(&[ROW[1], ROW[0], ROW[1]]),
        )),
    },
    Case {
        name: CaseName("select_refusal_never_reads_invalid_kept_columns"),
        word: Word::Succeeds,
        input: || Scenario::SelectRejected,
        expect: Expect::Value(Observation::Selected(false, Cow::Borrowed(&[ROW[0]]))),
    },
    Case {
        name: CaseName("select_empty_kept_preserves_original_without_copying"),
        word: Word::Succeeds,
        input: || Scenario::SelectOriginal,
        expect: Expect::Value(Observation::Selected(true, Cow::Borrowed(&[ROW[0]]))),
    },
    Case {
        name: CaseName("select_reports_first_missing_kept_column"),
        word: Word::Refuses,
        input: || Scenario::SelectMissing,
        expect: Expect::Error(Failure::NoSuchColumn { index: 9 }),
    },
    Case {
        name: CaseName("words_dispatch_keeps_unicode_case_and_digit_boundaries"),
        word: Word::Succeeds,
        input: || Scenario::Words,
        expect: Expect::Value(Observation::Text(Cow::Borrowed("httpserver2|go|über|name"))),
    },
    Case {
        name: CaseName("words_separators_produce_no_empty_words"),
        word: Word::Succeeds,
        input: || Scenario::WordsEmpty,
        expect: Expect::Value(Observation::Text(Cow::Borrowed(""))),
    },
    Case {
        name: CaseName("capture_dispatch_keeps_utf8_boundaries_and_match_order"),
        word: Word::Succeeds,
        input: || Scenario::CaptureUtf8,
        expect: Expect::Value(Observation::Text(Cow::Borrowed("é,ab|éa,b"))),
    },
    Case {
        name: CaseName("capture_ordinary_slots_refuse_crossing_a_slash"),
        word: Word::Succeeds,
        input: || Scenario::CaptureSlash,
        expect: Expect::Value(Observation::Text(Cow::Borrowed(""))),
    },
    Case {
        name: CaseName("rank_dispatch_chooses_first_matching_layer"),
        word: Word::Succeeds,
        input: || Scenario::RankFirst,
        expect: Expect::Value(Observation::Rank(Some(0))),
    },
    Case {
        name: CaseName("rank_unmatched_text_has_no_output"),
        word: Word::Succeeds,
        input: || Scenario::RankAbsent,
        expect: Expect::Value(Observation::Rank(None)),
    },
    Case {
        name: CaseName("replacement_dispatch_treats_dollar_text_literally"),
        word: Word::Succeeds,
        input: || Scenario::ReplaceLiteral,
        expect: Expect::Value(Observation::Text(Cow::Borrowed("$1$1é"))),
    },
    Case {
        name: CaseName("aggregate_count_dispatch_preserves_key_order"),
        word: Word::Succeeds,
        input: || Scenario::CountOutput,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            ROW[1],
            ROW[0],
            ROW[1],
            Cell::Number(Integer(4)),
        ]))),
    },
    Case {
        name: CaseName("aggregate_digest_precedes_count"),
        word: Word::Succeeds,
        input: || Scenario::DigestOutput,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            Cell::Number(Integer(-5)),
            Cell::Number(Integer(3)),
        ]))),
    },
    Case {
        name: CaseName("aggregate_reports_first_missing_key"),
        word: Word::Refuses,
        input: || Scenario::AggregateMissing,
        expect: Expect::Error(Failure::NoSuchColumn { index: 4 }),
    },
    Case {
        name: CaseName("ordered_dispatch_preserves_fifo_witnesses_and_repeated_columns"),
        word: Word::Succeeds,
        input: || Scenario::OrderedPairs,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            Cell::Number(Integer(1)),
            Cell::Number(Integer(0)),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(1)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(3)),
            Cell::Number(Integer(2)),
            Cell::Number(Integer(3)),
        ]))),
    },
    Case {
        name: CaseName("ordered_projection_fault_precedes_any_delivery"),
        word: Word::Refuses,
        input: || Scenario::OrderedMissing,
        expect: Expect::Error(Failure::NoSuchColumn { index: 2 }),
    },
    Case {
        name: CaseName("ordered_sink_failure_preserves_prefix_and_stops_reads"),
        word: Word::Fails,
        input: || Scenario::OrderedInterrupted,
        expect: Expect::Error(Failure::Interrupted),
    },
    Case {
        name: CaseName("source_dispatch_preserves_crlf_and_capture_line_numbers"),
        word: Word::Succeeds,
        input: || Scenario::SourceLines,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            Cell::Number(Integer(2)),
            Cell::Number(Integer(3)),
        ]))),
    },
    Case {
        name: CaseName("source_unknown_capture_fails_even_when_regex_matches_nothing"),
        word: Word::Refuses,
        input: || Scenario::SourceMissingCapture,
        expect: Expect::Error(Failure::MissingCapture),
    },
    Case {
        name: CaseName("source_capture_outputs_require_named_capture"),
        word: Word::Refuses,
        input: || Scenario::SourceMissingOptions,
        expect: Expect::Error(Failure::WrongOperation),
    },
    Case {
        name: CaseName("source_second_sink_failure_stops_later_interns_and_delivery"),
        word: Word::Fails,
        input: || Scenario::SourceInterrupted,
        expect: Expect::Error(Failure::Interrupted),
    },
    Case {
        name: CaseName("source_ordinary_matches_keep_exact_rows_without_capture_outputs"),
        word: Word::Succeeds,
        input: || Scenario::SourceOrdinary,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            Cell::Number(Integer(7)),
            Cell::Span(Span {
                start: Offset(3),
                end: Offset(4),
            }),
            Cell::Number(Integer(7)),
            Cell::Span(Span {
                start: Offset(5),
                end: Offset(6),
            }),
        ]))),
    },
    Case {
        name: CaseName("source_ordinary_second_sink_refusal_keeps_prefix_and_stops"),
        word: Word::Fails,
        input: || Scenario::SourceOrdinaryInterrupted,
        expect: Expect::Error(Failure::Interrupted),
    },
    Case {
        name: CaseName("source_named_capture_without_line_metadata_keeps_exact_output_shape"),
        word: Word::Succeeds,
        input: || Scenario::SourceNoLine,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            Cell::Number(Integer(7)),
            Cell::Span(Span {
                start: Offset(3),
                end: Offset(4),
            }),
            Cell::Span(Span {
                start: Offset(3),
                end: Offset(4),
            }),
            Cell::Symbol(Symbol(1)),
            Cell::Number(Integer(7)),
            Cell::Span(Span {
                start: Offset(5),
                end: Offset(6),
            }),
            Cell::Span(Span {
                start: Offset(5),
                end: Offset(6),
            }),
            Cell::Symbol(Symbol(2)),
        ]))),
    },
    Case {
        name: CaseName("source_span_only_capture_omits_text_and_capture_line_outputs"),
        word: Word::Succeeds,
        input: || Scenario::SourceSpanOnly,
        expect: Expect::Value(Observation::Cells(Cow::Borrowed(&[
            Cell::Number(Integer(7)),
            Cell::Span(Span {
                start: Offset(3),
                end: Offset(4),
            }),
            Cell::Number(Integer(2)),
            Cell::Span(Span {
                start: Offset(3),
                end: Offset(4),
            }),
            Cell::Number(Integer(7)),
            Cell::Span(Span {
                start: Offset(5),
                end: Offset(6),
            }),
            Cell::Number(Integer(3)),
            Cell::Span(Span {
                start: Offset(5),
                end: Offset(6),
            }),
        ]))),
    },
    Case {
        name: CaseName("count_reduction_dispatch_restores_first_occurrence_order"),
        word: Word::Succeeds,
        input: || Scenario::CountGroups,
        expect: Expect::Value(Observation::Groups(Cow::Borrowed(&[(0, 3), (1, 1)]))),
    },
    Case {
        name: CaseName("distinct_count_reduction_does_not_count_repeats"),
        word: Word::Succeeds,
        input: || Scenario::DistinctGroups,
        expect: Expect::Value(Observation::Groups(Cow::Borrowed(&[(0, 2), (1, 1)]))),
    },
    Case {
        name: CaseName("collected_count_operand_fault_precedes_key_fault"),
        word: Word::Refuses,
        input: || Scenario::CountOperandFirst,
        expect: Expect::Error(Failure::NoSuchColumn { index: 98 }),
    },
    Case {
        name: CaseName("digest_dispatch_uses_resolved_text_not_handles_repeats_or_order"),
        word: Word::Succeeds,
        input: || Scenario::DigestGroups,
        expect: Expect::Value(Observation::Groups(Cow::Borrowed(&[(0, 2)]))),
    },
    Case {
        name: CaseName("hash_dispatch_is_stable_for_equal_borrowed_text"),
        word: Word::Succeeds,
        input: || Scenario::HashValue,
        expect: Expect::Value(Observation::Hashes {
            same_text: true,
            different_numbers: true,
        }),
    },
    Case {
        name: CaseName("count_metadata_keeps_repeated_key_kinds"),
        word: Word::Succeeds,
        input: || Scenario::CheckCount,
        expect: Expect::Value(Observation::Kinds(Cow::Borrowed(&[
            CellKind::Number,
            CellKind::Number,
        ]))),
    },
    Case {
        name: CaseName("digest_metadata_keeps_key_kinds"),
        word: Word::Succeeds,
        input: || Scenario::CheckDigest,
        expect: Expect::Value(Observation::Kinds(Cow::Borrowed(&[CellKind::Number]))),
    },
    Case {
        name: CaseName("streamed_count_metadata_checks_keys_first"),
        word: Word::Refuses,
        input: || Scenario::CheckStreamCountOrder,
        expect: Expect::Error(Failure::NoSuchColumn { index: 99 }),
    },
    Case {
        name: CaseName("collected_count_metadata_checks_operand_first"),
        word: Word::Refuses,
        input: || Scenario::CheckCollectedCountOrder,
        expect: Expect::Error(Failure::NoSuchColumn { index: 98 }),
    },
    Case {
        name: CaseName("streamed_digest_metadata_checks_operand_first"),
        word: Word::Refuses,
        input: || Scenario::CheckStreamDigestOrder,
        expect: Expect::Error(Failure::NoSuchColumn { index: 98 }),
    },
    Case {
        name: CaseName("stream_count_and_finish_keep_original_representatives"),
        word: Word::Succeeds,
        input: || Scenario::StreamCount,
        expect: Expect::Value(Observation::Groups(Cow::Borrowed(&[(0, 2), (1, 1)]))),
    },
    Case {
        name: CaseName("stream_digest_and_finish_count_distinct_resolved_values"),
        word: Word::Succeeds,
        input: || Scenario::StreamDigest,
        expect: Expect::Value(Observation::Groups(Cow::Borrowed(&[(0, 2), (1, 1)]))),
    },
    Case {
        name: CaseName("stream_bad_early_row_prevents_later_upstream_demand"),
        word: Word::Refuses,
        input: || Scenario::StreamEarlyFault,
        expect: Expect::Error(Failure::NoSuchColumn { index: 1 }),
    },
    Case {
        name: CaseName("stream_group_storage_refusal_prevents_value_access"),
        word: Word::Fails,
        input: || Scenario::StreamStorageFailure,
        expect: Expect::Error(Failure::Interrupted),
    },
    Case {
        name: CaseName("finish_digest_sorts_deduplicates_and_keeps_group_coordinates"),
        word: Word::Succeeds,
        input: || Scenario::FinishDigest,
        expect: Expect::Value(Observation::Groups(Cow::Borrowed(&[(0, 2), (1, 1)]))),
    },
];
fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}
fn written(call: OperatorCall<'_>) -> Result<(), Failure> {
    assert_eq!(
        OperatorsUseCaseImpl::new().operators(call)?,
        OperatorAnswer::Written
    );
    Ok(())
}
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::Project
        | Scenario::Append
        | Scenario::MissingProjectColumn
        | Scenario::SelectRepeated
        | Scenario::SelectRejected
        | Scenario::SelectOriginal
        | Scenario::SelectMissing
        | Scenario::CountOutput
        | Scenario::DigestOutput
        | Scenario::AggregateMissing => rows::subject(input),
        Scenario::Words
        | Scenario::WordsEmpty
        | Scenario::CaptureUtf8
        | Scenario::CaptureSlash
        | Scenario::RankFirst
        | Scenario::RankAbsent
        | Scenario::ReplaceLiteral
        | Scenario::SourceLines
        | Scenario::SourceMissingCapture
        | Scenario::SourceMissingOptions
        | Scenario::SourceInterrupted
        | Scenario::SourceOrdinary
        | Scenario::SourceOrdinaryInterrupted
        | Scenario::SourceNoLine
        | Scenario::SourceSpanOnly => text::subject(input),
        Scenario::OrderedPairs | Scenario::OrderedMissing | Scenario::OrderedInterrupted => {
            ordered::subject(input)
        }
        Scenario::CountGroups
        | Scenario::DistinctGroups
        | Scenario::CountOperandFirst
        | Scenario::DigestGroups
        | Scenario::HashValue
        | Scenario::CheckCount
        | Scenario::CheckDigest
        | Scenario::CheckStreamCountOrder
        | Scenario::CheckCollectedCountOrder
        | Scenario::CheckStreamDigestOrder
        | Scenario::StreamCount
        | Scenario::StreamDigest
        | Scenario::StreamEarlyFault
        | Scenario::StreamStorageFailure
        | Scenario::FinishDigest => groups::subject(input),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
