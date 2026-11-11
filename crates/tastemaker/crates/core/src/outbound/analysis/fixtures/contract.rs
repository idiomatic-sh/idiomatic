//! What every `analysis` adapter must pass: a blank query is refused, a lint of no rules answers nothing, a lint of two
//! rules of one id is refused whole, and every listed relation and operator has a page.

use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::applications::types::{GlobText, LintRuleId, PlannedRule};
use crate::domain::catalog::types::{CatalogName, OperatorAnswer, RelationAnswer};
use crate::domain::definitions::types::QueryText;
use crate::domain::files::types::MachinePath;
use crate::outbound::analysis::errors::AnalysisError;
use crate::outbound::analysis::interface::AnalysisPort;
use crate::outbound::analysis::types::{AnalysisRequest, Corpus, Hosted, QueryVerdict};

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// A lint of no rules over an empty directory finds nothing.
    NothingToLint,
    /// A blank query is refused.
    Blank,
    /// A lint of two rules of one id is refused whole, by the engine: one of them would never run.
    DuplicateIds,
    /// Every relation and operator listed has a page, a name none is has none, and the grammar has a production: the
    /// count of what breaks that.
    Catalog,
}

/// What a row asks.
type Input = Question;
/// What a row answers: a count.
type Value = usize;
/// The text of the refusal a row ends with.
type Failure = String;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_lint_of_no_rules_finds_nothing"),
        word: Word::Succeeds,
        input: || Question::NothingToLint,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_blank_query_is_refused"),
        word: Word::Refuses,
        input: || Question::Blank,
        expect: Expect::error(String::new()),
    },
    Case {
        name: CaseName("a_request_with_two_rules_of_one_id_is_refused_whole"),
        word: Word::Refuses,
        input: || Question::DuplicateIds,
        expect: Expect::Refusal(|error| {
            if error.starts_with("the analysis failed: ") && error.contains("`rule-0`") {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("every_listed_relation_and_operator_has_a_page"),
        word: Word::Succeeds,
        input: || Question::Catalog,
        expect: Expect::Value(0),
    },
];

/// How many listed relations and operators have no page, plus one for a page under a name none has, plus one for an empty
/// grammar.
fn broken(adapter: &impl AnalysisPort) -> Result<Value, AnalysisError> {
    let nobody = CatalogName("no-such-name".into());
    let mut broken = 0;
    for entry in adapter.relations()?.0 {
        broken += usize::from(adapter.relation(&entry.name)? == RelationAnswer::Unknown);
    }
    for entry in adapter.operators()?.0 {
        broken += usize::from(adapter.operator(&entry.name)? == OperatorAnswer::Unknown);
    }
    broken += usize::from(adapter.relation(&nobody)? != RelationAnswer::Unknown);
    broken += usize::from(adapter.operator(&nobody)? != OperatorAnswer::Unknown);
    broken += usize::from(adapter.grammar()?.0.is_empty());
    Ok(broken)
}

/// The answer of `adapter` to `question`, over the empty directory at `root`.
fn answer(
    adapter: &impl AnalysisPort,
    root: &MachinePath,
    question: Question,
) -> Result<Value, Failure> {
    match question {
        Question::NothingToLint => {
            let report = adapter
                .lint(AnalysisRequest {
                    root: root.clone(),
                    rules: Vec::new(),
                    placed: Vec::new(),
                    corpus: Corpus::Walked,
                    beside: Vec::new(),
                    hosted: Hosted::Nothing,
                })
                .map_err(|error| error.to_string())?;
            Ok(report.findings.len())
        }
        Question::DuplicateIds => {
            // A valid rule, sent twice under one id.
            let rule = PlannedRule {
                id: LintRuleId("rule-0".into()),
                query: QueryText(
                    "let p = select * from \"tastemaker.placed\" where template = \"x\"; emit p subject \"path\" key(\"path\");"
                        .into(),
                ),
                parameters: std::collections::BTreeMap::new(),
                include: vec![GlobText("**".into())],
                exclude: Vec::new(),
            };
            let report = adapter
                .lint(AnalysisRequest {
                    root: root.clone(),
                    rules: vec![rule.clone(), rule],
                    placed: Vec::new(),
                    corpus: Corpus::Walked,
                    beside: Vec::new(),
                    hosted: Hosted::Nothing,
                })
                .map_err(|error| error.to_string())?;
            Ok(report.findings.len())
        }
        Question::Blank => match adapter
            .validate(&QueryText(" ".into()))
            .map_err(|error| error.to_string())?
        {
            QueryVerdict::Valid => Ok(0),
            QueryVerdict::Invalid(_) => Err(String::new()),
        },
        Question::Catalog => broken(adapter).map_err(|error| error.to_string()),
    }
}

/// Runs the contract's rows against one implementation, linting the empty directory at `root`.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl AnalysisPort, root: &MachinePath) {
    run(CASES, Word::Succeeds, |question| {
        answer(adapter, root, question)
    });
    run(CASES, Word::Refuses, |question| {
        answer(adapter, root, question)
    });
}
