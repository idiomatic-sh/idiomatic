//! Pure immutable selector fixture shared by domain and application tests.
use crate::domain::stored_selection::types::Selector;
use crate::domain::table_model::types::ParameterBinding;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Operand, ParameterId, Test, WholeNumber,
};
use std::sync::Arc;
/// A predicate and a duplicate output map, with only its referenced bound value.
pub(crate) fn program(value: i64) -> Arc<Selector> {
    Arc::new(Selector {
        relation: Relation::Function,
        identity: Arc::new(crate::domain::table_model::types::SelectionIdentity {
            comparisons: vec![Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Parameter(ParameterId(0)),
                origin: None,
            }],
            bindings: vec![ParameterBinding::Number(WholeNumber(value))],
        }),
        columns: vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
        distinct: false,
    })
}
