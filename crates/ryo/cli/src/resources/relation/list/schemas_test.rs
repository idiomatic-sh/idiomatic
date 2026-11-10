//! What the schemas of this command accept and refuse, one row of `CASES` for each input.

use std::borrow::Cow;

use ryo_core::inbound::use_cases::list_relations::types::ListRelationsReport;
use ryo_values::domain::descriptions::types::RelationSummary;

use ryo_relations::domain::labels::types::Holds;
use ryo_values::domain::descriptions::types::RelationName;

use super::AnswerSchema;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = ListRelationsReport;
/// What the answer prints, and the JSON a script reads: the shape written out here, not what the answer's serialization happens to
/// give.
type Value = (Cow<'static, str>, Cow<'static, str>);

/// No conversion of this command refuses: its report always becomes an answer.
type Failure = std::convert::Infallible;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_relation_prints_as_its_name_a_tab_and_its_summary"),
        word: Word::Succeeds,
        input: || ListRelationsReport {
            relations: vec![RelationSummary {
                name: RelationName("function".into()),
                summary: Holds("every function"),
            }],
        },
        expect: Expect::Value((
            Cow::Borrowed("function\tevery function\n"),
            Cow::Borrowed(r#"{"relations":[{"name":"function","summary":"every function"}]}"#),
        )),
    },
    Case {
        name: CaseName("relations_print_one_to_a_line_in_the_order_given"),
        word: Word::Succeeds,
        input: || ListRelationsReport {
            relations: vec![
                RelationSummary {
                    name: RelationName("call".into()),
                    summary: Holds("calls"),
                },
                RelationSummary {
                    name: RelationName("function".into()),
                    summary: Holds("functions"),
                },
            ],
        },
        expect: Expect::Value((
            Cow::Borrowed("call\tcalls\nfunction\tfunctions\n"),
            Cow::Borrowed(
                r#"{"relations":[{"name":"call","summary":"calls"},{"name":"function","summary":"functions"}]}"#,
            ),
        )),
    },
];

/// Converts the report into the answer and returns the text it prints and the JSON it writes.
fn subject(report: Input) -> Value {
    let answer = AnswerSchema::from(report);
    // `--format json` prints the bytes serde_json writes of the answer, on one line.
    assert_eq!(
        encoded(&answer, crate::output::Spacing::Compact),
        serde_json::to_vec(&answer).unwrap_or_default()
    );
    assert_eq!(
        encoded(&answer, crate::output::Spacing::Indented),
        serde_json::to_vec_pretty(&answer).unwrap_or_default()
    );
    (
        Cow::Owned(answer.to_string()),
        Cow::Owned(
            String::from_utf8(encoded(&answer, crate::output::Spacing::Compact))
                .unwrap_or_default(),
        ),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

/// What `answer` writes as JSON, spaced as `spacing` says.
fn encoded(answer: &impl crate::output::Encoded, spacing: crate::output::Spacing) -> Vec<u8> {
    let mut written = Vec::new();
    answer.json(&mut written, spacing).unwrap_or_default();
    written
}
