//! Public text calls keep matcher construction at the edge and preserve exact delivery prefixes.
use super::*;
use ryo_relations::domain::relations::types::{Offset, Span};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::plan_parts::types::LiteralText;

fn regex(pattern: &str) -> regex_automata::meta::Regex {
    regex_automata::meta::Regex::new(pattern).expect("valid acceptance pattern")
}
fn span(start: u32, end: u32) -> Cell {
    Cell::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}
#[derive(Default)]
struct Delivery {
    rows: Vec<Vec<Cell>>,
    attempts: usize,
    interns: usize,
    names: Vec<String>,
    fail_second: bool,
}
impl TextRowDriver for Delivery {
    fn intern(&mut self, text: &str) -> Cell {
        self.interns += 1;
        assert!(
            matches!(text, "b" | "c" | "d"),
            "capture resolves the original physical text"
        );
        self.names.push(text.to_owned());
        Cell::Symbol(Symbol(u32::try_from(self.interns).unwrap()))
    }
    fn emit(&mut self, row: &[Cell]) -> Result<(), SemanticInterrupted> {
        self.attempts += 1;
        if self.fail_second && self.attempts == 2 {
            return Err(SemanticInterrupted);
        }
        self.rows.push(row.to_vec());
        Ok(())
    }
}
fn source_optional(input: Scenario) -> Result<Observation, Failure> {
    let interrupted = matches!(input, Scenario::SourceOrdinaryInterrupted);
    let ordinary = matches!(
        input,
        Scenario::SourceOrdinary | Scenario::SourceOrdinaryInterrupted
    );
    let source = if interrupted {
        "a\r\nb\rc\nd"
    } else {
        "a\r\nb\rc"
    };
    let regex = regex("(?P<cap>b|c|d)");
    let mut delivery = Delivery {
        fail_second: interrupted,
        ..Delivery::default()
    };
    let mut output = Vec::new();
    let result = written(OperatorCall::SourceMatches {
        input: SourceMatchesInput {
            row: &[Cell::Number(Integer(7))],
            source: source.as_bytes(),
            base: 0,
            text: source,
            line: if matches!(input, Scenario::SourceSpanOnly) {
                Some(1)
            } else {
                None
            },
            regex: &regex,
            capture: if ordinary { None } else { Some("cap") },
            capture_text: matches!(input, Scenario::SourceNoLine),
            capture_line: false,
        },
        driver: &mut delivery,
        target: &mut output,
    });
    if ordinary {
        let expected = if interrupted {
            vec![vec![number(7), span(3, 4)]]
        } else {
            vec![vec![number(7), span(3, 4)], vec![number(7), span(5, 6)]]
        };
        assert_eq!(
            delivery.rows, expected,
            "ordinary matches retain the original row and append only the whole span"
        );
        assert_eq!(
            delivery.attempts, 2,
            "second refusal prevents any third delivery"
        );
        assert_eq!(delivery.interns, 0);
        assert!(
            delivery.names.is_empty(),
            "ordinary find_iter never interns capture text"
        );
    } else if matches!(input, Scenario::SourceNoLine) {
        assert_eq!(
            delivery.rows,
            vec![
                vec![number(7), span(3, 4), span(3, 4), Cell::Symbol(Symbol(1))],
                vec![number(7), span(5, 6), span(5, 6), Cell::Symbol(Symbol(2))],
            ]
        );
        assert_eq!((delivery.attempts, delivery.interns), (2, 2));
        assert_eq!(delivery.names, ["b", "c"]);
    } else {
        assert_eq!(
            delivery.rows,
            vec![
                vec![number(7), span(3, 4), number(2), span(3, 4)],
                vec![number(7), span(5, 6), number(3), span(5, 6)],
            ]
        );
        assert_eq!((delivery.attempts, delivery.interns), (2, 0));
        assert!(
            delivery.names.is_empty(),
            "span-only captures never intern text"
        );
    }
    result?;
    Ok(Observation::Cells(Cow::Owned(
        delivery.rows.into_iter().flatten().collect(),
    )))
}
fn source(input: Scenario) -> Result<Observation, Failure> {
    let source = if matches!(input, Scenario::SourceInterrupted) {
        "a\r\nb\rc\nd"
    } else {
        "a\r\nb\rc"
    };
    let regex = regex(if matches!(input, Scenario::SourceMissingCapture) {
        "(?P<cap>x)"
    } else {
        "(?P<cap>b|c|d)"
    });
    let capture = match input {
        Scenario::SourceMissingCapture => Some("absent"),
        Scenario::SourceMissingOptions => None,
        _ => Some("cap"),
    };
    let mut delivery = Delivery {
        fail_second: matches!(input, Scenario::SourceInterrupted),
        ..Delivery::default()
    };
    let mut output = vec![number(99)];
    let result = written(OperatorCall::SourceMatches {
        input: SourceMatchesInput {
            row: &[],
            source: source.as_bytes(),
            base: 0,
            text: source,
            line: Some(1),
            regex: &regex,
            capture,
            capture_text: true,
            capture_line: true,
        },
        driver: &mut delivery,
        target: &mut output,
    });
    match input {
        Scenario::SourceLines => {
            assert_eq!(
                delivery.rows,
                vec![
                    vec![
                        span(3, 4),
                        number(2),
                        span(3, 4),
                        Cell::Symbol(Symbol(1)),
                        number(2)
                    ],
                    vec![
                        span(5, 6),
                        number(3),
                        span(5, 6),
                        Cell::Symbol(Symbol(2)),
                        number(3)
                    ]
                ]
            );
            assert_eq!((delivery.attempts, delivery.interns), (2, 2));
            assert_eq!(delivery.names, ["b", "c"]);
        }
        Scenario::SourceInterrupted => {
            assert_eq!(
                delivery.rows,
                vec![vec![
                    span(3, 4),
                    number(2),
                    span(3, 4),
                    Cell::Symbol(Symbol(1)),
                    number(2)
                ]]
            );
            assert_eq!(
                (delivery.attempts, delivery.interns),
                (2, 2),
                "failure prevents the third capture and downstream delivery"
            );
            assert_eq!(delivery.names, ["b", "c"]);
        }
        Scenario::SourceMissingCapture | Scenario::SourceMissingOptions => {
            assert!(delivery.rows.is_empty());
            assert_eq!((delivery.attempts, delivery.interns), (0, 0));
            assert_eq!(
                output,
                vec![number(99)],
                "option faults precede output mutation"
            );
        }
        _ => unreachable!(),
    }
    result?;
    Ok(Observation::Cells(Cow::Owned(
        delivery.rows.iter().map(|row| row[1]).collect(),
    )))
}
pub(super) fn subject(input: Scenario) -> Result<Observation, Failure> {
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
                text,
                word: &mut word,
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
                pattern,
                text,
                slots: &mut slots,
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
            let answer = OperatorsUseCaseImpl::new().operators(OperatorCall::Rank {
                text,
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
                text: "aaé",
                replacement: "$1",
                target: &mut output,
            })?;
            assert_eq!(output.as_ptr(), original);
            Ok(Observation::Text(Cow::Owned(output)))
        }
        Scenario::SourceLines
        | Scenario::SourceMissingCapture
        | Scenario::SourceMissingOptions
        | Scenario::SourceInterrupted => source(input),
        Scenario::SourceOrdinary
        | Scenario::SourceOrdinaryInterrupted
        | Scenario::SourceNoLine
        | Scenario::SourceSpanOnly => source_optional(input),
        _ => unreachable!("text fixture receives only text scenarios"),
    }
}
