//! The line keeps the exact spelling it was given, and a tool's default line is its name and version.
use super::super::{FileObservation, RuleContextProfile, Tool};
use super::value::VersionLine;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::context::types::ObservationSelectionPolicy;
use crate::domain::requests::types::{ParserSelection, ProviderSelection, SearchRoots};
use std::borrow::Cow;

/// The line given, or the tool whose default line is taken.
#[derive(Clone, Copy, Debug)]
enum Input {
    Given(&'static str),
    Tool(&'static str, &'static str),
}

const CASES: &[Case<Input, Cow<'static, str>, ()>] = &[
    Case {
        name: CaseName("display is the exact line"),
        word: Word::Succeeds,
        input: || Input::Given("Version: 1.76.0"),
        expect: Expect::Value(Cow::Borrowed("Version: 1.76.0")),
    },
    Case {
        name: CaseName("a tool's default line is its name, a space and its version"),
        word: Word::Succeeds,
        input: || Input::Tool("ruff", "0.16.5"),
        expect: Expect::Value(Cow::Borrowed("ruff 0.16.5")),
    },
];

fn subject(input: Input) -> Result<Cow<'static, str>, ()> {
    match input {
        Input::Given(line) => Ok(VersionLine(line.into()).to_string().into()),
        Input::Tool(name, version) => {
            let tool = Tool {
                rule_observation_selection: ObservationSelectionPolicy::Declared,
                supporting_rules: Vec::new(),
                rule_observations: Default::default(),
                rule_context: RuleContextProfile::Undeclared,
                file_observation: FileObservation::Undeclared,
                project_context_mode: Default::default(),
                tool: name.into(),
                version: version.into(),
                sha256: Vec::new(),
                defaults: Vec::new(),
                deferred: Default::default(),
                pending: Default::default(),
                query_modules: Default::default(),
                rule_applicability: Default::default(),
                parser_configuration: ParserSelection::Unspecified,
                source_language: ProviderSelection::Unspecified,
                project_search_roots: SearchRoots::Undeclared,
            };
            Ok(VersionLine::from(&tool).to_string().into())
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
