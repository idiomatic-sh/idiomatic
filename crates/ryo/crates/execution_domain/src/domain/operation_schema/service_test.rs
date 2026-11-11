//! Catalog coverage and virtual schema projection are pure domain contracts.
use super::{fixtures::*, service::schema_of, types::InputShape};
use crate::domain::{features::service::catalog, operating::errors::OperatingError};
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, HostedNumber, Input as Source, JoinColumn, Side},
    plans::{service::kind_of, types::Operation},
    run_metadata::types::Feature,
};
const FROM: Source = Source::Relation(Relation::Function);

#[derive(Clone, Copy)]
enum Scenario {
    DeclaredKinds,
    VirtualJoin,
    HostedWithoutSource,
    HostedKinds,
    CyclesOfTwoKinds,
    ClosureOfTwoKinds,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;

fn every_declared_kind_has_one_evaluator_fixture_and_one_feature() {
    let declared: std::collections::HashSet<_> = OperationKind::ALL.iter().copied().collect();
    assert_eq!(
        declared.len(),
        OperationKind::ALL.len(),
        "duplicate kind declaration"
    );
    let mut fixtures = std::collections::HashSet::new();
    for operation in operations() {
        assert!(
            fixtures.insert(kind_of(&operation)),
            "duplicate operation fixture"
        );
    }
    assert_eq!(fixtures, declared, "missing operator fixture");
    let features: Vec<_> = catalog()
        .into_iter()
        .filter_map(|feature| match feature {
            Feature::Operation(kind) => Some(kind),
            Feature::Family(_) | Feature::Direction(_) | Feature::Relation(_) => None,
        })
        .collect();
    assert_eq!(
        features.len(),
        declared.len(),
        "duplicate or omitted feature"
    );
    assert_eq!(
        features
            .into_iter()
            .collect::<std::collections::HashSet<_>>(),
        declared
    );
}

fn demand_join_schema_uses_virtual_partition_source_destination_projection()
-> Result<Value, Failure> {
    use ryo_values::domain::plan_parts::types::ReachLookup;
    let operation = Operation::Join {
        left: FROM,
        right: FROM,
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(2)],
        output: vec![
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(1),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(0),
            },
        ],
        filter: vec![],
        view: vec![],
        reach: Some(ReachLookup {
            source: ColumnIndex(1),
            destination: ColumnIndex(0),
            keys: vec![ColumnIndex(2)],
            columns: vec![ColumnIndex(2), ColumnIndex(0), ColumnIndex(1)],
        }),
    };
    let inputs = [
        InputShape {
            kinds: &[CellKind::Node],
            tree: None,
        },
        InputShape {
            kinds: &[CellKind::Symbol, CellKind::Node, CellKind::File],
            tree: None,
        },
    ];
    assert_eq!(
        schema_of(&operation, &inputs, &[])?,
        [CellKind::Symbol, CellKind::File, CellKind::Symbol]
    );
    Ok(())
}

fn hosted_without_its_request_source() -> Result<Value, Failure> {
    schema_of(
        &Operation::Hosted {
            relation: HostedNumber(0),
        },
        &[],
        &[],
    )
    .map(|_| ())
}

fn hosted_metadata_preserves_every_supplied_kind_without_constructing_rows()
-> Result<Value, Failure> {
    let supplied = [
        CellKind::Number,
        CellKind::Symbol,
        CellKind::Node,
        CellKind::Span,
        CellKind::Flag,
        CellKind::File,
    ];
    assert_eq!(
        schema_of(
            &Operation::Hosted {
                relation: HostedNumber(0)
            },
            &[InputShape {
                kinds: &supplied,
                tree: None
            }],
            &[]
        )?,
        supplied
    );
    Ok(())
}

/// A cycles step whose source is a node and whose destination is a file of the edges.
fn cycles_of_two_kinds() -> Result<Value, Failure> {
    let edges = [InputShape {
        kinds: &[CellKind::Node, CellKind::File],
        tree: None,
    }];
    schema_of(
        &Operation::Cycles {
            edges: FROM,
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        &edges,
        &[],
    )
    .map(|_| ())
}

/// A closure whose source is a node and whose destination is a file of the edges.
fn closure_of_two_kinds() -> Result<Value, Failure> {
    let edges = [InputShape {
        kinds: &[CellKind::Node, CellKind::File],
        tree: None,
    }];
    schema_of(
        &Operation::Closure {
            edges: FROM,
            keys: vec![],
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        &edges,
        &[],
    )
    .map(|_| ())
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::DeclaredKinds => {
            every_declared_kind_has_one_evaluator_fixture_and_one_feature();
            Ok(())
        }
        Scenario::VirtualJoin => {
            demand_join_schema_uses_virtual_partition_source_destination_projection()
        }
        Scenario::HostedWithoutSource => hosted_without_its_request_source(),
        Scenario::CyclesOfTwoKinds => cycles_of_two_kinds(),
        Scenario::ClosureOfTwoKinds => closure_of_two_kinds(),
        Scenario::HostedKinds => {
            hosted_metadata_preserves_every_supplied_kind_without_constructing_rows()
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_declared_kind_has_one_evaluator_fixture_and_one_feature"),
        word: Word::Succeeds,
        input: || Scenario::DeclaredKinds,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "a_demand_join_schema_uses_the_virtual_partition_source_destination_projection",
        ),
        word: Word::Succeeds,
        input: || Scenario::VirtualJoin,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("hosted_metadata_preserves_every_supplied_kind_without_constructing_rows"),
        word: Word::Succeeds,
        input: || Scenario::HostedKinds,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("hosted_without_its_request_source_is_missing_not_unsupported"),
        word: Word::Refuses,
        input: || Scenario::HostedWithoutSource,
        expect: Expect::Error(OperatingError::MissingInput { position: 1 }),
    },
    Case {
        name: CaseName("cycles_whose_source_and_destination_differ_in_kind_is_refused"),
        word: Word::Refuses,
        input: || Scenario::CyclesOfTwoKinds,
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("a_closure_whose_source_and_destination_differ_in_kind_is_refused"),
        word: Word::Refuses,
        input: || Scenario::ClosureOfTwoKinds,
        expect: Expect::Error(OperatingError::ShapeMismatch),
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
mod join;
