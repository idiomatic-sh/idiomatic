//! The names of the parse contract.
use ryo::languages;

/// The optional capabilities the python language answers.
pub(crate) const CAPABILITIES: &[languages::OptionalCapability] = &[
    languages::OptionalCapability::ModuleIdentity,
    languages::OptionalCapability::Identifier,
    languages::OptionalCapability::QualifiedName,
    languages::OptionalCapability::EmbeddedExpression,
    languages::OptionalCapability::ExpressionShape,
    languages::OptionalCapability::BindingEvent,
    languages::OptionalCapability::BindingRead,
    languages::OptionalCapability::BindingUse,
    languages::OptionalCapability::BindingShadow,
    languages::OptionalCapability::ImplicitSymbol,
    languages::OptionalCapability::LiteralProperties,
    languages::OptionalCapability::NumericLiteral,
    languages::OptionalCapability::InterpolatedStringLiteral,
    languages::OptionalCapability::BytesLiteral,
    languages::OptionalCapability::FormatSpecification,
    languages::OptionalCapability::ConstantKey,
    languages::OptionalCapability::FormatParse,
    languages::OptionalCapability::FormatField,
    languages::OptionalCapability::RawFormatSegment,
    languages::OptionalCapability::RawFormatField,
    languages::OptionalCapability::TokenLink,
    languages::OptionalCapability::LiteralExpressionParse,
    languages::OptionalCapability::BraceFormatParse,
    languages::OptionalCapability::BraceFormatField,
    languages::OptionalCapability::LanguageTarget,
    languages::OptionalCapability::StandardLibraryImport,
    languages::OptionalCapability::CompilerFeature,
    languages::OptionalCapability::FunctionTerminal,
    languages::OptionalCapability::ExpressionType,
];
/// The extension of a stub file.
pub(crate) const STUB_EXTENSION: &str = "pyi";
