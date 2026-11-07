//! Catalog coverage and virtual schema projection are pure domain contracts.
use super::{fixtures::*, service::schema_of, types::InputShape};
use crate::domain::{features::service::catalog, operating::errors::OperatingError};
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, HostedNumber, Input as Source, JoinColumn, Side},
    plans::{service::kind_of, types::Operation},
    run_metadata::types::Feature,
};
const FROM: Source = Source::Relation(Relation::Function);

#[test]
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

#[test]
fn demand_join_schema_uses_virtual_partition_source_destination_projection() {
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
        schema_of(&operation, &inputs, &[]).expect("virtual kinds"),
        [CellKind::Symbol, CellKind::File, CellKind::Symbol]
    );
}

#[test]
fn hosted_without_its_request_source_is_missing_not_unsupported() {
    assert_eq!(
        schema_of(
            &Operation::Hosted {
                relation: HostedNumber(0)
            },
            &[],
            &[]
        ),
        Err(OperatingError::MissingInput { position: 1 })
    );
}

#[test]
fn hosted_metadata_preserves_every_supplied_kind_without_constructing_rows() {
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
        )
        .unwrap(),
        supplied
    );
}
