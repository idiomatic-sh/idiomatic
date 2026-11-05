//! Text vocabulary at the Python type-fact boundary.
use super::types::BasicType;
/// Unary logical negation spelling.
pub(super) const NOT: &str = "not";
/// Decode the closed literal-provider vocabulary, retaining its unrecognized label.
pub(super) fn literal_type(name: ryo::languages::NameText<'_>) -> BasicType {
    match name.0 {
        "boolean" => BasicType::Boolean,
        "integer" => BasicType::Integer,
        "float" => BasicType::Float,
        "complex" => BasicType::Complex,
        "string" => BasicType::String,
        "bytes" => BasicType::Bytes,
        "null" => BasicType::Null,
        "ellipsis" => BasicType::Ellipsis,
        _ => BasicType::Unrecognized,
    }
}
/// Name emitted by the expression-type relation; strings do not flow through inference.
pub(crate) fn name(kind: BasicType) -> ryo::languages::NameText<'static> {
    ryo::languages::NameText(match kind {
        BasicType::Boolean => "boolean",
        BasicType::Integer => "integer",
        BasicType::Float => "float",
        BasicType::Complex => "complex",
        BasicType::String => "string",
        BasicType::Bytes => "bytes",
        BasicType::List => "list",
        BasicType::Tuple => "tuple",
        BasicType::Set => "set",
        BasicType::Dict => "dict",
        BasicType::Generator => "generator",
        BasicType::Null => "null",
        BasicType::Ellipsis => "ellipsis",
        BasicType::Unrecognized => "unknown",
    })
}
