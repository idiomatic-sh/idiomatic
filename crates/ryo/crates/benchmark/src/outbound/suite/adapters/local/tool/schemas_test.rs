//! The accepted and refused profile documents.
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
        name: CaseName("rule applicability maps are typed and restricted to declared rules"),
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
        name: CaseName("explicit policies cover selected rules and supporting producers"),
        word: Word::Succeeds,
        input: || "tool='ruff'\nversion='1'\nsha256=['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb']\ndefaults=['F401']\nsupporting-rules=['E501']\n[rule-applicability]\nF401='accepted-syntax'\nE501='accepted-syntax'",
        expect: Expect::Value(true),
    },

    Case {
        name: CaseName("an explicit policy map cannot omit a supporting producer"),
        word: Word::Refuses,
        input: || "tool='ruff'\nversion='1'\nsha256=['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb']\ndefaults=['F401']\nsupporting-rules=['E501']\n[rule-applicability]\nF401='accepted-syntax'",
        expect: Expect::Error("invalid applicability"),
    },

    Case {
        name: CaseName("an explicit policy map cannot omit a selected rule"),
        word: Word::Refuses,
        input: || "tool='ruff'\nversion='1'\nsha256=['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb']\ndefaults=['F401']\nsupporting-rules=['E501']\n[rule-applicability]\nE501='accepted-syntax'",
        expect: Expect::Error("invalid applicability"),
    },

    Case {
        name: CaseName("query module paths cannot leave the profile root"),
        word: Word::Refuses,
        input: || "tool='ruff'\nversion='1'\nsha256=['abc']\ndefaults=['F401']\n[query-modules]\n'suppress'='../outside.ryoql'",
        expect: Expect::Error("unsafe module path"),
    },
    Case {
        name: CaseName("a profile without a context mode is isolated"),
        word: Word::Succeeds,
        input: || "tool='ruff'\nversion='1'\nsha256=['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb']\ndefaults=['F401']\n",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an explicit listed-disk context mode is read"),
        word: Word::Succeeds,
        input: || "tool='ruff'\nversion='1'\nsha256=['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb']\ndefaults=['F401']\nproject-context-mode='listed-disk'\n",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an unknown context mode is refused"),
        word: Word::Refuses,
        input: || "tool='ruff'\nversion='1'\nsha256=['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb']\ndefaults=['F401']\nproject-context-mode='implicit-disk'\n",
        expect: Expect::Error("unexpected schema failure"),
    },
    Case {
        name: CaseName("a declared empty project scope is kept distinct from an undeclared one"),
        word: Word::Succeeds,
        input: || "tool='ruff'\nversion='1'\nsha256=['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb']\ndefaults=['F401']\nproject-search-roots=[]\n",
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a project search root cannot leave the project"),
        word: Word::Refuses,
        input: || "tool='ruff'\nversion='1'\nsha256=['aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa','bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb']\ndefaults=['F401']\nproject-search-roots=['../outside']\n",
        expect: Expect::Error("unsafe module path"),
    },
];
/// Decodes one profile and checks the values the row names.
fn subject(text: Input) -> Result<Value, Failure> {
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
                .get(&ryo_benchmark_contracts::domain::requests::types::QueryModuleName("python/ruff/suppress".into()))
                .is_some_and(|path| path == std::path::Path::new("fragments/suppress.ryoql"))
        } else {
            tool.query_modules.is_empty()
        }
        && if text.contains("supporting-rules") {
            tool.supporting_rules
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                == ["E501"]
                && tool
                    .rule_applicability
                    .get(&ryo_benchmark_contracts::domain::requests::types::RuleLabel("E501".into()))
                    .map(|policy| policy.0.as_str())
                    == Some("accepted-syntax")
        } else {
            tool.supporting_rules.is_empty()
        }
        && if text.contains("project-context-mode='listed-disk'") {
            tool.project_context_mode
                == ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::ListedDisk
        } else {
            tool.project_context_mode
                == ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::Isolated
        }
        && if text.contains("project-search-roots=[]") {
            tool.project_search_roots == ryo_benchmark_contracts::domain::requests::types::SearchRoots::Declared(Vec::new())
        } else {
            tool.project_search_roots == ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared
        }
        && if text.contains("[rule-applicability]") {
            tool.rule_applicability
                .get(&ryo_benchmark_contracts::domain::requests::types::RuleLabel("F401".into()))
                .is_some_and(|value| value.0 == "accepted-syntax")
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
