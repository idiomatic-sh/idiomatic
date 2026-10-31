//! Language typing forms relevant to the context of quoted expressions.
/// Resolved standard-library typing forms, distinct from names with the same spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TypingForm {
    /// Arguments are literal values rather than type expressions.
    Literal,
    /// Only the first argument is a type; remaining arguments are metadata.
    Annotated,
    /// Its first positional or `typ` argument is a type expression.
    Cast,
    /// Its second positional or `tp` argument is a type expression.
    NewType,
    /// Constraints, bounds, and defaults are type expressions; the name is a value.
    TypeVar,
    /// An explicit annotation marking a type alias value.
    TypeAlias,
    /// A local, unknown, or unrelated imported object.
    Other,
}

/// Membership in the explicit semantic syntax projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Visibility {
    /// Native syntax or a parsed type expression.
    Visible,
    /// A parsed candidate in a value-only context.
    Hidden,
}
/// Whether a syntactic operand carries a type expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TypeContext {
    /// Quoted text is parsed as a type expression.
    Type,
    /// Quoted text remains an ordinary runtime value.
    Value,
}
