//! The first matching layer is the rank; unmatched text has no row.
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::row_texts::types::CellText;
/// The zero-based first layer containing this text, or no rank when it occurs in none.
pub fn rank(text: CellText<'_>, layers: &[Vec<LiteralText>]) -> Option<Integer> {
    layers
        .iter()
        .position(|set| set.iter().any(|each| each.0.as_ref() == text.0))
        .map(|place| Integer(i64::try_from(place).unwrap_or(i64::MAX)))
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
