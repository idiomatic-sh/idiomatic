//! What the `governance` service decides for a placed rule whose globs do not compile, on values alone.

use std::collections::BTreeMap;

use super::service::{governance, governed};
use super::types::Answers;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::types::{
    GlobText, LintAnswer, LintRuleId, Plan, PlannedRule, Purpose, Rendering,
};
use crate::domain::definitions::types::{ColumnName, Diagnostic, QueryText};
use crate::domain::findings::types::{MessageTemplate, RuleId};
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::fixtures::refusing_sets;

/// What a row of the cases asks.
#[derive(Clone, Copy)]
enum Question {
    /// The answers a rule whose globs do not compile keeps.
    Kept,
    /// Whether a rule whose globs do not compile governs a corpus.
    Verdict,
}

type Input = Question;
/// What the service answered, written out, empty when it is what the row expects.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_whose_globs_do_not_compile_keeps_every_answer"),
        word: Word::Succeeds,
        input: || Question::Kept,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rule_whose_globs_do_not_compile_governs"),
        word: Word::Succeeds,
        input: || Question::Verdict,
        expect: Expect::Value(String::new()),
    },
];

/// One placed rule, `r`, that reads `src/**`.
fn plan() -> Plan {
    let id = LintRuleId("r".into());
    Plan {
        rules: vec![PlannedRule {
            id: id.clone(),
            query: QueryText("q".into()),
            parameters: BTreeMap::new(),
            include: vec![GlobText("src/**".into())],
            exclude: Vec::new(),
        }],
        purposes: BTreeMap::from([(
            id,
            Purpose::Rule(Rendering {
                rule: RuleId("p/r".into()),
                diagnostic: Diagnostic {
                    primary: ColumnName("span".into()),
                    message: MessageTemplate("m".into()),
                    help: MessageTemplate("h".into()),
                },
            }),
        )]),
        ..Plan::default()
    }
}

fn answer(path: &str) -> LintAnswer {
    LintAnswer {
        rule: LintRuleId("r".into()),
        path: RepoPath(path.into()),
        cells: Vec::new(),
    }
}

fn subject(question: Input) -> Value {
    let plan = plan();
    let (said, expected) = match question {
        Question::Kept => {
            let kept = governed(
                &plan,
                Answers {
                    answers: vec![answer("src/lib.txt"), answer("docs/a.md")],
                    unknowns: Vec::new(),
                    unread: Vec::new(),
                },
                &refusing_sets,
            );
            (
                kept.answers
                    .iter()
                    .map(|answer| &*answer.path.0)
                    .collect::<Vec<_>>()
                    .join(","),
                "src/lib.txt,docs/a.md",
            )
        }
        Question::Verdict => (
            format!(
                "{:?}",
                governance(
                    &plan,
                    &LintRuleId("r".into()),
                    &[RepoPath("docs/a.md".into())],
                    &refusing_sets
                )
            ),
            "Governs",
        ),
    };
    if said == expected {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
