//! Pure recognition of a total first-literal equality.
use crate::application::file_dag::derived::owner::selection::types::{
    FirstLiteral, LiteralSpelling,
};
use crate::domain::stored_selection::types::Selector;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{Operand, QueryLiteral, Test};
/// Only a total, authored, nonempty text equality in the first predicate can
/// reject rows before any caller-owned comparison or error origin is visited.
pub(in crate::application::file_dag::derived::owner::selection) fn first_literal<'a>(
    selector: &'a Selector,
    kinds: &[CellKind],
) -> Option<FirstLiteral<'a>> {
    let comparison = selector.identity.comparisons.first()?;
    if comparison.test != Test::Equal {
        return None;
    }
    let (column, text) = match (&comparison.left, &comparison.right) {
        (Operand::Column(column), Operand::Literal(QueryLiteral::Text(text)))
        | (Operand::Literal(QueryLiteral::Text(text)), Operand::Column(column)) => {
            (column, &text.0)
        }
        _ => return None,
    };
    (kinds.get(column.0 as usize) == Some(&CellKind::Symbol) && !text.is_empty()).then_some(
        FirstLiteral {
            column: *column,
            text: LiteralSpelling(text.as_ref()),
        },
    )
}
