//! The `parse` contract's answers to the relations of ryo's catalog declared `optional`: for each, how the language reads it
//! or that it is not in the language yet. `Parse::semantics` hands this implementation to the engine.

use ryo::languages::{self, AbsenceCause, Reader, Support};

use crate::domain::model::types::Declarations;
use crate::domain::relations::collections::relations::{
    binding_event::item::BindingEventItem, binding_read::item::BindingReadItem,
    binding_shadow::item::BindingShadowItem, binding_use::item::BindingUseItem,
    bytes_literal::item::BytesLiteralItem, compiler_feature::item::CompilerFeatureItem,
    constant_key::item::ConstantKeyItem, embedded_expression::item::EmbeddedExpressionItem,
    expression_shape::item::ExpressionShapeItem, expression_type::item::ExpressionTypeItem,
    function_terminal::item::FunctionTerminalItem, identifier::item::IdentifierItem,
    implicit_symbol::item::ImplicitSymbolItem,
    interpolated_string_literal::item::InterpolatedStringLiteralItem,
    language_target::item::LanguageTargetItem, literal_properties::item::LiteralPropertiesItem,
    numeric_literal::item::NumericLiteralItem, qualified_name::item::QualifiedNameItem,
    standard_library_import::item::StandardLibraryImportItem,
};

use super::contract::ParseContract;

impl languages::Semantics<Declarations> for ParseContract {
    fn embedded_expression(&self) -> Support<Reader<languages::EmbeddedExpression, Declarations>> {
        languages::reads::<EmbeddedExpressionItem>()
    }

    fn expression_shape(&self) -> Support<Reader<languages::ExpressionShape, Declarations>> {
        languages::reads::<ExpressionShapeItem>()
    }

    fn constant_key(&self) -> Support<Reader<languages::ConstantKey, Declarations>> {
        languages::reads::<ConstantKeyItem>()
    }

    fn format_parse(&self) -> Support<Reader<languages::FormatParse, Declarations>> {
        languages::reads::<super::readers::format::composition::FormatParseItem>()
    }

    fn literal_expression_parse(
        &self,
    ) -> Support<Reader<languages::LiteralExpressionParse, Declarations>> {
        languages::reads::<
            super::readers::literal_expression::composition::LiteralExpressionParseItem,
        >()
    }

    fn raw_format_segment(&self) -> Support<Reader<languages::RawFormatSegment, Declarations>> {
        languages::reads::<super::readers::raw_format::composition::RawFormatSegmentItem>()
    }

    fn token_link(&self) -> Support<Reader<languages::TokenLink, Declarations>> {
        languages::reads::<super::readers::token_link::composition::TokenLinkItem>()
    }

    fn raw_format_field(&self) -> Support<Reader<languages::RawFormatField, Declarations>> {
        languages::reads::<super::readers::raw_format::composition::RawFormatFieldItem>()
    }

    fn format_field(&self) -> Support<Reader<languages::FormatField, Declarations>> {
        languages::reads::<super::readers::format::composition::FormatFieldItem>()
    }

    fn brace_format_parse(&self) -> Support<Reader<languages::BraceFormatParse, Declarations>> {
        languages::reads::<super::readers::brace_format::composition::BraceFormatParseItem>()
    }

    fn brace_format_field(&self) -> Support<Reader<languages::BraceFormatField, Declarations>> {
        languages::reads::<super::readers::brace_format::composition::BraceFormatFieldItem>()
    }

    fn function_terminal(&self) -> Support<Reader<languages::FunctionTerminal, Declarations>> {
        languages::reads::<FunctionTerminalItem>()
    }

    fn expression_type(&self) -> Support<Reader<languages::ExpressionType, Declarations>> {
        languages::reads::<ExpressionTypeItem>()
    }

    fn literal_properties(&self) -> Support<Reader<languages::LiteralProperties, Declarations>> {
        languages::reads::<LiteralPropertiesItem>()
    }

    fn numeric_literal(&self) -> Support<Reader<languages::NumericLiteral, Declarations>> {
        languages::reads::<NumericLiteralItem>()
    }

    fn format_specification(
        &self,
    ) -> Support<Reader<languages::FormatSpecification, Declarations>> {
        languages::reads::<super::readers::format_specification::composition::FormatSpecificationItem>(
        )
    }

    fn bytes_literal(&self) -> Support<Reader<languages::BytesLiteral, Declarations>> {
        languages::reads::<BytesLiteralItem>()
    }

    fn interpolated_string_literal(
        &self,
    ) -> Support<Reader<languages::InterpolatedStringLiteral, Declarations>> {
        languages::reads::<InterpolatedStringLiteralItem>()
    }

    fn compiler_feature(&self) -> Support<Reader<languages::CompilerFeature, Declarations>> {
        languages::reads::<CompilerFeatureItem>()
    }

    fn language_target(&self) -> Support<Reader<languages::LanguageTarget, Declarations>> {
        languages::reads::<LanguageTargetItem>()
    }

    fn implicit_symbol(&self) -> Support<Reader<languages::ImplicitSymbol, Declarations>> {
        languages::reads::<ImplicitSymbolItem>()
    }

    fn binding_event(&self) -> Support<Reader<languages::BindingEvent, Declarations>> {
        languages::reads::<BindingEventItem>()
    }

    fn binding_read(&self) -> Support<Reader<languages::BindingRead, Declarations>> {
        languages::reads::<BindingReadItem>()
    }

    fn binding_use(&self) -> Support<Reader<languages::BindingUse, Declarations>> {
        languages::reads::<BindingUseItem>()
    }

    fn binding_shadow(&self) -> Support<Reader<languages::BindingShadow, Declarations>> {
        languages::reads::<BindingShadowItem>()
    }

    fn identifier(&self) -> Support<Reader<languages::Identifier, Declarations>> {
        languages::reads::<IdentifierItem>()
    }

    fn qualified_name(&self) -> Support<Reader<languages::QualifiedName, Declarations>> {
        languages::reads::<QualifiedNameItem>()
    }

    fn standard_library_import(
        &self,
    ) -> Support<Reader<languages::StandardLibraryImport, Declarations>> {
        languages::reads::<StandardLibraryImportItem>()
    }

    fn pattern_parse(&self) -> Support<Reader<languages::PatternParse, Declarations>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn pattern_syntax(&self) -> Support<Reader<languages::PatternSyntax, Declarations>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn pattern_child(&self) -> Support<Reader<languages::PatternChild, Declarations>> {
        languages::absent(AbsenceCause::NotYetRead)
    }
}
