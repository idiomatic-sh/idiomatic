//! The text a literal replacement is made of.
/// The authored text each match is replaced with.
#[derive(Clone, Copy, Debug)]
pub struct ReplacementText<'a>(pub &'a str);

/// The engine-owned scratch the replaced text is built in.
pub struct ReplacedText<'a>(pub &'a mut String);

/// What a replace lowers to: the text column, the pattern and the replacement.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ReplaceParts {
    /// The text column.
    pub(crate) text: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The pattern, compiled once per worker at its first row.
    pub(crate) pattern: ryo_values::domain::plan_parts::types::LiteralText,
    /// The replacement.
    pub(crate) replacement: ryo_values::domain::plan_parts::types::LiteralText,
}
