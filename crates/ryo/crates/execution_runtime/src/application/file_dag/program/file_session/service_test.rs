//! The planner and the file session agree on which numbers are stored: the recogniser classifies an ordered existence
//! only over a number the session hands over as stored columns, which the runtime's number admission requires.
use super::service::source_rows;
use super::types::SourceRows;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::self_join_recogniser::fixtures::proves_stored_number;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::service::{all, kinds};
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FileId, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[test]
fn the_recogniser_proves_a_number_exactly_when_the_session_hands_it_over_stored() {
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut numbered = 0_usize;
    for relation in all() {
        let schema = kinds(*relation);
        let Some(number) = schema.iter().position(|kind| *kind == CellKind::Number) else {
            continue;
        };
        numbered += 1;
        let read = |_: Relation| -> Result<SourceRows<'_>, OperatingError> {
            Ok(SourceRows::Stored(ColumnsView {
                kinds: schema,
                columns: &[],
            }))
        };
        let stored = matches!(
            source_rows(&read, &elements, FileId(0), *relation),
            Ok(Rows::Stored(_))
        );
        assert_eq!(
            proves_stored_number(*relation, ColumnIndex(u32::try_from(number).unwrap())),
            stored,
            "{relation:?}: the recogniser and the file session disagree on a stored number"
        );
    }
    assert!(
        numbered > 1,
        "the catalog holds native and stored numbers alike"
    );
}
