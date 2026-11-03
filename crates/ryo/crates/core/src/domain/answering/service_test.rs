//! The `answering` service's cases, on values alone.
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput, StepId};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};

use super::service::{cross_steps, settle, todo_list};
use super::types::{CrossSteps, Crossed, Todo};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::operator::types::RowsFed;
use crate::domain::render::service::{end_row, lent_row, row_cells};
use crate::domain::render::types::{Rendered, RowNumber};
use ryo_values::domain::records::service::keep;
use ryo_values::domain::records::types::{Cell, CellText, RowsTextView};
use ryo_values::domain::runs::types::RuleId;

/// What a row of the cases does; a number packs the answers of the calls it names, first call in the highest digits.
#[derive(Clone, Copy)]
enum Scenario {
    /// `cross_steps` on a plan with a cross-file step, on one with per-file steps only, on one with no step.
    CrossSteps,
    /// `todo_list` on three plans of which the second has no cross-file step: the places of the rules to make.
    TodoMixed,
    /// `settle` adds the rows each step was fed.
    SettleFed,
    /// `settle` puts the findings after those already there.
    SettleLater,
}

/// What a row of the cases does.
type Input = Scenario;
/// A yes or no as a number, or what a function counts or picks.
type Value = usize;
type Failure = ();

fn plan(scopes: &[StepScope]) -> Plan {
    Plan {
        parameters: Vec::new(),
        steps: scopes
            .iter()
            .map(|scope| PlanStep {
                operation: Operation::Union {
                    left: PlanInput::Relation(Relation::Defines),
                    right: PlanInput::Step(StepId(0)),
                },
                scope: *scope,
            })
            .collect(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

fn has(plan: &Plan) -> usize {
    usize::from(cross_steps(plan) == CrossSteps::Has)
}

/// Rows of one text cell each, one for each of `texts`.
fn rows_of(texts: &[&str]) -> Rendered {
    let mut rendered = Rendered::default();
    for text in texts {
        let kept = keep(&mut rendered.text, Cell::Text(CellText(text)));
        rendered.cells.push(Some(kept));
        end_row(&mut rendered);
    }
    rendered
}

/// Settles a rule whose steps found the rows `found` and were fed `rows` rows, over findings that already hold the rows `before`: the
/// rows fed, and the text of the rows after, one after the other.
fn settled(rows: u64, before: &[&str], found: &[&str]) -> (u64, String) {
    let todo = Todo {
        rule: RuleIndex(0),
        id: RuleId("r0".into()),
    };
    let mut later = rows_of(before);
    let mut fed = vec![vec![RowsFed::default()]];
    let crossed = Crossed {
        rendered: rows_of(found),
        fed: vec![RowsFed([rows, 0, 0])],
    };
    settle(&todo, crossed, &mut later, &mut fed);
    let texts: String = (0..later.ends.0.len())
        .flat_map(|row| {
            lent_row(
                RowsTextView(&later.text),
                row_cells(
                    &later.cells,
                    &later.ends,
                    RowNumber(u32::try_from(row).unwrap_or(0)),
                ),
            )
        })
        .map(|cell| match cell {
            Some(Cell::Text(text)) => text.0,
            Some(Cell::File(_) | Cell::Number(_) | Cell::Span(_)) | None => "",
        })
        .collect();
    (
        fed.first()
            .and_then(|each| each.first())
            .map_or(0, |each| each.0[0]),
        texts,
    )
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::CrossSteps => {
            has(&plan(&[StepScope::PerFile, StepScope::CrossFile])) * 100
                + has(&plan(&[StepScope::PerFile])) * 10
                + has(&plan(&[]))
        }
        Scenario::TodoMixed => {
            let plans = [
                plan(&[StepScope::CrossFile]),
                plan(&[StepScope::PerFile]),
                plan(&[StepScope::CrossFile]),
            ];
            let ids = [RuleId("a".into()), RuleId("b".into()), RuleId("c".into())];
            todo_list(&plans, &ids)
                .iter()
                .fold(0, |packed, each| packed * 10 + each.rule.0 + 1)
        }
        Scenario::SettleFed => usize::try_from(settled(7, &[], &[]).0).unwrap_or(0),
        Scenario::SettleLater => usize::from(settled(0, &["a"], &["b"]).1 == "ab"),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("cross_steps_are_found_in_a_plan_that_has_one"),
        word: Word::Succeeds,
        input: || Scenario::CrossSteps,
        expect: Expect::Value(100),
    },
    Case {
        name: CaseName("the_rules_to_make_are_those_with_cross_file_steps"),
        word: Word::Succeeds,
        input: || Scenario::TodoMixed,
        expect: Expect::Value(13),
    },
    Case {
        name: CaseName("settling_adds_the_rows_each_step_was_fed"),
        word: Word::Succeeds,
        input: || Scenario::SettleFed,
        expect: Expect::Value(7),
    },
    Case {
        name: CaseName("settling_puts_the_findings_after_those_already_there"),
        word: Word::Succeeds,
        input: || Scenario::SettleLater,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
