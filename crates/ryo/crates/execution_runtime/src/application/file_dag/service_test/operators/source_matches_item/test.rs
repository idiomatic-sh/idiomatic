//! Physical occurrence spans and capture rows through actual compiled execution.
use super::super::fixtures::span;
use super::fixtures::{captured, evaluate, evaluate_lines};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::constants::NO_SPAN;
use ryo_relations::domain::relations::types::{CellKind, Span};
use std::borrow::Cow;
/// A source match over a source text: its spans and pattern, a span with a base line, or a
/// named capture.
#[derive(Clone, Copy, Debug)]
enum Matched {
    Spans(&'static [u8], &'static [Span], &'static str),
    Lines(&'static [u8], Span, i64, &'static str),
    Captures(&'static [u8], &'static [Span], &'static str),
}

/// The occurrence spans; each with its line; or each with the capture's span and text.
#[derive(Debug, PartialEq)]
enum Answer {
    Spans(Cow<'static, [Span]>),
    Lines(Cow<'static, [(Span, i64)]>),
    Captures(Cow<'static, [(Span, Span, Cow<'static, str>)]>),
}

/// The answer; the output's column kinds and whether the streamed run answered the collected
/// run's table (spans and captures only); and the text the probe comment interned (captures only).
#[derive(Debug, PartialEq)]
struct Matches {
    answer: Answer,
    kinds: Option<Cow<'static, [CellKind]>>,
    streamed_same: Option<bool>,
    comment: Option<Cow<'static, str>>,
}

type Input = Matched;
type Value = Matches;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("nonzero_offset_utf8_and_repeated_source_text_keep_exact_order_and_bag"),
        word: Word::Succeeds,
        input: || Matched::Spans("Zéé!".as_bytes(), const { &[span(1, 5), span(1, 5)] }, "é"),
        expect: Expect::Value(Matches {
            answer: Answer::Spans(Cow::Borrowed(&[
                span(1, 3),
                span(3, 5),
                span(1, 3),
                span(3, 5),
            ])),
            kinds: Some(Cow::Borrowed(&[CellKind::Span, CellKind::Span])),
            streamed_same: Some(true),
            comment: None,
        }),
    },
    Case {
        name: CaseName("a_pattern_absent_from_the_source_matches_nothing"),
        word: Word::Succeeds,
        input: || Matched::Spans(b"abc", const { &[span(0, 3)] }, "z"),
        expect: Expect::Value(Matches {
            answer: Answer::Spans(Cow::Borrowed(&[])),
            kinds: Some(Cow::Borrowed(&[CellKind::Span, CellKind::Span])),
            streamed_same: Some(true),
            comment: None,
        }),
    },
    Case {
        name: CaseName("zero_width_and_absent_spans_are_explicit"),
        word: Word::Succeeds,
        input: || Matched::Spans("éx".as_bytes(), const { &[span(0, 3)] }, ""),
        expect: Expect::Value(Matches {
            answer: Answer::Spans(Cow::Borrowed(&[span(0, 0), span(2, 2), span(3, 3)])),
            kinds: Some(Cow::Borrowed(&[CellKind::Span, CellKind::Span])),
            streamed_same: Some(true),
            comment: None,
        }),
    },
    Case {
        name: CaseName("an_empty_pattern_matches_an_empty_span_once_and_skips_an_absent_one"),
        word: Word::Succeeds,
        input: || Matched::Spans(b"abc", const { &[span(1, 1), NO_SPAN] }, ""),
        expect: Expect::Value(Matches {
            answer: Answer::Spans(Cow::Borrowed(&[span(1, 1)])),
            kinds: Some(Cow::Borrowed(&[CellKind::Span, CellKind::Span])),
            streamed_same: Some(true),
            comment: None,
        }),
    },
    Case {
        name: CaseName("malformed_or_non_utf8_boundary_spans_refuse_before_their_row_emits"),
        word: Word::Refuses,
        input: || Matched::Spans(b"abc", const { &[span(2, 1)] }, "é"),
        expect: Expect::Error(OperatingError::InvalidSourceMatchSpan),
    },
    Case {
        name: CaseName("a_span_past_the_source_refuses_before_its_row_emits"),
        word: Word::Refuses,
        input: || Matched::Spans(b"abc", const { &[span(0, 4)] }, "é"),
        expect: Expect::Error(OperatingError::InvalidSourceMatchSpan),
    },
    Case {
        name: CaseName("a_span_ending_inside_a_character_refuses_before_its_row_emits"),
        word: Word::Refuses,
        input: || Matched::Spans("Zé!".as_bytes(), const { &[span(2, 3)] }, "é"),
        expect: Expect::Error(OperatingError::InvalidSourceMatchSpan),
    },
    Case {
        name: CaseName("a_whole_file_match_never_invents_offsets_in_lossy_decoded_source"),
        word: Word::Refuses,
        input: || Matched::Spans(b"a\xffb", const { &[span(0, 3)] }, "a"),
        expect: Expect::Error(OperatingError::InvalidSourceMatchSpan),
    },
    Case {
        name: CaseName("optional_line_uses_match_start_and_counts_crlf_and_single_endings_once"),
        word: Word::Succeeds,
        input: || Matched::Lines(b"a\r\nb\rc\nd", span(0, 8), 1, "\nb|b\\rc|\nd|c"),
        expect: Expect::Value(Matches {
            answer: Answer::Lines(Cow::Borrowed(&[
                (span(2, 4), 1),
                (span(5, 6), 3),
                (span(6, 8), 3),
            ])),
            kinds: None,
            streamed_same: None,
            comment: None,
        }),
    },
    Case {
        name: CaseName("an_empty_pattern_counts_a_crlf_ending_once"),
        word: Word::Succeeds,
        input: || Matched::Lines(b"a\r\nb", span(0, 4), 1, ""),
        expect: Expect::Value(Matches {
            answer: Answer::Lines(Cow::Borrowed(&[
                (span(0, 0), 1),
                (span(1, 1), 1),
                (span(2, 2), 1),
                (span(3, 3), 2),
                (span(4, 4), 2),
            ])),
            kinds: None,
            streamed_same: None,
            comment: None,
        }),
    },
    Case {
        name: CaseName("optional_line_rejects_zero_and_overflow"),
        word: Word::Refuses,
        input: || Matched::Lines(b"x", span(0, 1), 0, "x"),
        expect: Expect::Error(OperatingError::InvalidSourceMatchSpan),
    },
    Case {
        name: CaseName("an_overflowing_line_is_refused"),
        word: Word::Refuses,
        input: || Matched::Lines(b"a\nb", span(0, 3), i64::MAX, "b"),
        expect: Expect::Error(OperatingError::InvalidSourceMatchSpan),
    },
    Case {
        name: CaseName("named_capture_preserves_offsets_order_bag_and_skips_unmatched_branches"),
        word: Word::Succeeds,
        input: || {
            Matched::Captures(
                "é\\q\\z".as_bytes(),
                const { &[span(2, 6), span(2, 6)] },
                r"(?P<piece>\\.)",
            )
        },
        expect: Expect::Value(Matches {
            answer: Answer::Captures(Cow::Borrowed(&[
                (span(2, 4), span(2, 4), Cow::Borrowed("\\q")),
                (span(4, 6), span(4, 6), Cow::Borrowed("\\z")),
                (span(2, 4), span(2, 4), Cow::Borrowed("\\q")),
                (span(4, 6), span(4, 6), Cow::Borrowed("\\z")),
            ])),
            kinds: Some(Cow::Borrowed(&[
                CellKind::Span,
                CellKind::Span,
                CellKind::Span,
                CellKind::Symbol,
            ])),
            streamed_same: None,
            comment: Some(Cow::Borrowed(" noqa: W605")),
        }),
    },
    Case {
        name: CaseName("an_unmatched_branch_captures_nothing"),
        word: Word::Succeeds,
        input: || Matched::Captures(b"\\qz", const { &[span(0, 3)] }, r"(?P<piece>\\q)|z"),
        expect: Expect::Value(Matches {
            answer: Answer::Captures(Cow::Borrowed(&[(
                span(0, 2),
                span(0, 2),
                Cow::Borrowed("\\q"),
            )])),
            kinds: Some(Cow::Borrowed(&[
                CellKind::Span,
                CellKind::Span,
                CellKind::Span,
                CellKind::Symbol,
            ])),
            streamed_same: None,
            comment: Some(Cow::Borrowed(" noqa: W605")),
        }),
    },
    Case {
        name: CaseName("an_empty_capture_is_kept_at_every_character_boundary"),
        word: Word::Succeeds,
        input: || Matched::Captures("é".as_bytes(), const { &[span(0, 2)] }, r"(?P<piece>)"),
        expect: Expect::Value(Matches {
            answer: Answer::Captures(Cow::Borrowed(&[
                (span(0, 0), span(0, 0), Cow::Borrowed("")),
                (span(2, 2), span(2, 2), Cow::Borrowed("")),
            ])),
            kinds: Some(Cow::Borrowed(&[
                CellKind::Span,
                CellKind::Span,
                CellKind::Span,
                CellKind::Symbol,
            ])),
            streamed_same: None,
            comment: Some(Cow::Borrowed(" noqa: W605")),
        }),
    },
];

/// Runs the source match through actual compiled execution.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Matched::Spans(source, spans, pattern) => {
            evaluate(source, spans, pattern).map(|(spans, kinds, streamed_same)| Matches {
                answer: Answer::Spans(Cow::Owned(spans)),
                kinds: Some(Cow::Owned(kinds)),
                streamed_same: Some(streamed_same),
                comment: None,
            })
        }
        Matched::Lines(source, span, line, pattern) => evaluate_lines(source, span, line, pattern)
            .map(|lines| Matches {
                answer: Answer::Lines(Cow::Owned(lines)),
                kinds: None,
                streamed_same: None,
                comment: None,
            }),
        Matched::Captures(source, spans, pattern) => {
            captured(source, spans, pattern).map(|(captures, kinds, comment)| Matches {
                answer: Answer::Captures(Cow::Owned(
                    captures
                        .into_iter()
                        .map(|(at, piece, text)| (at, piece, Cow::Owned(text)))
                        .collect(),
                )),
                kinds: Some(Cow::Owned(kinds)),
                streamed_same: None,
                comment: comment.map(Cow::Owned),
            })
        }
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
