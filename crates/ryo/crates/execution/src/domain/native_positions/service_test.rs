//! The `native_positions` service's cases, on values alone.

use super::{
    service::evaluate,
    types::{PositionInput, PositionWork},
};
use crate::domain::lowering::types::{
    ElementPart, LoweredColumn, LoweredShape, RowPosition, RowWidth,
};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::contract::types::{PrimitiveSemantics, WorkCounters};
use crate::domain::positions::service_test::{SMALL_SOURCE, SmallTree, rows_over, small_tree};
use crate::domain::positions::types::{Positions, Slots};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::{OperatorAnswer, OperatorCall};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Direction, HostedNumber, Input as PlanInput, StepId,
};
use ryo_values::domain::plans::types::Operation;

/// The operators these cases hand to the part: none is ever asked.
struct NoOperators;
impl PrimitiveSemantics for NoOperators {
    fn operators(&self, _call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        Err(PortableOperatorFault::WrongOperation)
    }
}

#[derive(Clone, Copy)]
enum Scenario {
    WalkUp,
    HostedRows,
}
type Input = Scenario;
/// The slots of the answer: the position walked from and the one reached.
type Value = [u32; 2];
type Failure = OperatingError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let rows = rows_over(&elements);
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SMALL_SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let operation = match scenario {
        Scenario::WalkUp => Operation::Axis {
            from: PlanInput::Step(StepId(0)),
            edges: PlanInput::Relation(Relation::SyntaxChild),
            domain: PlanInput::Relation(Relation::SyntaxTree),
            direction: Direction::Parent,
            id: ColumnIndex(0),
            parent: ColumnIndex(0),
            child: ColumnIndex(1),
            domain_id: ColumnIndex(0),
            domain_filter: Vec::new(),
            domain_view: Vec::new(),
            output: Vec::new(),
        },
        Scenario::HostedRows => Operation::Hosted {
            relation: HostedNumber(0),
        },
    };
    let source_shape = LoweredShape {
        width: RowWidth(1),
        columns: vec![LoweredColumn {
            slot: RowPosition(0),
            field: ElementPart::Node,
        }],
    };
    let starts = Positions {
        width: RowWidth(1),
        slots: Slots(vec![3]),
    };
    let inputs = [PositionInput {
        shape: &source_shape,
        rows: &starts,
    }];
    let answer = evaluate(
        &NoOperators,
        None,
        &operation,
        &rows,
        &inputs,
        &LoweredShape {
            width: RowWidth(2),
            columns: vec![],
        },
        &mut context,
        &mut PositionWork::default(),
        &WorkCounters::default(),
    )?;
    let [from, to] = answer.slots.0.as_slice() else {
        return Err(OperatingError::ShapeMismatch);
    };
    Ok([*from, *to])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_axis_walk_is_evaluated_by_the_native_axis"),
        word: Word::Succeeds,
        input: || Scenario::WalkUp,
        expect: Expect::Value([3, 0]),
    },
    Case {
        name: CaseName("an_operation_the_native_operators_do_not_run_is_refused"),
        word: Word::Refuses,
        input: || Scenario::HostedRows,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
