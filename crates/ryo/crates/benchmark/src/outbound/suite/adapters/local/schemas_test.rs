//! Independent wire examples prove typed defaults and duplicate-disposition refusal.
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = &'static str;
type Value = bool;
type Failure = &'static str;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("profile preserves exact selected rules"),
        word: Word::Succeeds,
        input: || "tool='ruff'\nversion='1'\nsha256=['abc','def']\ndefaults=['F401']",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("overlapping capability groups cannot overwrite a disposition"),
        word: Word::Refuses,
        input: || "tool='ruff'\nversion='1'\nsha256=['abc']\ndefaults=['F401']\n[deferred]\none=['F401']\ntwo=['F401']",
        expect: Expect::Error("duplicate capability member"),
    },
    Case {
        name: CaseName("a profile without executable pins is refused"),
        word: Word::Refuses,
        input: || "tool='ruff'\nversion='1'\nsha256=[]\ndefaults=['F401']",
        expect: Expect::Error("invalid pins"),
    },
    Case {
        name: CaseName("duplicate executable pins are refused"),
        word: Word::Refuses,
        input: || "tool='ruff'\nversion='1'\nsha256=['abc','abc']\ndefaults=['F401']",
        expect: Expect::Error("invalid pins"),
    },
];
fn subject(text: Input) -> Result<Value, Failure> {
    let tool = super::tool(text).map_err(|error| {
        if error.to_string() == "rule appears in multiple capability groups" {
            "duplicate capability member"
        } else if error.to_string() == "executable pins must be nonempty and unique" {
            "invalid pins"
        } else {
            "unexpected schema failure"
        }
    })?;
    Ok(tool.tool.0 == "ruff"
        && tool.version.0 == "1"
        && tool.sha256 == vec!["abc".into(), "def".into()]
        && tool
            .defaults
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            == ["F401"]
        && tool.deferred.is_empty()
        && tool.pending.is_empty())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
