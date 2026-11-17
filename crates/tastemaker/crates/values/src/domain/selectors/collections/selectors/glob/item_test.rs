//! The glob kind spells a pattern into the glob and the expression the matching port compiles.

use super::item::{interpolated, spell};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::captures::types::{CaptureName, CaptureValue, Interpolations};
use crate::domain::selectors::types::{Binding, PathPattern};

/// A pattern and the values interpolated into it.
type Input = (&'static str, &'static [(&'static str, &'static str)]);
/// The glob, the expression and the literal prefix, joined by `|`.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_literal_pattern_needs_no_expression"),
        word: Word::Succeeds,
        input: || ("src/lib.txt", &[]),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_capture_becomes_a_group"),
        word: Word::Succeeds,
        input: || ("src/{domain}.txt", &[]),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_interpolated_name_is_literal"),
        word: Word::Succeeds,
        input: || ("src/{domain}.txt", &[("domain", "greeting")]),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_double_star_ends_the_literal_prefix"),
        word: Word::Succeeds,
        input: || ("crates/**/lib.txt", &[]),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_unclosed_brace_is_spelled_as_written"),
        word: Word::Refuses,
        input: || ("src/{domain", &[]),
        expect: Expect::error(()),
    },
];

fn expected(pattern: &str, values: usize) -> &'static str {
    match (pattern, values) {
        ("src/lib.txt", _) => "src/lib.txt|-|src,lib.txt",
        ("src/{domain}.txt", 0) => "src/*.txt|^src/([^/]+)\\.txt$|src",
        ("src/{domain}.txt", _) => "src/greeting.txt|-|src,greeting.txt",
        ("crates/**/lib.txt", _) => "crates/**/lib.txt|-|crates",
        _ => "",
    }
}

fn subject((pattern, given): Input) -> Result<Value, Failure> {
    let mut values = Interpolations::default();
    for (name, value) in given {
        drop(
            values
                .0
                .insert(CaptureName((*name).into()), CaptureValue((*value).into())),
        );
    }
    let values = interpolated(&PathPattern(pattern.into()), &values);
    let super::item::Spelled { spelling, prefix } = spell(&PathPattern(pattern.into()), &values);
    let expression = match &spelling.binder {
        Binding::GlobOnly => "-".to_owned(),
        Binding::Expression(text) => text.to_string(),
    };
    let answer = format!(
        "{}|{expression}|{}",
        spelling.glob,
        prefix
            .iter()
            .map(|segment| &*segment.0)
            .collect::<Vec<_>>()
            .join(",")
    );
    if pattern.contains('{') && !pattern.contains('}') {
        return if answer.contains("[{]") {
            Err(())
        } else {
            Ok(answer)
        };
    }
    Ok(if answer == expected(pattern, given.len()) {
        String::new()
    } else {
        answer
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
