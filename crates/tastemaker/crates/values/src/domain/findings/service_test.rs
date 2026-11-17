//! What the `findings` service renders, places and identifies.

use std::collections::BTreeMap;

use super::service::{identities, indexed_span, render, settle, spanning};
use super::types::{ByteOffset, MessageTemplate, Placeholder, PlaceholderValue, SourceText};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::findings::service::finding;
use crate::domain::findings::types::{Help, Place};
use crate::domain::paths::types::RepoPath;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    Render,
    Spanning,
    Ordinals,
    MessageMoves,
    Settle,
    OutOfText,
    /// A check of an entry is the entry and the check apart, joined only when printed, and read back from that text.
    Checks,
}

/// What a row asks.
type Input = Question;
/// What the service answers.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_check_keeps_its_entry_and_check_apart_and_joins_them_only_when_printed"),
        word: Word::Succeeds,
        input: || Question::Checks,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_message_names_its_columns_in_braces"),
        word: Word::Succeeds,
        input: || Question::Render,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_byte_range_is_read_as_lines_and_columns"),
        word: Word::Succeeds,
        input: || Question::Spanning,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("findings_with_the_same_parts_are_told_apart_by_their_order"),
        word: Word::Succeeds,
        input: || Question::Ordinals,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_finding_whose_message_alone_changes_keeps_its_identity"),
        word: Word::Succeeds,
        input: || Question::MessageMoves,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_finding_reported_twice_is_kept_once"),
        word: Word::Succeeds,
        input: || Question::Settle,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_range_outside_the_text_is_refused"),
        word: Word::Refuses,
        input: || Question::OutOfText,
        expect: Expect::error(()),
    },
];

fn whole(message: &str) -> crate::domain::findings::types::Finding {
    finding(
        crate::domain::findings::service::of_rule(crate::domain::findings::types::RuleId(
            "p/r".into(),
        )),
        RepoPath("a.txt".into()),
        Place::Whole,
        message.to_owned(),
        Help::Silent,
    )
}

fn subject(question: Input) -> Result<Value, Failure> {
    let (said, expected) = match question {
        Question::Render => {
            let mut values = BTreeMap::new();
            drop(values.insert(Placeholder("path"), PlaceholderValue("a.txt".to_owned())));
            let template = MessageTemplate("`{path}` has {lines} lines".into());
            let rendered = render(&template, &values).0;
            (rendered, "`a.txt` has {lines} lines".to_owned())
        }
        Question::Spanning => {
            // Every offset includes invalid UTF-8 boundaries and one past EOF;
            // each valid range must retain the scalar-column and newline contract.
            for text in ["", "ab\ncd\n", "é🙂\r\nx\n\n", "\r", "\n", "abc"] {
                let source = SourceText(text);
                let starts = std::iter::once(0)
                    .chain(text.bytes().enumerate().filter_map(|(offset, byte)| {
                        let next = (byte == b'\n').then_some(offset + 1)?;
                        u64::try_from(next).ok()
                    }))
                    .collect::<Vec<_>>();
                let indexed = crate::domain::files::types::SourceView {
                    text: crate::domain::files::types::BorrowedText(text),
                    starts: crate::domain::files::types::LineStarts(&starts),
                };
                for start in 0..=text.len() + 1 {
                    for end in start..=text.len() + 1 {
                        assert_eq!(
                            indexed_span(indexed, ByteOffset(start), ByteOffset(end)),
                            spanning(source, ByteOffset(start), ByteOffset(end))
                        );
                    }
                }
            }
            let region = spanning(SourceText("ab\ncd\n"), ByteOffset(3), ByteOffset(5)).ok_or(())?;
            (
                format!(
                    "{}:{}-{}:{}",
                    region.start_line.0,
                    region.start_column.0,
                    region.end_line.0,
                    region.end_column.0
                ),
                "2:1-2:3".to_owned(),
            )
        }
        Question::Ordinals => {
            let found = [whole("m"), whole("m")];
            let ordinals = identities(&found, |_| None)
                .iter()
                .map(|identifying| identifying.ordinal.0.to_string())
                .collect::<Vec<_>>();
            (ordinals.join(","), "0,1".to_owned())
        }
        Question::MessageMoves => {
            let at_line = |message: &str| crate::domain::findings::types::Finding {
                place: Place::Line(crate::domain::findings::types::Line(3)),
                ..whole(message)
            };
            let parts = |message: &str| {
                identities(&[at_line(message)], |_| {
                    Some(crate::domain::findings::types::FirstLine(
                        "fn f() {".to_owned(),
                    ))
                })
                .into_iter()
                .map(|identifying| identifying.parts)
                .collect::<Vec<_>>()
            };
            (
                format!("{}", parts("has 3 lines") == parts("has 4 lines")),
                "true".to_owned(),
            )
        }
        Question::Settle => {
            let mut found = vec![whole("m"), whole("m"), whole("n")];
            settle(&mut found);
            (found.len().to_string(), "2".to_owned())
        }
        Question::Checks => {
            use super::service::{check_id, of_rule, printed};
            use super::types::{CheckChoice, CheckName, RuleId};
            let check = check_id("p/crate", "count");
            let apart = check.rule == RuleId("p/crate".into())
                && check.check == CheckChoice::Named(CheckName("count".into()));
            let rule = of_rule(RuleId("p/file-length".into()));
            (
                format!("{apart} {} {}", printed(&check).0, printed(&rule).0),
                "true p/crate#count p/file-length".to_owned(),
            )
        }
        Question::OutOfText => {
            spanning(SourceText("ab"), ByteOffset(0), ByteOffset(9))
                .map(drop)
                .ok_or(())?;
            (String::new(), String::new())
        }
    };
    Ok(if said == expected {
        String::new()
    } else {
        said
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
