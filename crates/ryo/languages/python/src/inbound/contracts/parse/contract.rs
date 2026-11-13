//! The `parse` contract: it implements the contract another crate declares, with nothing of its own beside it. It says which files
//! are Python and builds, completes and answers, for every syntax relation of ryo's catalog, how Python reads it or that it is not in the language.

use super::constants::CAPABILITIES;
use super::service::{begin_file, complete_file, prepare_model, read_file};
use crate::domain::language::constants::UNKNOWN_REASONS;
use crate::domain::language::service::reads;
use crate::domain::model::types::Declarations;
use crate::domain::relations::collections::relations::{
    annotates::item::AnnotatesItem, assigns::item::AssignsItem,
    binding_event::item::BindingEventItem, binding_read::item::BindingReadItem,
    binding_shadow::item::BindingShadowItem, binding_use::item::BindingUseItem,
    bytes_literal::item::BytesLiteralItem, call_argument::item::CallArgumentItem,
    comment::item::CommentItem, compiler_feature::item::CompilerFeatureItem,
    constant_key::item::ConstantKeyItem, decorates::item::DecoratesItem,
    defines::item::DefinesItem, embedded_expression::item::EmbeddedExpressionItem,
    expression_shape::item::ExpressionShapeItem, expression_type::item::ExpressionTypeItem,
    function::item::FunctionItem, function_terminal::item::FunctionTerminalItem,
    identifier::item::IdentifierItem, implicit_symbol::item::ImplicitSymbolItem,
    indent_region::item::IndentRegionItem, inherits::item::InheritsItem,
    interpolated_string_literal::item::InterpolatedStringLiteralItem,
    language_target::item::LanguageTargetItem, literal_properties::item::LiteralPropertiesItem,
    numeric_literal::item::NumericLiteralItem, parameter::item::ParameterItem,
    qualified_name::item::QualifiedNameItem, reference::item::ReferenceItem,
    resolves::item::ResolvesItem, scope::item::ScopeItem, semantic_child::item::SemanticChildItem,
    semantic_syntax::item::SemanticSyntaxItem,
    standard_library_import::item::StandardLibraryImportItem, static_call::item::StaticCallItem,
    static_import::item::StaticImportItem, string_literal::item::StringLiteralItem,
    syntax_child::item::SyntaxChildItem, syntax_error::item::SyntaxErrorItem,
    syntax_tree::item::SyntaxTreeItem,
};
use crate::domain::syntax::service::{kind_name, kinds};
use crate::outbound::{notes::interface::NotesPort, syntax::interface::SyntaxPort};
use crate::settings::PythonParserSettings;
use ryo::languages::{self, AbsenceCause, Parse, Reader, Support, UnknownName};
use std::sync::Arc;

/// `parse`, implementing `ryo::languages::Parse`.
pub(crate) struct ParseContract {
    pub(super) settings: PythonParserSettings,
    pub(super) syntax: Arc<dyn SyntaxPort>,
    pub(super) notes: Arc<dyn NotesPort>,
    #[cfg(test)]
    pub(super) model_preparations: std::sync::atomic::AtomicUsize,
}

impl ParseContract {
    /// `parse` over the ports it calls, which the edge injects.
    #[must_use]
    pub(crate) fn new(
        settings: PythonParserSettings,
        syntax: Arc<dyn SyntaxPort>,
        notes: Arc<dyn NotesPort>,
    ) -> Self {
        Self {
            settings,
            syntax,
            notes,
            #[cfg(test)]
            model_preparations: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl Parse for ParseContract {
    type FileModel = Declarations;

    fn begin_semantics(
        &self,
        request: &languages::ParseRequest<'_>,
        tree: &languages::Elements,
        slot: &mut languages::SemanticSlot,
    ) {
        begin_file(self.syntax.as_ref(), request, tree, slot);
    }

    fn semantics(&self) -> Option<&dyn languages::Semantics<Declarations>> {
        Some(self)
    }

    fn capabilities(&self) -> &'static [languages::OptionalCapability] {
        CAPABILITIES
    }

    fn configuration_identity(&self) -> languages::ParseConfigurationIdentity<'_> {
        self.settings.identity()
    }

    fn completion_requirement(&self) -> languages::CompletionRequirement {
        languages::CompletionRequirement::PreparedModel
    }

    fn extend_file_model(
        &self,
        tree: &languages::Tree<'_>,
        from: languages::Position,
        model: &mut Declarations,
    ) {
        crate::domain::bindings::service::extend(tree, from, model);
    }

    fn file_model(&self, _tree: &languages::Tree<'_>, model: &mut Declarations) {
        #[cfg(test)]
        let _previous = self
            .model_preparations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        prepare_model(&self.settings, model);
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        UNKNOWN_REASONS
    }

    fn recognizes(&self, request: &languages::ParseRequest<'_>) -> languages::Recognition {
        reads(request.path)
    }

    fn recognizes_explicit(&self, _: &languages::ParseRequest<'_>) -> languages::Recognition {
        languages::Recognition::Reads
    }

    fn skips(&self) -> &'static [languages::SkippedDirectory] {
        crate::domain::language::constants::SKIPS
    }

    fn kind_count(&self) -> languages::Count {
        languages::Count(kinds().count())
    }

    fn kind_names(&self) -> languages::KindNames {
        languages::KindNames(kinds().map(kind_name).collect())
    }

    fn read_tree(
        &self,
        request: &languages::ParseRequest<'_>,
        builder: &mut languages::TableBuilder<'_>,
    ) -> Result<(), languages::ParseError> {
        read_file(self.syntax.as_ref(), request, builder);
        Ok(())
    }

    fn complete(
        &self,
        request: &languages::ParseRequest<'_>,
        completion: &mut languages::Completion<'_>,
    ) {
        complete_file(
            self.syntax.as_ref(),
            self.notes.as_ref(),
            request,
            completion,
        );
    }

    fn annotates(&self) -> Support<Reader<languages::Annotates, Declarations>> {
        languages::reads::<AnnotatesItem>()
    }

    fn assigns(&self) -> Support<Reader<languages::Assigns, Declarations>> {
        languages::reads::<AssignsItem>()
    }

    fn comment(&self) -> Support<Reader<languages::Comment, Declarations>> {
        languages::reads::<CommentItem>()
    }

    fn directive(&self) -> Support<Reader<languages::Directive, Declarations>> {
        // standard: The Python Language Reference, section 2 (Lexical analysis): a comment is no instruction to a build tool; the encoding declaration is read by the interpreter.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn decorates(&self) -> Support<Reader<languages::Decorates, Declarations>> {
        languages::reads::<DecoratesItem>()
    }

    fn defines(&self) -> Support<Reader<languages::Defines, Declarations>> {
        languages::reads::<DefinesItem>()
    }

    fn visibility(&self) -> Support<Reader<languages::Visibility, Declarations>> {
        // standard: The Python Language Reference, sections 4.2 (Naming and binding) and 7.11 (The import statement): no syntax writes a visibility; a leading underscore is a naming convention, and `__all__` is an ordinary assignment.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn function(&self) -> Support<Reader<languages::Function, Declarations>> {
        languages::reads::<FunctionItem>()
    }

    fn inherits(&self) -> Support<Reader<languages::Inherits, Declarations>> {
        languages::reads::<InheritsItem>()
    }

    fn parameter(&self) -> Support<Reader<languages::Parameter, Declarations>> {
        languages::reads::<ParameterItem>()
    }

    fn reference(&self) -> Support<Reader<languages::Reference, Declarations>> {
        languages::reads::<ReferenceItem>()
    }

    fn scope(&self) -> Support<Reader<languages::Scope, Declarations>> {
        languages::reads::<ScopeItem>()
    }

    fn static_call(&self) -> Support<Reader<languages::StaticCall, Declarations>> {
        languages::reads::<StaticCallItem>()
    }

    fn semantic_syntax(&self) -> Support<Reader<languages::SemanticSyntax, Self::FileModel>> {
        languages::reads::<SemanticSyntaxItem>()
    }

    fn semantic_child(&self) -> Support<Reader<languages::SemanticChild, Self::FileModel>> {
        languages::reads::<SemanticChildItem>()
    }

    fn resolves(&self) -> Support<Reader<languages::Resolves, Declarations>> {
        languages::reads::<ResolvesItem>()
    }

    fn call_argument(&self) -> Support<Reader<languages::CallArgument, Declarations>> {
        languages::reads::<CallArgumentItem>()
    }

    fn static_import(&self) -> Support<Reader<languages::StaticImport, Declarations>> {
        languages::reads::<StaticImportItem>()
    }

    fn string_literal(&self) -> Support<Reader<languages::StringLiteral, Declarations>> {
        languages::reads::<StringLiteralItem>()
    }

    fn prose(&self) -> Support<Reader<languages::Prose, Declarations>> {
        languages::absent(AbsenceCause::NotADocument)
    }

    fn syntax_child(&self) -> Support<Reader<languages::SyntaxChild, Declarations>> {
        languages::reads::<SyntaxChildItem>()
    }

    fn syntax_error(&self) -> Support<Reader<languages::SyntaxError, Declarations>> {
        languages::reads::<SyntaxErrorItem>()
    }

    fn syntax_tree(&self) -> Support<Reader<languages::SyntaxTree, Declarations>> {
        languages::reads::<SyntaxTreeItem>()
    }

    fn source_location(&self) -> Support<Reader<languages::SourceLocation, Declarations>> {
        languages::source_locations()
    }

    fn indent_region(&self) -> Support<Reader<languages::IndentRegion, Declarations>> {
        // Lexical INDENT/DEDENT tokens define this relation, not visual whitespace alone.
        languages::reads::<IndentRegionItem>()
    }

    fn mention(&self) -> Support<Reader<languages::Mention, Declarations>> {
        // standard: The Python Language Reference, section 2 (Lexical analysis) and sections 6 to 8 (expressions and statements): Python has no macro call and no token tree to write a name inside of.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn mapping(&self) -> Support<Reader<languages::Mapping, Declarations>> {
        languages::absent(AbsenceCause::NotAKeyedDocument)
    }

    fn entry(&self) -> Support<Reader<languages::Entry, Declarations>> {
        languages::absent(AbsenceCause::NotAKeyedDocument)
    }

    fn sequence_element(&self) -> Support<Reader<languages::SequenceElement, Declarations>> {
        languages::absent(AbsenceCause::NotAKeyedDocument)
    }
}

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
