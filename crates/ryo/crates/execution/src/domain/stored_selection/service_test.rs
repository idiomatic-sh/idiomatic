//! The projected tuple contract retains collision and invalid-cell controls through the capability.
use ryo_derived::domain::contract::types::{
    BuildOutcome, BuildWorkspace, DerivedFault, DerivedInputs, ViewRequest,
};
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn};
use ryo_values::domain::plan_parts::types::ColumnIndex;

fn representatives(
    source: ColumnsView<'_>,
    columns: &[ColumnIndex],
    positions: &[u32],
) -> Result<Vec<u32>, DerivedFault> {
    let port = std::sync::Arc::new(
        ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
    );
    let plan = port.prepare(
        ViewRequest::DistinctRepresentatives {
            columns: columns.to_vec(),
        },
        vec![source.kinds.to_vec()],
    );
    let rows = crate::domain::positions::service_test::SemanticRows(
        crate::domain::table_model::types::Rows::Stored(source),
    );
    let BuildOutcome::Built(payload) = port.build(
        &plan,
        DerivedInputs::Distinct {
            rows: &rows,
            positions,
        },
        &mut BuildWorkspace::default(),
        None,
    )?
    else {
        panic!("readable distinct rows")
    };
    Ok(payload
        .share_positions()
        .expect("successful distinct payload")
        .as_slice()
        .to_vec())
}
#[test]
fn distinct_representatives_handle_duplicates_and_invalid_cells() {
    let columns = [
        FactColumn(vec![7, 7, 7, 7, 7, 8]),
        FactColumn(vec![1, 2, 0, 7, 0, 1]),
        FactColumn(vec![10, 10, 20, 11, 20, 10]),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Flag, CellKind::Symbol],
        columns: &columns,
    };
    let on = [ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)];
    assert_eq!(
        representatives(source, &on, &[0, 1, 2, 3, 4]).unwrap(),
        [0, 2, 3]
    );
    assert!(matches!(
        representatives(source, &on, &[0, 99]),
        Err(DerivedFault::ShapeMismatch)
    ));
    assert!(matches!(
        representatives(source, &[ColumnIndex(99)], &[0]),
        Err(DerivedFault::NoSuchColumn {
            column: ColumnIndex(99)
        })
    ));
}
#[test]
fn distinct_representatives_compare_exact_values_after_a_real_full_hash_collision() {
    use ryo_derived::domain::contract::key::hash_row;
    use ryo_derived::domain::contract::types::On;
    use ryo_relations::domain::relations::types::{Integer, Value};
    let collision = 0x517c_c1b7_2722_0a95_u64.rotate_left(5);
    let columns = [FactColumn(vec![0, 1, 0]), FactColumn(vec![0, collision, 0])];
    let source = ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    };
    let on = [ColumnIndex(0), ColumnIndex(1)];
    assert_eq!(
        hash_row(
            &[Value::Number(Integer(0)), Value::Number(Integer(0))],
            On::Whole
        ),
        hash_row(
            &[
                Value::Number(Integer(1)),
                Value::Number(Integer(collision.cast_signed()))
            ],
            On::Whole
        ),
        "actual semantic full-hash collision"
    );
    assert_eq!(representatives(source, &on, &[0, 1, 2]).unwrap(), [0, 1]);
}
