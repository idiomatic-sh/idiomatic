//! Behavioral proof of `canonical_text`: the compact sorted-key text and the digest the python side pins.
use super::service::{digest, render};
use super::types::{Fragment, Label, Quantity};
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::convert::Infallible;

/// Which answer of the document a row asks for.
#[derive(Clone, Copy, Debug)]
enum Question {
    Text,
    Digest,
}

/// A document and the question asked of it.
struct Request {
    question: Question,
    node: Fragment,
}

type Input = Request;
type Value = Cow<'static, str>;
type Failure = Infallible;

fn text(value: &str) -> Fragment {
    Fragment::Text(Label(value.into()))
}

fn object(entries: Vec<(&str, Fragment)>) -> Fragment {
    Fragment::Object(
        entries
            .into_iter()
            .map(|(name, value)| (Label(name.into()), value))
            .collect::<BTreeMap<_, _>>(),
    )
}

/// The pinned cross-language document, read as the python side wrote it.
fn pinned(json: &serde_json::Value) -> Fragment {
    match json {
        serde_json::Value::Null => Fragment::Null,
        serde_json::Value::Number(number) => Fragment::Number(Quantity(number.as_i64().unwrap())),
        serde_json::Value::String(value) => text(value),
        serde_json::Value::Array(items) => Fragment::List(items.iter().map(pinned).collect()),
        serde_json::Value::Object(entries) => Fragment::Object(
            entries
                .iter()
                .map(|(name, value)| (Label(name.clone()), pinned(value)))
                .collect(),
        ),
        serde_json::Value::Bool(_) => unreachable!("no canonical document holds a flag"),
    }
}

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/reference-discrepancy-canonical.json"
    ))
    .unwrap()
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("nothing renders as null"),
        word: Word::Succeeds,
        input: || Request {
            question: Question::Text,
            node: Fragment::Null,
        },
        expect: Expect::Value(Cow::Borrowed("null")),
    },
    Case {
        name: CaseName("a number renders without padding"),
        word: Word::Succeeds,
        input: || Request {
            question: Question::Text,
            node: Fragment::Number(Quantity(-12)),
        },
        expect: Expect::Value(Cow::Borrowed("-12")),
    },
    Case {
        name: CaseName("quote, backslash, newline and control characters are escaped"),
        word: Word::Succeeds,
        input: || Request {
            question: Question::Text,
            node: text("a\"b\\c\nd\u{1}"),
        },
        expect: Expect::Value(Cow::Borrowed("\"a\\\"b\\\\c\\nd\\u0001\"")),
    },
    Case {
        name: CaseName("characters beyond ASCII stay as written"),
        word: Word::Succeeds,
        input: || Request {
            question: Question::Text,
            node: text("雪/é"),
        },
        expect: Expect::Value(Cow::Borrowed("\"雪/é\"")),
    },
    Case {
        name: CaseName("object keys render in name order and lists in order"),
        word: Word::Succeeds,
        input: || Request {
            question: Question::Text,
            node: object(vec![
                (
                    "b",
                    Fragment::List(vec![Fragment::Number(Quantity(2)), Fragment::Null]),
                ),
                ("a", Fragment::List(Vec::new())),
            ]),
        },
        expect: Expect::Value(Cow::Borrowed("{\"a\":[],\"b\":[2,null]}")),
    },
    Case {
        name: CaseName("the pinned cross-language document renders as the python side wrote it"),
        word: Word::Succeeds,
        input: || Request {
            question: Question::Text,
            node: pinned(&fixture()["output"]),
        },
        expect: Expect::Value(Cow::Borrowed(
            "{\"diagnostics\":[{\"end\":7,\"file\":\"雪/é.py\",\"rule\":\"F821\",\"start\":2},{\"end\":7,\"file\":\"雪/é.py\",\"rule\":\"F821\",\"start\":2}],\"files\":[\"z.py\",\"雪/é.py\"],\"skipped\":[]}",
        )),
    },
    Case {
        name: CaseName("the pinned cross-language document digests as the python side pinned"),
        word: Word::Succeeds,
        input: || Request {
            question: Question::Digest,
            node: pinned(&fixture()["output"]),
        },
        expect: Expect::Value(Cow::Borrowed(
            "fbf701690e0fda3c89da930b0fa5cc87c9a20f34684dfacee7ff3305d4a58e2b",
        )),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(match input.question {
        Question::Text => Cow::Owned(render(&input.node).0),
        Question::Digest => Cow::Owned(digest(&input.node).0),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
