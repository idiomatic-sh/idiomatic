//! The `parse` contract's answers to the relations of ryo's catalog declared `optional`: for each, how the language reads it
//! or that it is not in the language yet. `Parse::semantics` hands this implementation to the engine.

use ryo::languages::{self, AbsenceCause, Reader, Support};

use crate::domain::model::types::Sections;

use super::contract::ParseContract;

impl languages::Semantics<Sections> for ParseContract {
    fn embedded_expression(&self) -> Support<Reader<languages::EmbeddedExpression, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn expression_shape(&self) -> Support<Reader<languages::ExpressionShape, Sections>> {
        // Parsed expression identity is not yet implemented by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn constant_key(&self) -> Support<Reader<languages::ConstantKey, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn format_parse(&self) -> Support<Reader<languages::FormatParse, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn format_field(&self) -> Support<Reader<languages::FormatField, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn literal_expression_parse(
        &self,
    ) -> Support<Reader<languages::LiteralExpressionParse, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn raw_format_segment(&self) -> Support<Reader<languages::RawFormatSegment, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn token_link(&self) -> Support<Reader<languages::TokenLink, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn raw_format_field(&self) -> Support<Reader<languages::RawFormatField, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn brace_format_parse(&self) -> Support<Reader<languages::BraceFormatParse, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn brace_format_field(&self) -> Support<Reader<languages::BraceFormatField, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn function_terminal(&self) -> Support<Reader<languages::FunctionTerminal, Sections>> {
        // This provider does not yet implement this optional semantic relation.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn expression_type(&self) -> Support<Reader<languages::ExpressionType, Sections>> {
        // This provider does not yet implement this optional semantic relation.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn format_specification(&self) -> Support<Reader<languages::FormatSpecification, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn bytes_literal(&self) -> Support<Reader<languages::BytesLiteral, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn interpolated_string_literal(
        &self,
    ) -> Support<Reader<languages::InterpolatedStringLiteral, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn literal_properties(&self) -> Support<Reader<languages::LiteralProperties, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn numeric_literal(&self) -> Support<Reader<languages::NumericLiteral, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn compiler_feature(&self) -> Support<Reader<languages::CompilerFeature, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn language_target(&self) -> Support<Reader<languages::LanguageTarget, Sections>> {
        // This provider does not expose a parser target version.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn implicit_symbol(&self) -> Support<Reader<languages::ImplicitSymbol, Sections>> {
        // This optional provider catalog is not yet implemented or advertised here.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_event(&self) -> Support<Reader<languages::BindingEvent, Sections>> {
        // This optional analysis is not yet implemented or advertised by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_read(&self) -> Support<Reader<languages::BindingRead, Sections>> {
        // This optional analysis is not yet implemented or advertised by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_use(&self) -> Support<Reader<languages::BindingUse, Sections>> {
        // This optional analysis is not yet implemented or advertised by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_shadow(&self) -> Support<Reader<languages::BindingShadow, Sections>> {
        // This optional analysis is not yet implemented or advertised by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn identifier(&self) -> Support<Reader<languages::Identifier, Sections>> {
        // Canonical identifier spelling is not yet exposed by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn qualified_name(&self) -> Support<Reader<languages::QualifiedName, Sections>> {
        // standard: CommonMark 0.31.2, Blocks and inlines, https://spec.commonmark.org/0.31.2/#blocks-and-inlines defines document/link structure, not lexical import or builtin namespaces; embedded code is read by its own language.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn standard_library_import(
        &self,
    ) -> Support<Reader<languages::StandardLibraryImport, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn pattern_parse(&self) -> Support<Reader<languages::PatternParse, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn pattern_syntax(&self) -> Support<Reader<languages::PatternSyntax, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn pattern_child(&self) -> Support<Reader<languages::PatternChild, Sections>> {
        languages::absent(AbsenceCause::NotYetRead)
    }
}
