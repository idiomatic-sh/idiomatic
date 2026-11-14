//! The `parse` contract's answers to the relations of ryo's catalog declared `optional`: for each, how the language reads it
//! or that it is not in the language yet. `Parse::semantics` hands this implementation to the engine.

use ryo::languages::{self, AbsenceCause, Reader, Support};

use crate::domain::model::types::Model;

use super::contract::ParseContract;

impl languages::Semantics<Model> for ParseContract {
    fn embedded_expression(&self) -> Support<Reader<languages::EmbeddedExpression, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn expression_shape(&self) -> Support<Reader<languages::ExpressionShape, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn constant_key(&self) -> Support<Reader<languages::ConstantKey, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn format_parse(&self) -> Support<Reader<languages::FormatParse, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn format_field(&self) -> Support<Reader<languages::FormatField, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn literal_expression_parse(
        &self,
    ) -> Support<Reader<languages::LiteralExpressionParse, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn raw_format_segment(&self) -> Support<Reader<languages::RawFormatSegment, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn token_link(&self) -> Support<Reader<languages::TokenLink, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn raw_format_field(&self) -> Support<Reader<languages::RawFormatField, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn brace_format_parse(&self) -> Support<Reader<languages::BraceFormatParse, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn brace_format_field(&self) -> Support<Reader<languages::BraceFormatField, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn function_terminal(&self) -> Support<Reader<languages::FunctionTerminal, Model>> {
        // This provider does not yet implement this optional semantic relation.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn expression_type(&self) -> Support<Reader<languages::ExpressionType, Model>> {
        // This provider does not yet implement this optional semantic relation.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn format_specification(&self) -> Support<Reader<languages::FormatSpecification, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn bytes_literal(&self) -> Support<Reader<languages::BytesLiteral, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn interpolated_string_literal(
        &self,
    ) -> Support<Reader<languages::InterpolatedStringLiteral, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn literal_properties(&self) -> Support<Reader<languages::LiteralProperties, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn numeric_literal(&self) -> Support<Reader<languages::NumericLiteral, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn compiler_feature(&self) -> Support<Reader<languages::CompilerFeature, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn language_target(&self) -> Support<Reader<languages::LanguageTarget, Model>> {
        // This provider does not expose a parser target version.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn implicit_symbol(&self) -> Support<Reader<languages::ImplicitSymbol, Model>> {
        // This optional provider catalog is not yet implemented or advertised here.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_event(&self) -> Support<Reader<languages::BindingEvent, Model>> {
        // This optional analysis is not yet implemented or advertised by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_read(&self) -> Support<Reader<languages::BindingRead, Model>> {
        // This optional analysis is not yet implemented or advertised by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_use(&self) -> Support<Reader<languages::BindingUse, Model>> {
        // This optional analysis is not yet implemented or advertised by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_shadow(&self) -> Support<Reader<languages::BindingShadow, Model>> {
        // This optional analysis is not yet implemented or advertised by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn identifier(&self) -> Support<Reader<languages::Identifier, Model>> {
        // Data keys are not programming-language lexical identifiers.
        languages::absent(AbsenceCause::NotCode)
    }

    fn qualified_name(&self) -> Support<Reader<languages::QualifiedName, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn standard_library_import(&self) -> Support<Reader<languages::StandardLibraryImport, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn pattern_parse(&self) -> Support<Reader<languages::PatternParse, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn pattern_syntax(&self) -> Support<Reader<languages::PatternSyntax, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn pattern_child(&self) -> Support<Reader<languages::PatternChild, Model>> {
        languages::absent(AbsenceCause::NotYetRead)
    }
}
