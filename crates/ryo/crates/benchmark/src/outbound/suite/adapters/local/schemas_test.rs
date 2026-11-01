//! Independent wire examples prove typed defaults and duplicate-disposition refusal.
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = &'static str;
type Value = bool;
type Failure = &'static str;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("profile preserves exact selected rules"),
        word: Word::Succeeds,
        input: || "tool='ruff'\nversion='1'\nsha256=['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb']\ndefaults=['F401']",
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
    Case {
        name: CaseName("query modules retain logical IDs and safe relative paths"),
        word: Word::Succeeds,
        input: || "tool='ruff'\nversion='1'\nsha256=['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb']\ndefaults=['F401']\n[query-modules]\n'python/ruff/suppress'='fragments/suppress.ryoql'",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("rule applicability maps are typed and restricted to selected defaults"),
        word: Word::Succeeds,
        input: || "tool='ruff'\nversion='1'\nsha256=['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb']\ndefaults=['F401']\n[rule-applicability]\nF401='accepted-syntax'",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("rule applicability rejects unknown policies and rules"),
        word: Word::Refuses,
        input: || "tool='ruff'\nversion='1'\nsha256=['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa']\ndefaults=['F401']\n[rule-applicability]\nF402='maybe'",
        expect: Expect::Error("invalid applicability"),
    },
    Case {
        name: CaseName("query module paths cannot leave the profile root"),
        word: Word::Refuses,
        input: || "tool='ruff'\nversion='1'\nsha256=['abc']\ndefaults=['F401']\n[query-modules]\n'suppress'='../outside.ryoql'",
        expect: Expect::Error("unsafe module path"),
    },
    Case {
        name: CaseName("full report accepts explicit frontend identity"),
        word: Word::Succeeds,
        input: || "[[cases]]\nname='parser'\ncomparison='full'\nfiles={'case.py'='x'}\nexpected=[{rule='invalid-syntax',file='case.py',start=0,end=1}]",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("full report refuses inherited rule identity"),
        word: Word::Refuses,
        input: || "[[cases]]\nname='parser'\ncomparison='full'\nfiles={'case.py'='x'}\nexpected=[{file='case.py',start=0,end=1}]",
        expect: Expect::Error("invalid report envelope"),
    },
    Case {
        name: CaseName("comparison scope rejects unknown modes"),
        word: Word::Refuses,
        input: || "[[cases]]\nname='parser'\ncomparison='partial'\nfiles={'case.py'='x'}\nexpected=[]",
        expect: Expect::Error("invalid report envelope"),
    },
];
fn subject(text: Input) -> Result<Value, Failure> {
    if text.starts_with("[[cases]]") {
        return report_subject(text);
    }
    let tool = super::tool(text).map_err(|error| {
        if error.to_string() == "rule appears in multiple capability groups" {
            "duplicate capability member"
        } else if error.to_string() == "executable pins must be nonempty and unique" {
            "invalid pins"
        } else if error.to_string().starts_with("unsafe corpus path") {
            "unsafe module path"
        } else if error
            .to_string()
            .starts_with("invalid rule applicability policy")
        {
            "invalid applicability"
        } else {
            "unexpected schema failure"
        }
    })?;
    Ok(tool.tool.0 == "ruff"
        && tool.version.0 == "1"
        && tool.sha256
            == vec![
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            ]
        && tool
            .defaults
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            == ["F401"]
        && tool.deferred.is_empty()
        && tool.pending.is_empty()
        && if text.contains("[query-modules]") {
            tool.query_modules
                .get("python/ruff/suppress")
                .is_some_and(|path| path == std::path::Path::new("fragments/suppress.ryoql"))
        } else {
            tool.query_modules.is_empty()
        }
        && if text.contains("[rule-applicability]") {
            tool.rule_applicability
                .get("F401")
                .is_some_and(|value| value == "accepted-syntax")
        } else {
            tool.rule_applicability.is_empty()
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

/// Decode and validate the same envelope used by the suite adapter.
fn report_subject(text: Input) -> Result<Value, Failure> {
    let cases = super::cases(text).map_err(|_refusal| "invalid report envelope")?;
    crate::domain::suite::service::validate_case_sources(&cases)
        .map_err(|_refusal| "invalid report envelope")?;
    Ok(
        matches!(cases.cases.first(), Some(case) if case.comparison == crate::domain::suite::types::CaseComparison::Full),
    )
}
