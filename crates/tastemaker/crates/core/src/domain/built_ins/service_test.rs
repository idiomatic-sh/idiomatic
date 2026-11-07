//! The `built_ins` service's cases, on a plan's values: the rules the lint call carries besides the placed ones.

use super::service::{comments, compared};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::types::Plan;
use crate::domain::contracts::types::{Contract, RenderedContent, TemplateIdentity};
use crate::domain::definitions::types::Exactness;
use crate::domain::paths::types::RepoPath;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The comments rule over a plan's files.
    Comments,
    /// The comparison rules over one placed file and its rendered content, whatever its language.
    Compared,
    /// The comparison rules over an exact contract: the runs and literals as well.
    Exact,
}

type Input = Question;
/// What the row said, empty when it said what was expected.
type Value = String;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_comments_rule_reads_every_file_not_excluded_whatever_its_language"),
        word: Word::Succeeds,
        input: || Question::Comments,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_placed_file_is_read_by_every_shared_relation_whatever_its_extension"),
        word: Word::Succeeds,
        input: || Question::Compared,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_exact_contract_is_read_by_its_runs_and_literals_too"),
        word: Word::Succeeds,
        input: || Question::Exact,
        expect: Expect::Value(String::new()),
    },
];

fn path(text: &str) -> RepoPath {
    RepoPath(text.into())
}

/// Each rule of the plan, by its id and what it reads.
fn rules(plan: &Plan) -> String {
    plan.rules
        .iter()
        .map(|rule| {
            format!(
                "{} {}",
                rule.id.0,
                rule.include
                    .iter()
                    .map(|glob| &*glob.0)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn subject(question: Input) -> Value {
    let mut plan = Plan {
        files: vec![path("a.txt"), path("b/c.txt"), path("d.py"), path("e.txt")],
        excluded: [path("e.txt")].into(),
        ..Plan::default()
    };
    let (said, expected) = match question {
        Question::Comments => {
            comments(&mut plan);
            (
                rules(&plan),
                "tastemaker-comments a.txt,b/c.txt,d.py".to_owned(),
            )
        }
        Question::Compared | Question::Exact => {
            let exact = if matches!(question, Question::Exact) {
                Exactness::Exact
            } else {
                Exactness::Contains
            };
            compared(
                &mut plan,
                vec![Contract {
                    target: path("d.py"),
                    rendered: path(".tastemaker-rendered/0.d.py"),
                    text: RenderedContent("def a():\n    pass\n".into()),
                    identity: TemplateIdentity("p/t".into()),
                    exact,
                }],
            );
            let both = "d.py,.tastemaker-rendered/0.d.py";
            let mut wanted = [
                "defines",
                "scopes",
                "decorates",
                "visibility",
                "annotates",
                "entries",
                "mappings",
                "elements",
            ]
            .iter()
            .map(|name| format!("declarations-{name} {both}"))
            .collect::<Vec<_>>();
            if exact == Exactness::Exact {
                wanted.extend(
                    ["prose", "literals"].map(|name| format!("declarations-{name} {both}")),
                );
            }
            (rules(&plan), wanted.join("; "))
        }
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
