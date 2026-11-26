//! Unicode word splitting with the original case-boundary and digit policy.
pub use ryo_values::domain::words::service::split;
/// The one lowering of a word split: its text column, for the `text_map` kernel.
pub(crate) fn lower(
    text: ryo_values::domain::plan_parts::types::ColumnIndex,
) -> super::types::WordsParts {
    super::types::WordsParts { text }
}
