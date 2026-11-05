//! Comparison families and real common-engine input/key validation.
use crate::application::file_dag::testing::TestOperator as _;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::family_of;
use crate::domain::operating::types::{Family, OperatorContext};
use crate::domain::operator::collections::operators::antijoin::item::AntijoinItem;
use ryo_values::domain::rows::errors::TableModelError;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use {ryo_values::domain::files::types::SourceView};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Relation, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, Test};
use ryo_values::domain::plans::types::Operation;

fn numbers(values: &[i64]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    for value in values {
        push_row(&mut table, &[Value::Number(Integer(*value))]);
    }
    table
}
fn antijoin(
    left: &Table,
    right: Option<&Table>,
    left_column: u32,
) -> Result<Table, OperatingError> {
    let operation = Operation::Antijoin {
        left: Input::Relation(Relation::Function),
        right: Input::Relation(Relation::Function),
        left_on: vec![ColumnIndex(left_column)],
        right_on: vec![ColumnIndex(0)],
    };
    let mut inputs = vec![Rows::Computed(left)];
    if let Some(right) = right {
        inputs.push(Rows::Computed(right));
    }
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    let mut out = Table::default();
    AntijoinItem.compute(&operation, &inputs, &mut context, &mut out)?;
    Ok(out)
}
#[test]
fn a_test_belongs_to_its_family() {
    assert_eq!(family_of(Test::NotGlobAny), Family::Glob);
}
#[test]
fn the_first_and_second_input_are_consumed_by_the_common_engine() {
    // The two different RHS populations change output: this is not just a metadata check.
    let left = numbers(&[2]);
    assert_eq!(
        antijoin(&left, Some(&numbers(&[1, 2])), 0).unwrap().count.0,
        0
    );
    assert_eq!(
        antijoin(&left, Some(&numbers(&[1])), 0).unwrap().cells,
        left.cells
    );
}
#[test]
fn an_input_the_real_operator_does_not_have_is_missing() {
    // No retained primitive has three inputs. The obsolete third() wrapper's
    // missing-input scenario now exercises the real binary contract instead.
    assert!(matches!(
        antijoin(&numbers(&[2]), None, 0),
        Err(OperatingError::MissingInput { position: 2 })
    ));
}
#[test]
fn a_key_the_right_holds_is_not_absent() {
    assert_eq!(
        antijoin(&numbers(&[2]), Some(&numbers(&[1, 2])), 0)
            .unwrap()
            .count
            .0,
        0
    );
}
#[test]
fn a_key_the_right_lacks_is_absent() {
    let left = numbers(&[9]);
    assert_eq!(
        antijoin(&left, Some(&numbers(&[1, 2])), 0).unwrap().cells,
        left.cells
    );
}
#[test]
fn an_antijoin_row_without_its_key_column_is_refused() {
    assert!(matches!(
        antijoin(&numbers(&[1]), Some(&numbers(&[1, 2])), 4),
        Err(OperatingError::Table(TableModelError::NoSuchColumn {
            index: 4
        }))
    ));
}

use ryo_values::domain::rows::types::Table;
