//! Public text calls preserve Unicode word, path-capture, rank and literal replacement semantics.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::capture::types::*;
use crate::domain::contract::types::*;
use crate::domain::equality_join::types::*;
use crate::domain::ordered_existence::types::*;
use crate::domain::ordered_match::types::*;
use crate::domain::reduction::types::*;
use crate::domain::replace::types::{ReplacedText, ReplacementText};
use crate::domain::set_rows::types::*;
use crate::domain::source_matches::types::*;
use crate::domain::words::types::*;
use crate::inbound::use_cases::operators::types::*;
use crate::inbound::use_cases::operators::{fixtures, interface::OperatorsUseCase};
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::row_texts::types::CellText;
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    Words,
    WordsEmpty,
    CaptureUtf8,
    CaptureSlash,
    RankFirst,
    RankAbsent,
    ReplaceLiteral,
}
#[derive(Debug, PartialEq)]
enum Observation {
    Text(Cow<'static, str>),
    Rank(Option<i64>),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
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
];
fn written(call: OperatorCall<'_>) -> Result<(), Failure> {
    assert_eq!(
        fixtures::operators().operators(call)?,
        OperatorAnswer::Written
    );
    Ok(())
}
fn regex(pattern: &str) -> ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex {
    fixtures::regex(pattern)
}
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::Words | Scenario::WordsEmpty => {
            let text = if matches!(input, Scenario::Words) {
                "HTTPServer2Go_ÜberName"
            } else {
                "_- ."
            };
            let mut word = String::with_capacity(64);
            let original = word.as_ptr();
            let mut words = Vec::new();
            written(OperatorCall::Words {
                text: CellText(text),
                word: WordScratch(&mut word),
                each: &mut |word: &str| words.push(word.to_owned()),
            })?;
            assert!(
                word.is_empty(),
                "the engine-owned word scratch has no retained logical contents"
            );
            assert_eq!(
                word.as_ptr(),
                original,
                "the borrowed scratch keeps its capacity"
            );
            Ok(Observation::Text(Cow::Owned(words.join("|"))))
        }
        Scenario::CaptureUtf8 | Scenario::CaptureSlash => {
            let (pattern, text) = if matches!(input, Scenario::CaptureUtf8) {
                ("{a}{b}", "éab")
            } else {
                ("{a}.rs", "a/b.rs")
            };
            let mut slots = Vec::with_capacity(8);
            let original = slots.as_ptr();
            let mut captures: Vec<Vec<String>> = Vec::new();
            written(OperatorCall::Capture {
                pattern: SlotPattern(pattern),
                text: SlotSubject(text),
                slots: CaptureSlots(&mut slots),
                each: &mut |ranges: &[std::ops::Range<usize>]| {
                    captures.push(
                        ranges
                            .iter()
                            .map(|range| {
                                text.get(range.clone())
                                    .expect("capture is on UTF-8 boundaries")
                                    .to_owned()
                            })
                            .collect(),
                    );
                },
            })?;
            assert!(
                slots.is_empty(),
                "capture unwinds its borrowed range scratch"
            );
            assert_eq!(slots.as_ptr(), original);
            Ok(Observation::Text(Cow::Owned(
                captures
                    .iter()
                    .map(|slots| slots.join(","))
                    .collect::<Vec<_>>()
                    .join("|"),
            )))
        }
        Scenario::RankFirst | Scenario::RankAbsent => {
            let layers = [
                vec![LiteralText("same".into())],
                vec![LiteralText("same".into()), LiteralText("other".into())],
            ];
            let text = if matches!(input, Scenario::RankFirst) {
                "same"
            } else {
                "absent"
            };
            let answer = fixtures::operators().operators(OperatorCall::Rank {
                text: CellText(text),
                layers: &layers,
            })?;
            let OperatorAnswer::Rank(rank) = answer else {
                panic!("rank must return its selected layer")
            };
            Ok(Observation::Rank(rank.map(|rank| rank.0)))
        }
        Scenario::ReplaceLiteral => {
            let regex = regex("a");
            let mut output = String::with_capacity(32);
            output.push_str("stale");
            let original = output.as_ptr();
            written(OperatorCall::Replace {
                regex: &regex,
                text: CellText("aaé"),
                replacement: ReplacementText("$1"),
                target: ReplacedText(&mut output),
            })?;
            assert_eq!(output.as_ptr(), original);
            Ok(Observation::Text(Cow::Owned(output)))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
