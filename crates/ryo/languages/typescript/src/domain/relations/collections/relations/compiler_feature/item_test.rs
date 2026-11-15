//! Cases for TypeScript `compiler_feature` rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::CompilerFeatureItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading_at;
use ryo::languages::contract::{Reading, want_file, want_flag, want_name};
type Input = (&'static str, &'static str);
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_typescript_file_has_typescript_syntax"),
        word: Word::Succeeds,
        input: || ("x.ts", "let a = 1;\n"),
        expect: Expect::Value(Reading {
            rows: &[&[
                want_file("x.ts"),
                want_name("typescript.syntax"),
                want_flag(true),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_javascript_file_lacks_typescript_syntax"),
        word: Word::Succeeds,
        input: || ("x.js", "let a = 1;\n"),
        expect: Expect::Value(Reading {
            rows: &[&[
                want_file("x.js"),
                want_name("typescript.syntax"),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
];
fn subject((path, source): Input) -> Value {
    check::<CompilerFeatureItem>();
    reading_at::<CompilerFeatureItem>(path, source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
