//! The first matching layer is the rank; unmatched text has no row.
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::LiteralText;
pub(crate) fn rank(text: &str, layers: &[Vec<LiteralText>]) -> Option<Integer> {
    layers
        .iter()
        .position(|set| set.iter().any(|each| each.0.as_ref() == text))
        .map(|place| Integer(i64::try_from(place).unwrap_or(i64::MAX)))
}
