//! The names of the parse contract.
use ryo::languages;

/// The optional capabilities the python language promises: the relations of its files' trees and those of its project.
pub(crate) const CAPABILITIES: &[languages::OptionalCapability] = &[
    languages::OptionalCapability::BindingEvent,
    languages::OptionalCapability::BindingRead,
    languages::OptionalCapability::BindingUse,
    languages::OptionalCapability::BindingResolution,
    languages::OptionalCapability::LookupScope,
    languages::OptionalCapability::BindingShadow,
    languages::OptionalCapability::ImplicitSymbol,
    languages::OptionalCapability::QualifiedName,
    languages::OptionalCapability::Identifier,
    languages::OptionalCapability::ExpressionShape,
    languages::OptionalCapability::LiteralProperties,
    languages::OptionalCapability::FunctionTerminal,
    languages::OptionalCapability::ExpressionType,
    languages::OptionalCapability::InterpolatedStringLiteral,
    languages::OptionalCapability::LanguageTarget,
    languages::OptionalCapability::CompilerFeature,
    languages::OptionalCapability::ConstantKey,
    languages::OptionalCapability::FormatParse,
    languages::OptionalCapability::FormatField,
    languages::OptionalCapability::BraceFormatParse,
    languages::OptionalCapability::BraceFormatField,
    languages::OptionalCapability::RawFormatSegment,
    languages::OptionalCapability::RawFormatField,
    languages::OptionalCapability::LiteralExpressionParse,
    languages::OptionalCapability::StandardLibraryImport,
    languages::OptionalCapability::BytesLiteral,
    languages::OptionalCapability::FormatSpecification,
    languages::OptionalCapability::EmbeddedExpression,
    languages::OptionalCapability::NumericLiteral,
    languages::OptionalCapability::TokenLink,
    languages::OptionalCapability::AttributePath,
    languages::OptionalCapability::ModuleIdentity,
];
/// The extension of a stub file.
pub(crate) const STUB_EXTENSION: &str = "pyi";
