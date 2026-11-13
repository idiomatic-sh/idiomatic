//! What the schemas of this adapter accept and refuse, one row of `CASES` for each input: rule codes, parse errors, selection
//! and byte spans of documents oxlint 1.76.0 printed.
#![cfg(test)]
use super::schemas::{decode, rule_of_code, selector_of_rule};
use ryo::linter::{RelativePath, RuleId};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ProcessExitCode, ProcessOutput, ResourceObservation, TerminationSignal,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use std::borrow::Cow;
use std::collections::BTreeSet;

/// `const é = "ü"; debugger;` linted with `-D eslint/no-debugger`.
const NON_ASCII: &str = r#"{ "diagnostics": [{"message": "`debugger` statement is not allowed","code": "eslint(no-debugger)","severity": "error","causes": [],"url": "https://oxc.rs/docs/guide/usage/linter/rules/eslint/no-debugger.html","help": "Remove the debugger statement","filename": "case.ts","labels": [{"span": {"offset": 17,"length": 9,"line": 1,"column": 18}}],"related": []}],
              "number_of_files": 1,
              "number_of_rules": 1,
              "threads_count": 1,
              "start_time": 0.231916858
            }"#;
/// `let x = ;` linted with `-D eslint/no-debugger`: the parse error has no code.
const PARSE_ERROR: &str = r#"{ "diagnostics": [{"message": "Unexpected token","severity": "error","causes": [],"filename": "broken.ts","labels": [{"span": {"offset": 8,"length": 1,"line": 1,"column": 9}}],"related": []}],
              "number_of_files": 1,
              "number_of_rules": 1,
              "threads_count": 1,
              "start_time": 0.204518515
            }"#;
/// What oxlint prints on stdout, before its JSON, when none of the named files exists.
const NO_FILES: &str = "No files found to lint. Please check your paths and ignore patterns.";

/// One wire input: an oxlint code, or a document decoded for one file and a comma list of selected rules.
#[derive(Clone, Copy, Debug)]
enum Wire {
    /// A diagnostic's `code`.
    Code(&'static str),
    /// The file the run was given, the rules it selected and what it printed, with exit 1.
    Document(&'static str, &'static str, &'static str),
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
            "rule_code_keeps_plugin: an eslint code keeps its plugin in the id and the selector",
        ),
        word: Word::Succeeds,
        input: || Wire::Code("eslint(no-debugger)"),
        expect: Expect::Value(Cow::Borrowed("eslint.no-debugger eslint/no-debugger")),
    },
    Case {
        name: CaseName(
            "rule_code_keeps_plugin: a typescript code keeps its plugin in the id and the selector",
        ),
        word: Word::Succeeds,
        input: || Wire::Code("typescript(no-this-alias)"),
        expect: Expect::Value(Cow::Borrowed(
            "typescript.no-this-alias typescript/no-this-alias",
        )),
    },
    Case {
        name: CaseName("rule_code_keeps_plugin: a code without its plugin is refused"),
        word: Word::Refuses,
        input: || Wire::Code("no-debugger"),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("codeless_diagnostic_is_invalid_syntax"),
        word: Word::Succeeds,
        input: || Wire::Document("broken.ts", "eslint.no-debugger", PARSE_ERROR),
        expect: Expect::Value(Cow::Borrowed("invalid-syntax broken.ts 8..9")),
    },
    Case {
        name: CaseName("unselected_code_is_refused"),
        word: Word::Refuses,
        input: || Wire::Document("case.ts", "typescript.no-this-alias", NON_ASCII),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("non_ascii_offsets_are_bytes: the span after é and ü is UTF-8 bytes 17..26"),
        word: Word::Succeeds,
        input: || Wire::Document("case.ts", "eslint.no-debugger", NON_ASCII),
        expect: Expect::Value(Cow::Borrowed("eslint.no-debugger case.ts 17..26")),
    },
    Case {
        name: CaseName("a finding in a file the run was not given is refused"),
        word: Word::Refuses,
        input: || Wire::Document("other.ts", "eslint.no-debugger", NON_ASCII),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("a run that enabled fewer rules than it selected is refused"),
        word: Word::Refuses,
        input: || Wire::Document("case.ts", "eslint.no-debugger,eslint.no-caller", NON_ASCII),
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("a document that is not oxlint's JSON is not a finding"),
        word: Word::Refuses,
        input: || Wire::Document("case.ts", "eslint.no-debugger", NO_FILES),
        expect: Expect::Error(Refusal::Unexpected),
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

/// Converts one wire input through the schemas: a code into its id and selector, a document into `rule file start..end` lines.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Wire::Code(code) => {
            let rule = rule_of_code(code).map_err(|error| refusal(&error))?;
            let selector = selector_of_rule(&rule).map_err(|error| refusal(&error))?;
            Ok(format!("{} {selector}", rule.as_ref()).into())
        }
        Wire::Document(file, rules, stdout) => {
            let rules = rules
                .split(',')
                .map(rule_id)
                .collect::<Result<Vec<_>, _>>()?;
            let selected: BTreeSet<_> = rules.iter().cloned().collect();
            let output = ProcessOutput {
                signal: TerminationSignal::Exited,
                status: ProcessExitCode(1),
                stdout: CapturedBytes(stdout.as_bytes().to_vec()),
                stderr: CapturedBytes(Vec::new()),
                wall: Default::default(),
                resources: ResourceObservation::WallOnly,
            };
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
