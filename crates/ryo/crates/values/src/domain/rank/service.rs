//! The first matching layer is the rank; unmatched text has no row.
use crate::domain::plan_parts::types::LiteralText;
use crate::domain::row_texts::types::CellText;
use ryo_relations::domain::relations::types::Integer;

/// The zero-based first layer containing this text, or no rank when it occurs in none.
pub fn rank(text: CellText<'_>, layers: &[Vec<LiteralText>]) -> Option<Integer> {
    layers
        .iter()
        .position(|set| set.iter().any(|each| each.0.as_ref() == text.0))
        .map(|place| Integer(i64::try_from(place).unwrap_or(i64::MAX)))
}
