//! What the `catalog` service writes, on values alone.

use super::service::{entry_lines, grammar_sections, operator_sections, relation_sections};
use super::types::{
    CatalogColumn, CatalogEntry, CatalogField, CatalogLanguage, CatalogName, CatalogText,
    OperatorPage, Production, Reading, RelationPage, Requirement,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::explanations::service::page;

/// What a row asks the service to write.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    Entries,
    Relation,
    Operator,
    Grammar,
    Walkthrough,
}

/// What a row writes.
type Input = Scenario;
/// What the row wrote, when it is not what it must write; empty when it is.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_entry_is_one_line"),
        word: Word::Succeeds,
        input: || Scenario::Entries,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_relation_says_a_line_per_column_and_per_language"),
        word: Word::Succeeds,
        input: || Scenario::Relation,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_operator_says_a_line_per_field_and_its_example"),
        word: Word::Succeeds,
        input: || Scenario::Operator,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_grammar_says_a_line_per_production"),
        word: Word::Succeeds,
        input: || Scenario::Grammar,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "the_ryoql_page_opens_with_four_steps_and_worked_queries_before_the_grammar",
        ),
        word: Word::Succeeds,
        input: || Scenario::Walkthrough,
        expect: Expect::Value(String::new()),
    },
];

fn name(text: &str) -> CatalogName {
    CatalogName(text.into())
}

fn entry() -> CatalogEntry {
    CatalogEntry {
        name: name("defines"),
        summary: CatalogText("A declaration.".into()),
    }
}

/// What the row writes, and what it must.
fn written(scenario: Scenario) -> (String, &'static str) {
    match scenario {
        Scenario::Entries => (
            entry_lines(&[entry()])
                .into_iter()
                .map(|line| line.0.to_string())
                .collect(),
            "- `defines`: A declaration.",
        ),
        Scenario::Relation => (
            page(&relation_sections(&RelationPage {
                entry: entry(),
                columns: vec![CatalogColumn {
                    name: name("kind"),
                    kind: name("string"),
                    words: vec![name("function"), name("method")],
                }],
                languages: vec![
                    CatalogLanguage {
                        name: name("go"),
                        reading: Reading::Reads,
                        unknowns: vec![name("unnamed")],
                    },
                    CatalogLanguage {
                        name: name("css"),
                        reading: Reading::Absent(name("no-such-construct")),
                        unknowns: Vec::new(),
                    },
                ],
            }))
            .0
            .into(),
            "## defines\n\nA declaration.\n\n## Columns\n\n- `kind` string, one of `function`, `method`\n\n## Languages\n\n- go: reads; unknown values `unnamed`\n- css: absent (no-such-construct)\n",
        ),
        Scenario::Operator => (
            page(&operator_sections(&OperatorPage {
                entry: entry(),
                fields: vec![CatalogField {
                    name: name("left"),
                    requirement: Requirement::Required,
                    summary: CatalogText("Left input.".into()),
                }],
                options: Vec::new(),
                example: CatalogText("let x = select * from \"function\";".into()),
            }))
            .0
            .into(),
            "## defines\n\nA declaration.\n\n## Fields\n\n- `left` required: Left input.\n\n## Example\n\n`let x = select * from \"function\";`\n",
        ),
        Scenario::Grammar => (
            {
                // The grammar is the page's last section, after the walkthrough.
                let said: String = page(&grammar_sections(&[Production {
                    name: name("parameter"),
                    form: CatalogText("param <name>: <type>;".into()),
                    example: CatalogText("param max-lines: integer;".into()),
                }]))
                .0
                .into();
                said.find("## RyoQL")
                    .and_then(|at| said.get(at..))
                    .map_or_else(|| said.clone(), str::to_owned)
            },
            "## RyoQL\n\n- parameter: `param <name>: <type>;`, as in `param max-lines: integer;`\n",
        ),
        Scenario::Walkthrough => {
            let said: String = page(&grammar_sections(&[])).0.into();
            // The four steps come first, in order, then the worked queries, each fenced, then the grammar.
            let steps = ["1. Pick a relation", "2. Narrow it", "3. Join", "4. Emit"]
                .iter()
                .map(|step| said.find(step))
                .collect::<Option<Vec<_>>>();
            let ordered = steps.as_ref().is_some_and(|at| {
                at.windows(2)
                    .all(|pair| matches!(pair, [before, after] if before < after))
            });
            let worked = said.find("## Worked queries");
            let grammar = said.find("## RyoQL");
            let fenced = said.matches("```text\n").count();
            if said.starts_with("## Writing a query\n")
                && ordered
                && worked.is_some_and(|worked| {
                    steps
                        .as_ref()
                        .is_some_and(|at| at.iter().all(|step| *step < worked))
                })
                && grammar > worked
                && (2..=3).contains(&fenced)
            {
                (String::new(), "")
            } else {
                (said, "")
            }
        }
    }
}

/// What the row wrote, when it is not what it must write; empty when it is.
fn subject(scenario: Scenario) -> String {
    let (got, wanted) = written(scenario);
    if got == wanted { String::new() } else { got }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |scenario| Ok(subject(scenario)));
}
