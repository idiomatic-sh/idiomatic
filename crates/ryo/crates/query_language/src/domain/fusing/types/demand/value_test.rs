//! Only exclusively source-bound closure consumers may abandon all-pairs evaluation.

use super::value::DemandClosure;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input, JoinColumn, Operand, RepeatedRows, Side, StepId, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep, StepScope};

fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}
fn closure(keys: Vec<ColumnIndex>) -> PlanStep {
    step(Operation::Closure {
        edges: Input::Relation(Relation::SemanticChild),
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
        keys,
    })
}
fn join(right: u32, right_on: Vec<ColumnIndex>) -> PlanStep {
    step(Operation::Join {
        left: Input::Relation(Relation::Function),
        right: Input::Step(StepId(right)),
        left_on: vec![ColumnIndex(0); right_on.len()],
        right_on,
        output: vec![JoinColumn {
            side: Side::Right,
            column: ColumnIndex(1),
        }],
        filter: vec![],
        view: vec![],
        reach: None,
    })
}

#[test]
fn all_source_keyed_consumers_share_physical_edges_without_exposing_closure_rows() {
    let steps = [
        closure(vec![]),
        join(0, vec![ColumnIndex(0)]),
        join(0, vec![ColumnIndex(0), ColumnIndex(1)]),
    ];
    let rewritten = DemandClosure::new(&steps, &[]).rewrite();
    for item in &rewritten[1..] {
        let Operation::Join {
            right,
            reach: Some(reach),
            ..
        } = &item.operation
        else {
            panic!("source-bound join is physical demand");
        };
        assert_eq!(*right, Input::Relation(Relation::SemanticChild));
        assert_eq!(reach.columns, [ColumnIndex(0), ColumnIndex(1)]);
    }
    // This pass changes only consumers. Existing ordinary unused-step pruning removes the unreferenced root.
    assert_eq!(rewritten[0], steps[0]);
}

#[test]
fn pure_reordered_duplicate_projection_preserves_the_logical_rhs_map() {
    let steps = [
        closure(vec![ColumnIndex(2)]),
        step(Operation::Project {
            from: Input::Step(StepId(0)),
            columns: vec![
                ColumnIndex(2),
                ColumnIndex(1),
                ColumnIndex(0),
                ColumnIndex(1),
            ],
            distinct: RepeatedRows::Kept,
        }),
        join(1, vec![ColumnIndex(2), ColumnIndex(3)]),
    ];
    let rewritten = DemandClosure::new(&steps, &[]).rewrite();
    let Operation::Join {
        reach: Some(reach), ..
    } = &rewritten[2].operation
    else {
        panic!("partition+source bound");
    };
    assert_eq!(reach.keys, [ColumnIndex(2)]);
    assert_eq!(
        reach.columns,
        [
            ColumnIndex(2),
            ColumnIndex(1),
            ColumnIndex(0),
            ColumnIndex(1)
        ]
    );
}

#[test]
fn any_exported_full_or_unbound_consumer_retains_the_original_graph() {
    let simple = [closure(vec![]), join(0, vec![ColumnIndex(0)])];
    assert_eq!(DemandClosure::new(&simple, &[StepId(0)]).rewrite(), simple);
    let full = [
        closure(vec![]),
        join(0, vec![ColumnIndex(0)]),
        step(Operation::Project {
            from: Input::Step(StepId(0)),
            columns: vec![ColumnIndex(1)],
            distinct: RepeatedRows::Kept,
        }),
    ];
    assert_eq!(DemandClosure::new(&full, &[StepId(2)]).rewrite(), full);
    let destination = [closure(vec![]), join(0, vec![ColumnIndex(1)])];
    assert_eq!(DemandClosure::new(&destination, &[]).rewrite(), destination);
    let partition = [closure(vec![ColumnIndex(2)]), join(0, vec![ColumnIndex(1)])];
    assert_eq!(DemandClosure::new(&partition, &[]).rewrite(), partition);
    let mixed = [
        closure(vec![]),
        join(0, vec![ColumnIndex(0)]),
        join(0, vec![ColumnIndex(1)]),
    ];
    assert_eq!(DemandClosure::new(&mixed, &[]).rewrite(), mixed);
}

#[test]
fn distinct_predicate_native_and_cross_file_barriers_are_unchanged() {
    let mut native = [closure(vec![]), join(0, vec![ColumnIndex(0)])];
    if let Operation::Closure { edges, .. } = &mut native[0].operation {
        *edges = Input::Relation(Relation::SyntaxChild);
    }
    assert_eq!(DemandClosure::new(&native, &[]).rewrite(), native);
    let mut cross = [closure(vec![]), join(0, vec![ColumnIndex(0)])];
    cross[0].scope = StepScope::CrossFile;
    assert_eq!(DemandClosure::new(&cross, &[]).rewrite(), cross);
    let distinct = [
        closure(vec![]),
        step(Operation::Project {
            from: Input::Step(StepId(0)),
            columns: vec![ColumnIndex(0), ColumnIndex(1)],
            distinct: RepeatedRows::Dropped,
        }),
        join(1, vec![ColumnIndex(0)]),
    ];
    assert_eq!(DemandClosure::new(&distinct, &[]).rewrite(), distinct);
    let mut filtered = [closure(vec![]), join(0, vec![ColumnIndex(0)])];
    if let Operation::Join { filter, .. } = &mut filtered[1].operation {
        filter.push(Comparison {
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Column(ColumnIndex(0)),
            test: Test::Equal,
            origin: None,
        });
    }
    assert_eq!(DemandClosure::new(&filtered, &[]).rewrite(), filtered);
}

#[test]
fn distinct_projected_edge_input_maps_endpoints_and_partitions_without_changing_outer_bags() {
    let steps = [
        step(Operation::Project {
            from: Input::Relation(Relation::SemanticChild),
            columns: vec![
                ColumnIndex(1),
                ColumnIndex(4),
                ColumnIndex(0),
                ColumnIndex(1),
            ],
            distinct: RepeatedRows::Dropped,
        }),
        step(Operation::Closure {
            edges: Input::Step(StepId(0)),
            source: ColumnIndex(2),
            destination: ColumnIndex(0),
            keys: vec![ColumnIndex(1)],
        }),
        join(1, vec![ColumnIndex(0), ColumnIndex(1)]),
    ];
    let result = DemandClosure::new(&steps, &[]).rewrite();
    let Operation::Join {
        right,
        reach: Some(reach),
        ..
    } = &result[2].operation
    else {
        panic!("projected graph recognized");
    };
    assert_eq!(*right, Input::Relation(Relation::SemanticChild));
    assert_eq!(
        (reach.source, reach.destination),
        (ColumnIndex(0), ColumnIndex(1))
    );
    assert_eq!(reach.keys, [ColumnIndex(4)]);
    assert_eq!(
        reach.columns,
        [ColumnIndex(0), ColumnIndex(1), ColumnIndex(2)]
    );
}

#[test]
fn a_projection_with_an_unchecked_extra_column_retains_its_validation_barrier() {
    let steps = [
        step(Operation::Project {
            from: Input::Relation(Relation::SemanticChild),
            columns: vec![ColumnIndex(0), ColumnIndex(1), ColumnIndex(3)],
            distinct: RepeatedRows::Dropped,
        }),
        step(Operation::Closure {
            edges: Input::Step(StepId(0)),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            keys: vec![],
        }),
        join(1, vec![ColumnIndex(0)]),
    ];
    assert_eq!(DemandClosure::new(&steps, &[]).rewrite(), steps);
}
