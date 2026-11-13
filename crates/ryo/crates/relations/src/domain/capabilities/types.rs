//! The types of the `capabilities` concept: the opt-in semantic promises a language declares beyond the required relation baseline.

/// Opt-in semantic promises beyond the required language relation baseline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OptionalCapability {
    /// Project-established importable module identity.
    ModuleIdentity,
    /// Project membership of an imported root under declared search roots.
    ProjectNamespaceMembership,
    /// Canonical language identifier spellings.
    Identifier,
    /// Static qualified import and builtin provenance.
    QualifiedName,
    /// Explicit parsed expressions embedded in source carriers.
    EmbeddedExpression,
    /// Collision-safe structural classes of parsed expression occurrences.
    ExpressionShape,
    /// Ordered static name events within explicit evaluation phases.
    BindingEvent,
    /// Static reference classification; definite absence, possible absence and unsupported knowledge are distinct.
    BindingRead,
    /// Known real declaration candidates for a semantic reference; unresolved references have no fake target edge.
    BindingUse,
    /// Concrete replacement and lexical-shadow associations between distinct declaration occurrences.
    BindingShadow,
    /// Canonical builtin catalog without requiring a matching source occurrence.
    ImplicitSymbol,
    /// Parsed primitive category and narrowly defined zero/empty/true predicates.
    LiteralProperties,
    /// Parser-decoded literal fragments belonging to interpolated strings.
    InterpolatedStringLiteral,
    /// Decoded byte-string literals in canonical lowercase hexadecimal.
    BytesLiteral,
    /// Standard mini-language observations for decoded format specifications.
    FormatSpecification,
    /// Parser target version attached to each source file.
    LanguageTarget,
    /// Positive standard-library membership for imported modules at the selected language target.
    StandardLibraryImport,
    /// Explicit known file-wide compiler-language feature flags.
    CompilerFeature,
    /// Collision-checked keys for supported parsed constant expressions.
    ConstantKey,
    /// Parsed percent-format strings and their format fields.
    FormatParse,
    /// Parsed percent-format strings and their format fields.
    FormatField,
    /// Parsed brace-format strings, kept separate from percent-format demand.
    BraceFormatParse,
    /// Ordered brace-format fields, including nested format-spec fields.
    BraceFormatField,
    /// Basic structured terminal outcomes of function bodies.
    FunctionTerminal,
    /// Basic parser-derived types of individual expressions.
    ExpressionType,
    /// Per-token raw percent-format parser outcomes.
    RawFormatSegment,
    /// Ordered conversion fields from individual successful raw-token parses.
    RawFormatField,
    /// Syntactic parse outcomes of original string carriers.
    LiteralExpressionParse,
    /// Parser-decoded integer or binary64 literal payloads.
    NumericLiteral,
    /// Physical native lexer-token successor and matching-delimiter edges.
    TokenLink,
}
