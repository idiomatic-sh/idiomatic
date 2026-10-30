//! The `language` service's cases.

use super::service::{dialect, reads};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::types::Dialect;
use ryo::languages;

/// A path.
type Input = &'static str;
/// Whether the language reads the file, and how.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_json_file_is_read"),
        word: Word::Succeeds,
        input: || "src/main.json",
        expect: Expect::Value("strict"),
    },
    Case {
        name: CaseName("a_jsonc_file_is_json_with_comments"),
        word: Word::Succeeds,
        input: || "settings.jsonc",
        expect: Expect::Value("jsonc"),
    },
    Case {
        name: CaseName("a_tsconfig_is_json_with_comments"),
        word: Word::Succeeds,
        input: || "packages/app/tsconfig.json",
        expect: Expect::Value("jsonc"),
    },
    Case {
        name: CaseName("a_jsconfig_is_json_with_comments"),
        word: Word::Succeeds,
        input: || "jsconfig.json",
        expect: Expect::Value("jsonc"),
    },
    Case {
        name: CaseName("a_configuration_named_like_a_tsconfig_is_json_with_comments"),
        word: Word::Succeeds,
        input: || "tsconfig.build.json",
        expect: Expect::Value("jsonc"),
    },
    Case {
        name: CaseName("a_babelrc_is_json_with_comments"),
        word: Word::Succeeds,
        input: || "app/.babelrc",
        expect: Expect::Value("jsonc"),
    },
    Case {
        name: CaseName("an_eslintrc_is_json_with_comments"),
        word: Word::Succeeds,
        input: || ".eslintrc",
        expect: Expect::Value("jsonc"),
    },
    Case {
        name: CaseName("a_swcrc_is_json_with_comments"),
        word: Word::Succeeds,
        input: || ".swcrc",
        expect: Expect::Value("jsonc"),
    },
    Case {
        name: CaseName("a_code_workspace_is_json_with_comments"),
        word: Word::Succeeds,
        input: || "project.code-workspace",
        expect: Expect::Value("jsonc"),
    },
    Case {
        name: CaseName("a_web_manifest_is_strict_json"),
        word: Word::Succeeds,
        input: || "public/site.webmanifest",
        expect: Expect::Value("strict"),
    },
    Case {
        name: CaseName("a_name_that_only_ends_like_a_dotfile_is_declined"),
        word: Word::Succeeds,
        input: || "my.babelrc.bak",
        expect: Expect::Value("declined"),
    },
    Case {
        name: CaseName("a_package_manifest_is_strict_json"),
        word: Word::Succeeds,
        input: || "package.json",
        expect: Expect::Value("strict"),
    },
    Case {
        name: CaseName("a_name_that_only_ends_like_a_tsconfig_is_strict_json"),
        word: Word::Succeeds,
        input: || "my-tsconfig.json",
        expect: Expect::Value("strict"),
    },
    Case {
        name: CaseName("a_file_of_another_language_is_declined"),
        word: Word::Succeeds,
        input: || "README.md",
        expect: Expect::Value("declined"),
    },
];

fn subject(path: Input) -> Value {
    let path = languages::SourcePath(path);
    match (reads(path), dialect(path)) {
        (languages::Recognition::Declines, Dialect::Strict | Dialect::Jsonc) => "declined",
        (languages::Recognition::Reads, Dialect::Strict) => "strict",
        (languages::Recognition::Reads, Dialect::Jsonc) => "jsonc",
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
