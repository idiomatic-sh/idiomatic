//! The cases of a pipeline's `Call` stage, each run through `made` on a pack the contract's encoding wrote: the cases of
//! each meaning the fill side runs, over the file's nodes (a root spanning `ab cd ef` and three tokens). A case says its
//! table's rows, each column as what it is, and the fill's work.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::Op;
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::UnbuiltFill;
use crate::domain::lowered::types::MadeTable;
use crate::domain::made_tables::fixtures::packs::{
    keep_equal, node_read, of, operands, pipeline_of, rows, scan_nodes, word,
};
use crate::domain::made_tables::fixtures::{Shown, Telling, framed, shown_as, world_with};
use crate::domain::made_tables::sources::service::{made, table_of};
use crate::domain::steps::calls::types::{SearchGroup, SearchLines, SourceSearch};
use crate::domain::steps::stages::types::FillStage;
use crate::domain::steps::types::{
    CallMeaning, MadeTableId, RegexId, Register, RegisterList, StepOperand,
};
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::constants::NO_SPAN;
use ryo_relations::domain::relations::service::to_bits;
use ryo_relations::domain::relations::types::{NodeColumn, Offset, Relation, Span, Value};
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
use std::borrow::Cow;

/// The call a case makes.
#[derive(Clone, Copy, Debug)]
enum Called {
    /// The start and the end of each token's span.
    SpanBounds,
    /// Each token's text with every `b` and `d` replaced.
    Replace,
    /// The slot of a pattern that matches one token's text.
    Capture,
    /// The two slots of a pattern that matches every token's text in one way.
    CaptureTwo,
    /// Every word of the root's span.
    SourceMatches,
    /// Every two letters of the root's span, with the line, and the first letter as a named group with its text and line.
    SourceMatchesGroup,
    /// The bounds of the absent span.
    AbsentSpan,
    /// The span from each token's start to the last token's end.
    SpanBetween,
    /// The same with the absent span as the second.
    SpanBetweenAbsent,
    /// The same with the first token's span as the second: the second token starts after it ends.
    SpanBetweenApart,
    /// Each token's text, a constant and the token's text again, one after the other.
    Concat,
    /// The cell that is no word, and a constant.
    ConcatNoText,
    /// Each token's text in lowercase, over a file whose tokens hold capitals.
    Lower,
    /// A meaning the fill side does not run: no rule is lowered to it.
    Words,
}

type Input = Called;
type Answer = Cow<'static, str>;
type Failure = FilledError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Answer, Failure>] = &[
    Case {
        name: CaseName("a_call_of_span_bounds_gives_each_spans_two_ends"),
        word: Word::Succeeds,
        input: || Called::SpanBounds,
        expect: Expect::Value(Cow::Borrowed(
            "n4 0 2; n5 3 5; n6 6 8 | read 33 given 25 made 3",
        )),
    },
    Case {
        name: CaseName("a_call_of_replace_gives_the_text_with_every_occurrence_replaced"),
        word: Word::Succeeds,
        input: || Called::Replace,
        expect: Expect::Value(Cow::Borrowed(
            "\"ab\" \"a_\"; \"cd\" \"c_\"; \"ef\" \"ef\" | read 33 given 25 made 3",
        )),
    },
    Case {
        name: CaseName("a_call_of_capture_gives_a_row_for_each_way_the_pattern_matches"),
        word: Word::Succeeds,
        input: || Called::Capture,
        expect: Expect::Value(Cow::Borrowed("\"ab\" \"a\" | read 31 given 23 made 1")),
    },
    Case {
        name: CaseName("a_capture_of_two_slots_gives_both_texts"),
        word: Word::Succeeds,
        input: || Called::CaptureTwo,
        expect: Expect::Value(Cow::Borrowed(
            "\"ab\" \"a\" \"b\"; \"cd\" \"c\" \"d\"; \"ef\" \"e\" \"f\" | read 33 given 25 made 3",
        )),
    },
    Case {
        name: CaseName("a_source_search_gives_a_row_for_each_occurrence_left_to_right"),
        word: Word::Succeeds,
        input: || Called::SourceMatches,
        expect: Expect::Value(Cow::Borrowed(
            "n0 0..2; n0 3..5; n0 6..8 | read 29 given 21 made 3",
        )),
    },
    Case {
        name: CaseName("a_source_search_with_lines_and_a_group_gives_its_span_text_and_line"),
        word: Word::Succeeds,
        input: || Called::SourceMatchesGroup,
        expect: Expect::Value(Cow::Borrowed(
            "n0 0..2 1 0..1 \"a\" 1; n0 3..5 1 3..4 \"c\" 1; n0 6..8 1 6..7 \"e\" 1 | read 29 given 21 made 3",
        )),
    },
    Case {
        name: CaseName("a_meanings_fault_on_a_row_is_handed_on"),
        word: Word::Refuses,
        input: || Called::AbsentSpan,
        expect: Expect::Error(FilledError::Meaning {
            fault: PortableOperatorFault::InvalidSpanBounds,
        }),
    },
    Case {
        name: CaseName(
            "a_call_of_span_between_gives_the_span_from_the_firsts_start_to_the_seconds_end",
        ),
        word: Word::Succeeds,
        input: || Called::SpanBetween,
        expect: Expect::Value(Cow::Borrowed(
            "n4 0..8; n5 3..8; n6 6..8 | read 33 given 25 made 3",
        )),
    },
    Case {
        name: CaseName("a_span_between_of_an_absent_span_is_the_absent_span"),
        word: Word::Succeeds,
        input: || Called::SpanBetweenAbsent,
        expect: Expect::Value(Cow::Borrowed(
            "n4 4294967295..0; n5 4294967295..0; n6 4294967295..0 | read 33 given 25 made 3",
        )),
    },
    Case {
        name: CaseName(
            "a_span_between_whose_first_starts_after_the_second_ends_is_the_meanings_fault",
        ),
        word: Word::Refuses,
        input: || Called::SpanBetweenApart,
        expect: Expect::Error(FilledError::Meaning {
            fault: PortableOperatorFault::InvalidSpan,
        }),
    },
    Case {
        name: CaseName("a_call_of_concat_gives_the_texts_one_after_the_other"),
        word: Word::Succeeds,
        input: || Called::Concat,
        expect: Expect::Value(Cow::Borrowed(
            "\"ab\" \"ab_ab\"; \"cd\" \"cd_cd\"; \"ef\" \"ef_ef\" | read 33 given 25 made 3",
        )),
    },
    Case {
        name: CaseName("a_concat_of_a_cell_that_is_no_text_gives_the_cell_that_is_no_word"),
        word: Word::Succeeds,
        input: || Called::ConcatNoText,
        expect: Expect::Value(Cow::Borrowed(
            "n4 18446744073709551615; n5 18446744073709551615; n6 18446744073709551615 | read 33 given 25 made 3",
        )),
    },
    Case {
        name: CaseName("a_call_of_lower_gives_the_text_in_lowercase"),
        word: Word::Succeeds,
        input: || Called::Lower,
        expect: Expect::Value(Cow::Borrowed(
            "\"Ab\" \"ab\"; \"cD\" \"cd\"; \"EF\" \"ef\" | read 33 given 25 made 3",
        )),
    },
    Case {
        name: CaseName("a_meaning_the_fill_side_does_not_run_yet_is_said_so"),
        word: Word::Refuses,
        input: || Called::Words,
        expect: Expect::Error(FilledError::Unbuilt {
            unbuilt: UnbuiltFill::Stage(Op::StageCall),
        }),
    },
];

/// A span of the file.
const fn span(start: u32, end: u32) -> Span {
    Span {
        start: Offset(start),
        end: Offset(end),
    }
}

/// A call stage.
fn call<const N: usize>(
    meaning: CallMeaning,
    arguments: [StepOperand; N],
    outputs: &[u32],
) -> FillStage {
    FillStage::Call {
        meaning,
        arguments: operands(arguments),
        outputs: RegisterList(outputs.iter().map(|each| Register(*each)).collect()),
    }
}

/// The table of a case and how each of its columns is shown: the tokens, or the root, with a column read and the call.
fn table_of_case(called: Called) -> (MadeTable, Vec<Shown>) {
    let tree = Relation::SemanticSyntax;
    let over = |kind: u64,
                column: NodeColumn,
                stage: FillStage,
                row: Vec<StepOperand>,
                shown: Vec<Shown>| {
        let mut all = Vec::from(scan_nodes(tree));
        all.push(keep_equal(1, kind));
        all.push(node_read(tree, column, 0, 2));
        all.push(stage);
        let width = row.len();
        let piped = pipeline_of(all, row);
        (rows(width, piped), shown)
    };
    let search = |lines, group| SourceSearch { lines, group };
    match called {
        Called::SpanBounds => over(
            13,
            NodeColumn::Span,
            call(CallMeaning::SpanBounds, [of(2)], &[3, 4]),
            vec![of(0), of(3), of(4)],
            vec![Shown::Node, Shown::Number, Shown::Number],
        ),
        Called::Replace => over(
            13,
            NodeColumn::Text,
            call(CallMeaning::Replace(RegexId(1)), [of(2), word(4)], &[3]),
            vec![of(2), of(3)],
            vec![Shown::Text, Shown::Text],
        ),
        Called::Capture => over(
            13,
            NodeColumn::Text,
            call(CallMeaning::Capture, [of(2), word(5)], &[3]),
            vec![of(2), of(3)],
            vec![Shown::Text, Shown::Text],
        ),
        Called::CaptureTwo => over(
            13,
            NodeColumn::Text,
            call(CallMeaning::Capture, [of(2), word(6)], &[3, 4]),
            vec![of(2), of(3), of(4)],
            vec![Shown::Text, Shown::Text, Shown::Text],
        ),
        Called::SourceMatches => over(
            11,
            NodeColumn::Span,
            call(
                CallMeaning::SourceMatches(
                    RegexId(2),
                    search(SearchLines::Unnumbered, SearchGroup::Whole),
                ),
                [of(2)],
                &[3],
            ),
            vec![of(0), of(3)],
            vec![Shown::Node, Shown::Span],
        ),
        Called::SourceMatchesGroup => over(
            11,
            NodeColumn::Span,
            call(
                CallMeaning::SourceMatches(
                    RegexId(3),
                    search(
                        SearchLines::Numbered,
                        SearchGroup::Named {
                            text: TextOutput::Appended,
                            line: LineOutput::Appended,
                        },
                    ),
                ),
                [of(2), word(1), word(7)],
                &[3, 4, 5, 6, 7],
            ),
            vec![of(0), of(3), of(4), of(5), of(6), of(7)],
            vec![
                Shown::Node,
                Shown::Span,
                Shown::Number,
                Shown::Span,
                Shown::Text,
                Shown::Number,
            ],
        ),
        Called::AbsentSpan => over(
            13,
            NodeColumn::Span,
            call(
                CallMeaning::SpanBounds,
                [word(to_bits(Value::Span(NO_SPAN)).0)],
                &[3, 4],
            ),
            vec![of(0), of(3), of(4)],
            vec![Shown::Node, Shown::Number, Shown::Number],
        ),
        Called::SpanBetween | Called::SpanBetweenAbsent | Called::SpanBetweenApart => over(
            13,
            NodeColumn::Span,
            call(
                CallMeaning::SpanBetween,
                [
                    of(2),
                    word(
                        to_bits(Value::Span(match called {
                            Called::SpanBetweenAbsent => NO_SPAN,
                            Called::SpanBetweenApart => span(0, 2),
                            Called::SpanBounds
                            | Called::Replace
                            | Called::Capture
                            | Called::CaptureTwo
                            | Called::SourceMatches
                            | Called::SourceMatchesGroup
                            | Called::AbsentSpan
                            | Called::SpanBetween
                            | Called::Concat
                            | Called::ConcatNoText
                            | Called::Lower
                            | Called::Words => span(6, 8),
                        }))
                        .0,
                    ),
                ],
                &[3],
            ),
            vec![of(0), of(3)],
            vec![Shown::Node, Shown::Span],
        ),
        Called::Concat => over(
            13,
            NodeColumn::Text,
            call(CallMeaning::Concat, [of(2), word(4), of(2)], &[3]),
            vec![of(2), of(3)],
            vec![Shown::Text, Shown::Text],
        ),
        Called::ConcatNoText => over(
            13,
            NodeColumn::Span,
            call(CallMeaning::Concat, [word(u64::MAX), word(4)], &[3]),
            vec![of(0), of(3)],
            vec![Shown::Node, Shown::Number],
        ),
        Called::Lower => over(
            13,
            NodeColumn::Text,
            call(CallMeaning::Lower, [of(2)], &[3]),
            vec![of(2), of(3)],
            vec![Shown::Text, Shown::Text],
        ),
        Called::Words => over(
            13,
            NodeColumn::Text,
            call(CallMeaning::Words, [of(2)], &[3]),
            vec![of(2), of(3)],
            vec![Shown::Text, Shown::Text],
        ),
    }
}

fn subject(called: Input) -> Result<Answer, Failure> {
    let (table, shown) = table_of_case(called);
    let mut world = world_with(vec![table], Vec::new(), &[], Telling::Keyed)?;
    if matches!(called, Called::Lower) {
        // The same file with capitals in its tokens.
        world.text = b"Ab cD EF".to_vec();
    }
    {
        let (mut frame, looped) = framed(&mut world);
        let _now = made(&mut frame, looped, MadeTableId(0))?;
    }
    let work = table_of(&world.store, MadeTableId(0))
        .map(|slot| slot.work)
        .unwrap_or_default();
    Ok(Cow::Owned(format!(
        "{} | read {} given {} made {}",
        shown_as(&world, MadeTableId(0), &shown),
        work.read.0,
        work.given.0,
        work.made.0
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
