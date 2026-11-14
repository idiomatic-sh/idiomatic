//! What the schemas of this adapter accept and refuse, one row of `CASES` for each input: rule codes, the message streams
//! clippy 0.1.100 printed for case crates (each line cut to the fields the schemas read), the sources a dependency file names,
//! and the manifest a case crate is given.
#![cfg(test)]
use super::schemas::{
    case_manifest, check_status, decode, dep_info_sources, rule_of_code, selector_of_rule,
};
use ryo::linter::{RelativePath, RuleId};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ProcessExitCode, ProcessOutput, ResourceObservation, TerminationSignal,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use std::borrow::Cow;
use std::collections::BTreeSet;

/// `let é = 0x1aB;` and an unused `let unused = 1;` in a function of `src/lib.rs`, checked with `-W clippy::mixed_case_hex_literals`:
/// the library build and its test build each print the clippy finding and rustc's `unused_variables`.
const NON_ASCII: &str = r#"{"reason":"compiler-message","message":{"message":"inconsistent casing in hexadecimal literal","code":{"code":"clippy::mixed_case_hex_literals"},"level":"warning","spans":[{"file_name":"src/lib.rs","byte_start":33,"byte_end":38,"is_primary":true}]}}
{"reason":"compiler-message","message":{"message":"unused variable: `unused`","code":{"code":"unused_variables"},"level":"warning","spans":[{"file_name":"src/lib.rs","byte_start":48,"byte_end":54,"is_primary":true}]}}
{"reason":"compiler-message","message":{"message":"inconsistent casing in hexadecimal literal","code":{"code":"clippy::mixed_case_hex_literals"},"level":"warning","spans":[{"file_name":"src/lib.rs","byte_start":33,"byte_end":38,"is_primary":true}]}}
{"reason":"compiler-message","message":{"message":"unused variable: `unused`","code":{"code":"unused_variables"},"level":"warning","spans":[{"file_name":"src/lib.rs","byte_start":48,"byte_end":54,"is_primary":true}]}}
{"reason":"build-finished","success":true}"#;
/// `pub fn f() { let x = ; }`: the parse error has no code and stops the build.
const PARSE_ERROR: &str = r#"{"reason":"compiler-message","message":{"message":"expected expression, found `;`","code":null,"level":"error","spans":[{"file_name":"src/lib.rs","byte_start":21,"byte_end":22,"is_primary":true}]}}
{"reason":"build-finished","success":false}"#;
/// `pub fn f() -> u8 { "" }`: a type error, coded, then the note that names its explanation.
const TYPE_ERROR: &str = r#"{"reason":"compiler-message","message":{"message":"mismatched types","code":{"code":"E0308"},"level":"error","spans":[{"file_name":"src/lib.rs","byte_start":19,"byte_end":21,"is_primary":true},{"file_name":"src/lib.rs","byte_start":14,"byte_end":16,"is_primary":false}]}}
{"reason":"compiler-message","message":{"message":"For more information about this error, try `rustc --explain E0308`.","code":null,"level":"failure-note","spans":[]}}
{"reason":"build-finished","success":false}"#;
/// `#![warn(clippy::print_stdout)]` above a `println!` and `0x1aB`, checked with `-W clippy::mixed_case_hex_literals`: the source
/// attribute turns `print_stdout` on, and no command-line flag outranks it.
const ATTRIBUTE_ENABLED: &str = r#"{"reason":"compiler-message","message":{"message":"inconsistent casing in hexadecimal literal","code":{"code":"clippy::mixed_case_hex_literals"},"level":"warning","spans":[{"file_name":"src/lib.rs","byte_start":74,"byte_end":79,"is_primary":true}]}}
{"reason":"compiler-message","message":{"message":"use of `println!`","code":{"code":"clippy::print_stdout"},"level":"warning","spans":[{"file_name":"src/lib.rs","byte_start":55,"byte_end":68,"is_primary":true}]}}
{"reason":"compiler-message","message":{"message":"inconsistent casing in hexadecimal literal","code":{"code":"clippy::mixed_case_hex_literals"},"level":"warning","spans":[{"file_name":"src/lib.rs","byte_start":74,"byte_end":79,"is_primary":true}]}}
{"reason":"compiler-message","message":{"message":"use of `println!`","code":{"code":"clippy::print_stdout"},"level":"warning","spans":[{"file_name":"src/lib.rs","byte_start":55,"byte_end":68,"is_primary":true}]}}
{"reason":"build-finished","success":true}"#;
/// `pub const X: u32 = 0123;` checked with `-W clippy::zero_prefixd_literal` beside `-W clippy::zero_prefixed_literal`: rustc
/// names the misspelled selection under `E0602`, with no span.
const MISSPELLED_SELECTION: &str = r#"{"reason":"compiler-message","message":{"message":"unknown lint: `clippy::zero_prefixd_literal`","code":{"code":"E0602"},"level":"warning","spans":[]}}
{"reason":"compiler-message","message":{"message":"this is a decimal constant","code":{"code":"clippy::zero_prefixed_literal"},"level":"warning","spans":[{"file_name":"src/lib.rs","byte_start":19,"byte_end":23,"is_primary":true}]}}
{"reason":"compiler-message","message":{"message":"For more information about this error, try `rustc --explain E0602`.","code":null,"level":"failure-note","spans":[]}}
{"reason":"build-finished","success":true}"#;
/// `#![allow(clippy::not_a_lint)]` in `src/lib.rs`: a source attribute naming a lint clippy does not have, under `unknown_lints`.
const UNKNOWN_ATTRIBUTE_LINT: &str = r#"{"reason":"compiler-message","message":{"message":"unknown lint: `clippy::not_a_lint`","code":{"code":"unknown_lints"},"level":"warning","spans":[{"file_name":"src/lib.rs","byte_start":9,"byte_end":27,"is_primary":true}]}}
{"reason":"compiler-message","message":{"message":"unknown lint: `clippy::not_a_lint`","code":{"code":"unknown_lints"},"level":"warning","spans":[{"file_name":"src/lib.rs","byte_start":9,"byte_end":27,"is_primary":true}]}}
{"reason":"build-finished","success":true}"#;
/// The end of a build that failed with no compiler message: cargo stopped it before rustc ran.
const FAILED_WITHOUT_ERROR: &str = r#"{"reason":"build-finished","success":false}"#;
/// The dependency file rustc wrote for a workspace member, its sources relative to the workspace root, beside a registry
/// source and a non-Rust prerequisite.
const RELATIVE_DEP_INFO: &str = "/t/debug/build/member/1/out/member-1.d: crates/member/src/lib.rs crates/member/src/a.rs /usr/local/cargo/registry/src/index.crates.io-1/cfg-if-1.0.5/src/lib.rs crates/member/README.md\n\n/t/debug/build/member/1/out/libmember-1.rmeta: crates/member/src/lib.rs crates/member/src/a.rs\n";
/// The dependency file of a build script named by absolute paths, one under the root `/r` and one outside it.
const ABSOLUTE_DEP_INFO: &str =
    "/t/debug/build/member/2/out/build_script_build-2.d: /r/build.rs /elsewhere/build.rs\n";
/// A source whose directory name holds a space, which a dependency file writes escaped.
const ESCAPED_DEP_INFO: &str =
    "/t/debug/build/member/3/out/member-3.d: src/lib.rs src/with\\ space/mod.rs\n";

/// One wire input: a clippy code, a message stream, a dependency file, or the case manifest.
#[derive(Clone, Copy, Debug)]
enum Wire {
    /// A diagnostic's `code`.
    Code(&'static str),
    /// The file the run was given, the comma list of rules it selected, its exit status and what it printed.
    Stream(&'static str, &'static str, i32, &'static str),
    /// A dependency file read under the root `/r`.
    DepInfo(&'static str),
    /// The manifest written into a case root.
    CaseManifest,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Invalid,
    Unexpected,
}
type Input = Wire;
type Value = Cow<'static, str>;
type Failure = Refusal;

/// The rows of this adapter's schema cases: a name, one case word, a wire input, and its rendering or refusal.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "rule_code_drops_the_tool: a clippy code is its lint, and the lint its selector",
        ),
        word: Word::Succeeds,
        input: || Wire::Code("clippy::needless_return"),
        expect: Expect::Value(Cow::Borrowed("needless_return clippy::needless_return")),
    },
    Case {
        name: CaseName("rule_code_drops_the_tool: a rustc lint is not a clippy code"),
        word: Word::Refuses,
        input: || Wire::Code("unused_variables"),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("rule_code_drops_the_tool: the tool alone names no lint"),
        word: Word::Refuses,
        input: || Wire::Code("clippy::"),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName(
            "rustc_warning_is_skipped, lib_and_test_build_are_one_finding and non_ascii_offsets_are_bytes: one finding, after é, in UTF-8 bytes",
        ),
        word: Word::Succeeds,
        input: || Wire::Stream("src/lib.rs", "mixed_case_hex_literals", 0, NON_ASCII),
        expect: Expect::Value(Cow::Borrowed("mixed_case_hex_literals src/lib.rs 33..38")),
    },
    Case {
        name: CaseName(
            "unselected_clippy_code_is_skipped: a lint a source attribute turns on is outside the comparison",
        ),
        word: Word::Succeeds,
        input: || {
            Wire::Stream(
                "src/lib.rs",
                "mixed_case_hex_literals,zero_prefixed_literal",
                0,
                ATTRIBUTE_ENABLED,
            )
        },
        expect: Expect::Value(Cow::Borrowed("mixed_case_hex_literals src/lib.rs 74..79")),
    },
    Case {
        name: CaseName("codeless_error_is_invalid_syntax"),
        word: Word::Succeeds,
        input: || Wire::Stream("src/lib.rs", "mixed_case_hex_literals", 101, PARSE_ERROR),
        expect: Expect::Value(Cow::Borrowed("invalid-syntax src/lib.rs 21..22")),
    },
    Case {
        name: CaseName(
            "coded_error_does_not_compile: a type error refuses the run, clippy's late lints never ran",
        ),
        word: Word::Refuses,
        input: || Wire::Stream("src/lib.rs", "mixed_case_hex_literals", 101, TYPE_ERROR),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("unknown_lint_refuses_the_run: a misspelled selection, E0602"),
        word: Word::Refuses,
        input: || {
            Wire::Stream(
                "src/lib.rs",
                "zero_prefixd_literal,zero_prefixed_literal",
                0,
                MISSPELLED_SELECTION,
            )
        },
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("unknown_lint_refuses_the_run: an unknown lint a source attribute names"),
        word: Word::Refuses,
        input: || {
            Wire::Stream(
                "src/lib.rs",
                "zero_prefixed_literal",
                0,
                UNKNOWN_ATTRIBUTE_LINT,
            )
        },
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName(
            "unrequested_file_is_refused: the finding is in a file the run was not given",
        ),
        word: Word::Refuses,
        input: || Wire::Stream("src/a.rs", "mixed_case_hex_literals", 0, NON_ASCII),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName(
            "failed_build_without_error_is_refused: exit 101 with no compiler error is not a clean run",
        ),
        word: Word::Refuses,
        input: || {
            Wire::Stream(
                "src/lib.rs",
                "mixed_case_hex_literals",
                101,
                FAILED_WITHOUT_ERROR,
            )
        },
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("an exit other than 0 or 101 is an operational failure"),
        word: Word::Refuses,
        input: || Wire::Stream("src/lib.rs", "mixed_case_hex_literals", 1, NON_ASCII),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName(
            "dep_info_keeps_root_sources: a member's relative sources, never a registry source or README.md",
        ),
        word: Word::Succeeds,
        input: || Wire::DepInfo(RELATIVE_DEP_INFO),
        expect: Expect::Value(Cow::Borrowed(
            "crates/member/src/a.rs\ncrates/member/src/lib.rs",
        )),
    },
    Case {
        name: CaseName(
            "dep_info_keeps_root_sources: an absolute source under the root, never one outside it",
        ),
        word: Word::Succeeds,
        input: || Wire::DepInfo(ABSOLUTE_DEP_INFO),
        expect: Expect::Value(Cow::Borrowed("build.rs")),
    },
    Case {
        name: CaseName("dep_info_keeps_root_sources: an escaped space is part of its path"),
        word: Word::Succeeds,
        input: || Wire::DepInfo(ESCAPED_DEP_INFO),
        expect: Expect::Value(Cow::Borrowed("src/lib.rs\nsrc/with space/mod.rs")),
    },
    Case {
        name: CaseName("a dependency file without a rule is refused"),
        word: Word::Refuses,
        input: || Wire::DepInfo("src/lib.rs\n"),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName(
            "case_manifest_is_its_own_workspace: the edition-2024 case package and an empty workspace table",
        ),
        word: Word::Succeeds,
        input: || Wire::CaseManifest,
        expect: Expect::Value(Cow::Borrowed(
            "[package]\nname = \"case\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n",
        )),
    },
];

fn refusal(error: &Error) -> Refusal {
    match error {
        Error::Invalid(_) => Refusal::Invalid,
        Error::Io(_) | Error::Json(_) | Error::Toml(_) => Refusal::Unexpected,
    }
}
fn rule_id(text: &str) -> Result<RuleId, Failure> {
    RuleId::try_from(text).map_err(|_blank| Refusal::Invalid)
}
/// A request of one file that selects the listed rules.
fn request(file: &str, rules: &[RuleId]) -> RunRequest {
    RunRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(
            rules.to_vec(),
        ),
        rule_observations:
            ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context:
            ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots:
            ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Unlisted,
        file_metadata:
            ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: "root".into(),
        state: "state".into(),
        files: vec![RelativePath(file.into())],
    }
}

/// Converts one wire input through the schemas: a code into its id and selector, a stream into `rule file start..end` lines,
/// a dependency file into its sources, one per line.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Wire::Code(code) => {
            let rule = rule_of_code(code).map_err(|error| refusal(&error))?;
            let selector = selector_of_rule(&rule).map_err(|error| refusal(&error))?;
            Ok(format!("{} {selector}", rule.as_ref()).into())
        }
        Wire::DepInfo(text) => Ok(dep_info_sources(text, std::path::Path::new("/r"))
            .map_err(|error| refusal(&error))?
            .iter()
            .map(|path| path.0.to_string())
            .collect::<Vec<_>>()
            .join("\n")
            .into()),
        Wire::CaseManifest => Ok(case_manifest().map_err(|error| refusal(&error))?.into()),
        Wire::Stream(file, rules, status, stdout) => {
            let rules = rules
                .split(',')
                .map(rule_id)
                .collect::<Result<Vec<_>, _>>()?;
            let selected: BTreeSet<_> = rules.iter().cloned().collect();
            let output = ProcessOutput {
                signal: TerminationSignal::Exited,
                status: ProcessExitCode(status),
                stdout: CapturedBytes(stdout.as_bytes().to_vec()),
                stderr: CapturedBytes(Vec::new()),
                wall: Default::default(),
                resources: ResourceObservation::WallOnly,
            };
            check_status(&output).map_err(|error| refusal(&error))?;
            let report = decode(&request(file, &rules), &output, &selected)
                .map_err(|error| refusal(&error))?;
            Ok(report
                .diagnostics
                .iter()
                .map(|finding| {
                    format!(
                        "{} {} {}..{}",
                        finding.rule.as_ref(),
                        finding.file.0,
                        finding.start.0,
                        finding.end.0
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
                .into())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
