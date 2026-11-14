//! The `parse` contract's answers to the relations of ryo's catalog declared `optional`: for each, how the language reads it
//! or that it is not in the language yet. `Parse::semantics` hands this implementation to the engine.

use ryo::languages::{self, AbsenceCause, Reader, Support};

use crate::domain::model::types::ReadIndexes;
use crate::domain::relations::collections::relations::{
    binding_read::item::BindingReadItem, binding_use::item::BindingUseItem,
    expression_shape::item::ExpressionShapeItem, numeric_literal::item::NumericLiteralItem,
    pattern_child::item::PatternChildItem, pattern_parse::item::PatternParseItem,
    pattern_syntax::item::PatternSyntaxItem,
};

use super::contract::ParseContract;

impl languages::Semantics<ReadIndexes> for ParseContract {
    fn embedded_expression(&self) -> Support<Reader<languages::EmbeddedExpression, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn expression_shape(&self) -> Support<Reader<languages::ExpressionShape, ReadIndexes>> {
        languages::reads::<ExpressionShapeItem>()
    }

    fn constant_key(&self) -> Support<Reader<languages::ConstantKey, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn format_parse(&self) -> Support<Reader<languages::FormatParse, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn format_field(&self) -> Support<Reader<languages::FormatField, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn literal_expression_parse(
        &self,
    ) -> Support<Reader<languages::LiteralExpressionParse, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn raw_format_segment(&self) -> Support<Reader<languages::RawFormatSegment, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn token_link(&self) -> Support<Reader<languages::TokenLink, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn raw_format_field(&self) -> Support<Reader<languages::RawFormatField, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn brace_format_parse(&self) -> Support<Reader<languages::BraceFormatParse, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn brace_format_field(&self) -> Support<Reader<languages::BraceFormatField, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn function_terminal(&self) -> Support<Reader<languages::FunctionTerminal, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn expression_type(&self) -> Support<Reader<languages::ExpressionType, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn format_specification(&self) -> Support<Reader<languages::FormatSpecification, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn bytes_literal(&self) -> Support<Reader<languages::BytesLiteral, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn interpolated_string_literal(
        &self,
    ) -> Support<Reader<languages::InterpolatedStringLiteral, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn literal_properties(&self) -> Support<Reader<languages::LiteralProperties, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn numeric_literal(&self) -> Support<Reader<languages::NumericLiteral, ReadIndexes>> {
        languages::reads::<NumericLiteralItem>()
    }

    fn compiler_feature(&self) -> Support<Reader<languages::CompilerFeature, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn language_target(&self) -> Support<Reader<languages::LanguageTarget, ReadIndexes>> {
        // This provider does not expose a parser target version.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn implicit_symbol(&self) -> Support<Reader<languages::ImplicitSymbol, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_event(&self) -> Support<Reader<languages::BindingEvent, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_read(&self) -> Support<Reader<languages::BindingRead, ReadIndexes>> {
        languages::reads::<BindingReadItem>()
    }

    fn binding_use(&self) -> Support<Reader<languages::BindingUse, ReadIndexes>> {
        languages::reads::<BindingUseItem>()
    }

    fn binding_shadow(&self) -> Support<Reader<languages::BindingShadow, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn identifier(&self) -> Support<Reader<languages::Identifier, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn qualified_name(&self) -> Support<Reader<languages::QualifiedName, ReadIndexes>> {
        // Qualified source identities are applicable, but this provider has not implemented their reader.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn standard_library_import(
        &self,
    ) -> Support<Reader<languages::StandardLibraryImport, ReadIndexes>> {
        // TypeScript module imports exist; a versioned host-library catalog is not read.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn pattern_parse(&self) -> Support<Reader<languages::PatternParse, ReadIndexes>> {
        languages::reads::<PatternParseItem>()
    }

    fn pattern_syntax(&self) -> Support<Reader<languages::PatternSyntax, ReadIndexes>> {
        languages::reads::<PatternSyntaxItem>()
    }

    fn pattern_child(&self) -> Support<Reader<languages::PatternChild, ReadIndexes>> {
        languages::reads::<PatternChildItem>()
    }
}
