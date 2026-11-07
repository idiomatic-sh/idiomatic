//! Proves the core domain's `native_positions` rules.

use super::{
    axis::axis,
    types::{PositionInput, PositionWork},
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Direction, Input as PlanInput, ObservationBindingId, StepId},
    plans::types::Operation,
};
use {
    crate::domain::lowering::types::ElementPart, crate::domain::lowering::types::LoweredColumn,
    crate::domain::lowering::types::LoweredShape, crate::domain::lowering::types::RowPosition,
    crate::domain::lowering::types::RowWidth, crate::domain::operating::errors::OperatingError,
    crate::domain::operating::types::OperatorContext,
    crate::domain::operator::contract::types::WorkCounters,
    crate::domain::positions::service_test::SMALL_SOURCE,
    crate::domain::positions::service_test::rows_over,
    crate::domain::positions::service_test::small_tree, crate::domain::positions::types::Positions,
    crate::domain::positions::types::Slots, crate::domain::table_scratch::types::OperatorScratch,
    ryo_values::domain::files::types::SourceView,
};

fn shape(columns: usize) -> LoweredShape {
    LoweredShape {
        width: RowWidth(1),
        columns: (0..columns)
            .map(|_| LoweredColumn {
                slot: RowPosition(0),
                field: ElementPart::Node,
            })
            .collect(),
    }
}

#[derive(Clone, Copy)]
enum Scenario {
    Subset,
    Empty,
    BoundDomain,
}
type Input = Scenario;
type Value = (Option<(u32, u32)>, [u64; 3]);
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("step_domain_is_third_operand_and_supports_wide_domain_id"),
        word: Word::Succeeds,
        input: || Scenario::Subset,
        expect: Expect::Value((Some((0, 2)), [1, 3, 1])),
    },
    Case {
        name: CaseName("empty_step_domain_rejects_every_reached_child"),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value((None, [1, 3, 0])),
    },
    // The control of this row is the first one: the same operation over a step domain answers.
    Case {
        name: CaseName("a_bound_observation_is_not_a_step_domain"),
        word: Word::Refuses,
        input: || Scenario::BoundDomain,
        expect: Expect::Error(OperatingError::NotAStep),
    },
];

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    let fixture = small_tree();
    let mut symbols = fixture.symbols;
    let tree = rows_over(&fixture.elements);
    let source_shape = shape(1);
    let edge_shape = shape(1);
    let domain_shape = shape(2);
    let answer_shape = LoweredShape {
        width: RowWidth(2),
        columns: vec![],
    };
    let from = Positions {
        width: RowWidth(1),
        slots: Slots(vec![0]),
    };
    let edges = Positions {
        width: RowWidth(1),
        slots: Slots(vec![1, 2, 3]),
    };
    let domain = Positions {
        width: RowWidth(1),
        slots: Slots(match scenario {
            Scenario::Subset => vec![2],
            Scenario::Empty | Scenario::BoundDomain => vec![],
        }),
    };
    let operation = Operation::Axis {
        from: PlanInput::Step(StepId(0)),
        edges: PlanInput::Relation(Relation::SyntaxChild),
        domain: match scenario {
            Scenario::Subset | Scenario::Empty => PlanInput::Step(StepId(2)),
            Scenario::BoundDomain => PlanInput::BoundObservation(ObservationBindingId(0)),
        },
        direction: Direction::Child,
        id: ColumnIndex(0),
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(1),
        domain_filter: vec![],
        domain_view: vec![],
        output: vec![],
    };
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
    let counts = WorkCounters::default();
    let inputs = [
        PositionInput {
            shape: &source_shape,
            rows: &from,
        },
        PositionInput {
            shape: &edge_shape,
            rows: &edges,
        },
        PositionInput {
            shape: &domain_shape,
            rows: &domain,
        },
    ];
    let selected = axis(
        &operation,
        &tree,
        &inputs,
        &answer_shape,
        &mut context,
        &mut PositionWork::default(),
        &counts,
    )?;
    let pair = match selected.slots.0.as_slice() {
        [from, reached] => Some((*from, *reached)),
        [] => None,
        _ => return Err(OperatingError::ShapeMismatch),
    };
    Ok((pair, counts.snapshot()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
