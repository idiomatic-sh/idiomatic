//! The first matching layer is the rank; unmatched text has no row.
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::row_texts::types::CellText;
pub(crate) fn rank(text: CellText<'_>, layers: &[Vec<LiteralText>]) -> Option<Integer> {
    layers
        .iter()
        .position(|set| set.iter().any(|each| each.0.as_ref() == text.0))
        .map(|place| Integer(i64::try_from(place).unwrap_or(i64::MAX)))
}

/// Appends the first layer the text of a scheduled row's column is in; a text in no layer, or a cell that
/// is not text, has no row.
pub(crate) fn evaluate(
    row: &[ryo_relations::domain::relations::types::Value],
    column: ryo_values::domain::plan_parts::types::ColumnIndex,
    layers: Option<&[Vec<LiteralText>]>,
    driver: &mut dyn crate::domain::contract::types::RowDriver,
    scratch: &mut crate::domain::contract::types::RowScratch,
) -> Result<(), crate::domain::contract::types::PortableOperatorFault> {
    let layers =
        layers.ok_or(crate::domain::contract::types::PortableOperatorFault::WrongBinding)?;
    let value = ryo_values::domain::rows::service::column_in(row, column)
        .copied()
        .ok_or(
            crate::domain::contract::types::PortableOperatorFault::NoSuchColumn { index: column },
        )?;
    scratch.text.0.clear();
    if driver.text(value, &mut scratch.text) == crate::domain::contract::types::TextFound::NotText {
        return Ok(());
    }
    let Some(found) = rank(CellText(&scratch.text.0), layers) else {
        return Ok(());
    };
    scratch.row.clear();
    scratch.row.extend_from_slice(row);
    scratch
        .row
        .push(ryo_relations::domain::relations::types::Value::Number(
            found,
        ));
    driver.emit(&scratch.row)
}

/// The one lowering of a rank: its text column and its layers, resolved from what its parameter is bound to; a parameter
/// with no binding, or one of another shape, is a refusal its kernel raises at the first row.
pub(crate) fn lower(
    column: ryo_values::domain::plan_parts::types::ColumnIndex,
    sets: ryo_values::domain::plan_parts::types::ParameterId,
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
) -> super::types::RankParts {
    use crate::domain::contract::types::PortableOperatorFault;
    use ryo_values::domain::query_bindings::types::ParameterBinding;
    let layers = match ryo_values::domain::plan_parts::service::parameter_in(bindings, sets) {
        Some(ParameterBinding::TextSets(layers)) => super::types::RankLayers::Bound(layers.clone()),
        Some(_) => super::types::RankLayers::Refused(PortableOperatorFault::WrongBinding),
        None => super::types::RankLayers::Refused(PortableOperatorFault::Unbound),
    };
    super::types::RankParts { column, layers }
}
