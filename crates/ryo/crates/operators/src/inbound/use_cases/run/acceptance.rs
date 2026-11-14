//! Executable acceptance steps for running a planned node.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_operators::inbound::use_cases::run::fixtures::{Step, TableInputs, TextResources, emitted};
use ryo_relations::domain::relations::fixtures::{kinds, node_id};
use ryo_relations::domain::relations::types::{FileId, Offset, Position, Relation, Span, Value};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

#[given(expr = "the run input {string}")]
fn given_input(world: &mut Behavior, input: String) {
    world.input = input;
}

#[when(expr = "run runs")]
fn runs(world: &mut Behavior) {
    let table = Table {
        kinds: kinds(Relation::Function).to_vec(),
        cells: vec![
            Value::Node(node_id(FileId(0), Position(1))),
            Value::Symbol(Symbol(0)),
            Value::File(FileId(0)),
            Value::Span(Span {
                start: Offset(0),
                end: Offset(1),
            }),
        ],
        count: RowCount(1),
    };
    let held: &[(Relation, &Table)] = if world.input == "refused" {
        &[]
    } else {
        &[(Relation::Function, &table)]
    };
    let inputs = TableInputs::new(held);
    let steps = [Step {
        operation: Operation::Select {
            from: Input::Relation(Relation::Function),
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        inputs: vec![PlanInput::Relation(Relation::Function)],
    }];
    world.answer = match emitted(
        &steps,
        &[PlanNodeId(0)],
        &inputs,
        &mut TextResources::new(&[]),
        &mut FileBuffers::default(),
    ) {
        Ok(rows) => format!("{} row", rows.first().map_or(0, |rows| rows.count.0)),
        Err(fault) => format!("refused {fault:?}"),
    };
}

#[then(expr = "run answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber owns captured strings"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
