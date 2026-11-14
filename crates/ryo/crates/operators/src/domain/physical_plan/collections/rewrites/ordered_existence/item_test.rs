//! The `ordered_existence` item's cases: the shapes it rewrites and the ones it declines.
use super::item::OrderedExistenceItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::equality_join::types::{OrderedProbe, PartitionExtremum, ProbeKey, ProbeMode};
use crate::domain::physical_plan::collections::rewrites::interface::RewritesItem as _;
use crate::domain::physical_plan::fixtures::{EDGES, OrderedShape, ordered_existence};
use crate::domain::physical_plan::service::plan_with;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeReaders, PlanInputList, PlanNodeId, RowSource, RuleOrdinal,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepScope, Test};

/// The shape of the program planned.
type Input = OrderedShape;
/// What became of the antijoin, the fourth node.
#[derive(Debug, PartialEq)]
enum Rewritten {
    /// It probes `right` in the ordered mode, keyed on the parent on both sides when `on_parent`, and the self join, the
    /// select and the projection have `unread` of them with no reader left.
    Ordered { right: RowSource, on_parent: bool, order: OrderedProbe, unread: usize },
    /// It is left as it was: an anti probe of the projection.
    Declined,
}
/// What became of the antijoin.
type Value = Rewritten;
type Failure = ();

/// The shape the item rewrites: per parent, the children of the greatest ordinal.
const LATEST: OrderedShape = OrderedShape { test: Test::LessThan, order: 2, key: &[0, 1, 2], shared_select: false };

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_ordered_existence_shape_probes_its_source"),
        word: Word::Succeeds,
        input: || LATEST,
        expect: Expect::Value(Rewritten::Ordered {
            right: RowSource::Facts(EDGES),
            on_parent: true,
            order: OrderedProbe { left_order: ColumnIndex(2), right_order: ColumnIndex(2), extremum: PartitionExtremum::Maximum },
            unread: 3,
        }),
    },
    Case {
        name: CaseName("a_greater_than_order_keeps_the_least"),
        word: Word::Succeeds,
        input: || OrderedShape { test: Test::GreaterThan, ..LATEST },
        expect: Expect::Value(Rewritten::Ordered {
            right: RowSource::Facts(EDGES),
            on_parent: true,
            order: OrderedProbe { left_order: ColumnIndex(2), right_order: ColumnIndex(2), extremum: PartitionExtremum::Minimum },
            unread: 3,
        }),
    },
    Case {
        name: CaseName("an_order_column_not_proven_a_number_declines"),
        word: Word::Succeeds,
        input: || OrderedShape { order: 1, ..LATEST },
        expect: Expect::Value(Rewritten::Declined),
    },
    Case {
        name: CaseName("a_select_another_node_reads_declines"),
        word: Word::Succeeds,
        input: || OrderedShape { shared_select: true, ..LATEST },
        expect: Expect::Value(Rewritten::Declined),
    },
    Case {
        name: CaseName("a_non_strict_order_declines"),
        word: Word::Succeeds,
        input: || OrderedShape { test: Test::LessThanOrEqual, ..LATEST },
        expect: Expect::Value(Rewritten::Declined),
    },
    Case {
        name: CaseName("an_antijoin_key_without_the_order_declines"),
        word: Word::Succeeds,
        input: || OrderedShape { key: &[0, 1], ..LATEST },
        expect: Expect::Value(Rewritten::Declined),
    },
];

/// Plans the shape with no rewrite, the antijoin a rule's emit, then applies the item.
fn subject(shape: Input) -> Result<Value, Failure> {
    let steps = ordered_existence(0, shape);
    let mut program = LogicalProgram::new();
    for step in &steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    program.emit(RuleOrdinal(0), PlanNodeId(3));
    let mut plan = plan_with(&program, &[]).map_err(|_| ())?;
    OrderedExistenceItem.rewrite(&mut plan.nodes, &mut plan.indexes);
    let unread = plan.nodes.iter().take(3).filter(|node| node.readers == NodeReaders(0)).count();
    match &plan.nodes.get(3).ok_or(())?.lowering {
        Lowering::Probe { right, probe, .. } => match probe.mode {
            ProbeMode::Ordered(order) => {
                let on_parent = matches!(&probe.key, ProbeKey::Columns { left, right }
                    if left == right && left.as_slice() == [ColumnIndex(0)]);
                Ok(Rewritten::Ordered { right: *right, on_parent, order, unread })
            }
            ProbeMode::Anti => Ok(Rewritten::Declined),
            ProbeMode::Inner | ProbeMode::Semi => Err(()),
        },
        _ => Err(()),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
