//! Real borrowed-index traversal, duplicate order and admission timing controls.
use super::*;
use crate::domain::derived_views::types::{DerivedInputs, DerivedScratch, MembershipSpace};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use std::sync::Arc;
use {
    crate::domain::operator::contract::axis::PreparedAxis,
    crate::domain::table_reach::borrowed::follow,
    crate::domain::table_reach::borrowed::reach_breadth, crate::domain::table_reach::types::Walk,
    ryo_values::domain::rows::service::push_row,
};

fn number(value: i64) -> Value {
    Value::Number(Integer(value))
}

fn table(width: usize, rows: &[&[i64]]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; width],
        ..Table::default()
    };
    for row in rows {
        push_row(
            &mut table,
            &row.iter().copied().map(number).collect::<Vec<_>>(),
        );
    }
    table
}

fn program(direction: Direction) -> AxisResources {
    AxisResources {
        id: ColumnIndex(0),
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(0),
        has_filter: false,
        domain_view: Vec::new(),
        output: Vec::new(),
        direction,
    }
}

fn acquired(
    kind: ryo_derived::domain::contract::types::DerivedKind,
    request: crate::domain::derived_views::types::ViewRequest,
    rows: BorrowedRows<'_>,
) -> crate::application::file_dag::derived::owner::acquisition::service::AcquiredView {
    let plan = Arc::new(
        crate::application::file_dag::derived::prepare::view(
            (std::sync::Arc::new(
                ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
            ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
            &request,
            &[crate::domain::operation_schema::types::InputShape {
                kinds: rows.kinds(),
                tree: None,
            }],
        )
        .unwrap(),
    );
    let owner = crate::application::file_dag::relations::RelationOwner::new(
        false,
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    );
    let BorrowedRows::Computed(table) = rows else {
        panic!("computed fixture")
    };
    let publication = owner.computed(table.clone()).unwrap();
    let held = owner
        .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let rows = BorrowedRows::from(&held);
    let proof = owner.admit(publication.as_ref(), rows).unwrap();
    let inputs = [rows];
    let data = if kind == ryo_derived::domain::contract::types::DerivedKind::Membership {
        DerivedInputs::Membership {
            rows,
            space: MembershipSpace::Values,
        }
    } else {
        DerivedInputs::Rows(&inputs)
    };
    owner
        .derived
        .acquire_stored(
            &plan,
            &owner.derived.admit_view(&plan, &owner, &[&proof]).unwrap(),
            crate::application::file_dag::derived::owner::ViewContext::default(),
            data,
            &mut DerivedScratch::default(),
            None,
        )
        .unwrap()
}

#[test]
fn borrowed_axis_keeps_duplicate_direct_edges_and_breadth_order() {
    let edges = table(2, &[&[1, 2], &[1, 2], &[2, 3]]);
    let domain = table(1, &[&[2], &[3]]);
    let resource = prepare_axis(
        &program(Direction::Child),
        BorrowedRows::Computed(&edges),
        BorrowedRows::Computed(&domain),
        Some(acquired(
            ryo_derived::domain::contract::types::DerivedKind::Equality,
            crate::domain::derived_views::types::ViewRequest::Equality {
                columns: vec![ColumnIndex(0)],
            },
            BorrowedRows::Computed(&edges),
        )),
        Some(acquired(
            ryo_derived::domain::contract::types::DerivedKind::Membership,
            crate::domain::derived_views::types::ViewRequest::Membership {
                column: ColumnIndex(0),
            },
            BorrowedRows::Computed(&domain),
        )),
    )
    .unwrap();
    let PreparedAxis::Indexed {
        edges: view, to, ..
    } = resource.view().unwrap()
    else {
        panic!("borrowed indexed axis")
    };
    let mut key = Vec::new();
    let mut direct = Vec::new();
    follow(&view, to, number(1), &mut key, &mut direct);
    assert_eq!(direct, vec![number(2), number(2)]);
    let mut walk = Walk::default();
    reach_breadth(&view, to, number(1), &mut walk);
    assert_eq!(walk.reached, vec![number(2), number(3)]);
}

#[test]
fn unsupported_and_empty_gates_precede_derived_handle_requirement() {
    let edges = table(2, &[&[1, 2]]);
    let domain = table(1, &[]);
    assert!(matches!(
        prepare_axis(
            &program(Direction::Child),
            BorrowedRows::Computed(&edges),
            BorrowedRows::Computed(&domain),
            None,
            None,
        ),
        Ok(AxisResource::Empty)
    ));
    let nonempty = table(1, &[&[2]]);
    assert_eq!(
        classify(
            &program(Direction::Deep),
            BorrowedRows::Computed(&edges),
            BorrowedRows::Computed(&nonempty),
        )
        .err(),
        Some(OperatingError::WrongOperation),
    );
}

use ryo_values::domain::rows::types::Table;
