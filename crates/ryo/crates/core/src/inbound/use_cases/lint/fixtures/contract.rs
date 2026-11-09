//! What every implementation of the `lint` use case must pass, and what the whole per-file path must pass: `lint` over the corpus the
//! fixtures state, with the real store and real queries and the language the caller brings. Every relation item the language lists and
//! every operator item the product lists is read from its fixtures, so a new item is covered without anyone editing a test.

use super::super::errors::LintError;
use super::super::interface::LintUseCase;
use super::super::types::LintRequest;
use super::super::types::ParameterCell;
use super::super::types::ParameterTable;
use super::super::types::ParameterValue;
use super::super::types::Rule;
use super::super::types::RuleParameter;
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Matched;
use crate::cases::Word;
use crate::cases::run;
use crate::cases::selects;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_relations::domain::relations::service::name;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::ParameterName;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::runs::types::AnswerShape;
use ryo_values::domain::runs::types::Session;

/// What a row of the contract is given: the request to lint.
type Input = LintRequest;

/// What a row of the contract expects when it succeeds: how many files were left unread.
type Value = usize;

/// What a row of the contract expects when it is refused.
type Failure = LintError;

/// What the parameters a fixture query declares are bound to: the layers the functions of the `function` fixture are ranked by in the
/// fixture of `rank`, and the one name the fixture of `rows` holds.
fn fixture_parameters(query: &str) -> Vec<RuleParameter> {
    if query.contains("param names: list<table>;") {
        return vec![RuleParameter {
            name: ParameterName("names".into()),
            value: ParameterValue::Tables(vec![ParameterTable(vec![ParameterCell {
                key: LiteralText("name".into()),
                value: LiteralText("run".into()),
            }])]),
        }];
    }
    if !query.contains("param layers: list<set<string>>;") {
        return Vec::new();
    }
    let layer = |names: &[&str]| -> Vec<LiteralText> {
        names
            .iter()
            .map(|each| LiteralText((*each).into()))
            .collect()
    };
    vec![RuleParameter {
        name: ParameterName("layers".into()),
        value: ParameterValue::TextSets(vec![layer(&["run"]), layer(&["inner", "outer"])]),
    }]
}

/// A rule of this id and query over the files these globs name.
pub(super) fn rule(id: &str, query: &str, include: Vec<Glob>) -> Rule {
    Rule {
        observations: Vec::new(),
        id: ryo_values::domain::runs::types::RuleId(id.into()),
        query: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(query.into())),
        applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
        parameters: fixture_parameters(query),
        include,
        exclude: Vec::new(),
    }
}

/// A request to lint the repository at the working directory with rules of these ids, whose queries are never run.
pub(super) fn asking(ids: &[&str]) -> LintRequest {
    LintRequest {
        source_language: crate::inbound::use_cases::lint::types::SourceProvider::Recognized,
        answer: AnswerShape::Findings,
        hosted: Vec::new(),
        session: Session {
            search_scope: Default::default(),
            given_inventory: Default::default(),
            repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
            root: RootPath(".".into()),
        },
        files: Vec::new(),
        rules: ids
            .iter()
            .map(|id| rule(id, "a rule", Vec::new()))
            .collect(),
    }
}

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_rule_governs_no_file_so_none_is_left_unread"),
        word: Word::Succeeds,
        input: || asking(&[]),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_rule_without_an_id_is_refused"),
        word: Word::Refuses,
        input: || asking(&[" "]),
        expect: Expect::Error(LintError::BlankRuleId),
    },
    Case {
        name: CaseName("two_rules_with_one_id_are_refused"),
        word: Word::Refuses,
        input: || asking(&["same", "same"]),
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, LintError::DuplicateRuleId { .. })))
        }),
    },
];

/// Runs the contract's rows against one implementation, in the crate that implements the trait.
pub fn check(subject: &impl LintUseCase) {
    let lints = |input: Input| subject.lint(input).map(|report| report.unread.len());
    run(CASES, Word::Succeeds, lints);
    run(CASES, Word::Refuses, lints);
}
