//! The `parse` contract: it implements the contract another crate declares, with nothing of its
//! own beside it. It says which files are Rust, builds the file's tree in the worker's element table through a
//! builder, and answers, for every syntax relation of ryo's catalog, how Rust reads it or that it is not in the
//! language. The engine reads the tree with the readers answered here; no relation is left unanswered, and a
//! relation added to the catalog stops this from compiling until it is.

use crate::domain::fields::service::{field_bits, field_name, fields, role_of};
use crate::outbound::syntax::types::SourceText;
use ryo::languages::UnknownName;
use std::sync::Arc;

use ryo::languages::{
    self, AbsenceCause, CompletionRequirement, Elements, Parse, ParseRequest, Position, Reader,
    Recognition, SemanticSlot, Support, Tree,
};

use crate::domain::fact_extraction::service::note_of;
use crate::domain::language::service::reads;
use crate::domain::model::{service::clear, types::ReadIndexes};
use crate::domain::relations::collections::relations::{
    annotates::item::AnnotatesItem, assigns::item::AssignsItem,
    call_argument::item::CallArgumentItem, comment::item::CommentItem,
    decorates::item::DecoratesItem, defines::item::DefinesItem,
    expression_shape::item::ExpressionShapeItem, function::item::FunctionItem,
    inherits::item::InheritsItem, mention::item::MentionItem,
    numeric_literal::item::NumericLiteralItem, parameter::item::ParameterItem,
    reference::item::ReferenceItem, resolves::item::ResolvesItem, scope::item::ScopeItem,
    static_call::item::StaticCallItem, static_import::item::StaticImportItem,
    string_literal::item::StringLiteralItem, syntax_child::item::SyntaxChildItem,
    syntax_error::item::SyntaxErrorItem, syntax_tree::item::SyntaxTreeItem,
    visibility::item::VisibilityItem,
};
use crate::domain::syntax::constants::NO_CONFIGURATION;
use crate::domain::syntax::service::{kind_bits, kind_name, kinds};
use crate::outbound::notes::interface::NotesPort;
use crate::outbound::syntax::interface::SyntaxPort;

/// `parse`, implementing `ryo::Parse`.
pub(crate) struct ParseContract {
    /// The parser that turns a file's text into its tree.
    syntax: Arc<dyn SyntaxPort>,
    /// The store that keeps what a parse noted of a file for the project step to read.
    notes: Arc<dyn NotesPort>,
}

impl ParseContract {
    /// `parse` over the ports it calls, which the edge injects.
    #[must_use]
    pub(crate) fn new(syntax: Arc<dyn SyntaxPort>, notes: Arc<dyn NotesPort>) -> Self {
        Self { syntax, notes }
    }
}

impl Parse for ParseContract {
    type FileModel = ReadIndexes;

    fn begin_semantics(&self, _: &ParseRequest<'_>, _: &Elements, _: &mut SemanticSlot) {}

    fn semantics(&self) -> Option<&dyn languages::Semantics<ReadIndexes>> {
        Some(self)
    }

    fn completion_requirement(&self) -> CompletionRequirement {
        CompletionRequirement::Independent
    }

    fn extend_file_model(&self, _: &Tree<'_>, _: Position, _: &mut ReadIndexes) {}

    fn recognizes_explicit(&self, _: &ParseRequest<'_>) -> Recognition {
        Recognition::Declines
    }

    fn capabilities(&self) -> &'static [languages::OptionalCapability] {
        &[]
    }

    fn configuration_identity(&self) -> languages::ParseConfigurationIdentity<'_> {
        languages::ParseConfigurationIdentity(NO_CONFIGURATION)
    }

    fn file_model(&self, _tree: &Tree<'_>, model: &mut ReadIndexes) {
        clear(model);
    }

    fn recognizes(&self, request: &ParseRequest<'_>) -> Recognition {
        reads(request.path)
    }

    fn skips(&self) -> &'static [languages::SkippedDirectory] {
        crate::domain::language::constants::SKIPS
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
    }

    fn kind_count(&self) -> languages::Count {
        languages::Count(kinds().count())
    }

    fn kind_names(&self) -> languages::KindNames {
        languages::KindNames(kinds().map(kind_name).collect())
    }

    fn read_tree(
        &self,
        request: &ParseRequest<'_>,
        builder: &mut languages::TableBuilder<'_>,
    ) -> Result<(), languages::ParseError> {
        // Bytes that are not text are read, not refused: the adapter reads each byte that is not part of a character as one the lexer
        // reports.
        self.syntax.read(SourceText(request.bytes.0), builder);
        Ok(())
    }

    fn complete(&self, request: &ParseRequest<'_>, completion: &mut languages::Completion<'_>) {
        for kind in kinds() {
            languages::name_kind(completion, kind_bits(kind), kind_name(kind));
        }
        if request.fields == languages::FieldRequest::Wanted {
            for field in fields() {
                languages::name_field(completion, field_bits(field), field_name(field));
            }
            languages::write_fields(completion, role_of);
        }
        if request.notes == languages::NoteRequest::Wanted {
            languages::write_notes(completion, |tree, out| {
                self.notes.write(&note_of(tree), out);
            });
        }
    }

    fn function(&self) -> Support<Reader<languages::Function, ReadIndexes>> {
        languages::reads::<FunctionItem>()
    }

    fn annotates(&self) -> Support<Reader<languages::Annotates, ReadIndexes>> {
        languages::reads::<AnnotatesItem>()
    }

    fn assigns(&self) -> Support<Reader<languages::Assigns, ReadIndexes>> {
        languages::reads::<AssignsItem>()
    }

    fn comment(&self) -> Support<Reader<languages::Comment, ReadIndexes>> {
        languages::reads::<CommentItem>()
    }

    fn directive(&self) -> Support<Reader<languages::Directive, ReadIndexes>> {
        // standard: The Rust Reference, Attributes and Conditional compilation: conditions are attributes of items, which `cfg` reads as syntax, not instructions to a build tool in a comment or header.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn decorates(&self) -> Support<Reader<languages::Decorates, ReadIndexes>> {
        languages::reads::<DecoratesItem>()
    }

    fn defines(&self) -> Support<Reader<languages::Defines, ReadIndexes>> {
        languages::reads::<DefinesItem>()
    }

    fn visibility(&self) -> Support<Reader<languages::Visibility, ReadIndexes>> {
        languages::reads::<VisibilityItem>()
    }

    fn inherits(&self) -> Support<Reader<languages::Inherits, ReadIndexes>> {
        languages::reads::<InheritsItem>()
    }

    fn mention(&self) -> Support<Reader<languages::Mention, ReadIndexes>> {
        languages::reads::<MentionItem>()
    }

    fn parameter(&self) -> Support<Reader<languages::Parameter, ReadIndexes>> {
        languages::reads::<ParameterItem>()
    }

    fn reference(&self) -> Support<Reader<languages::Reference, ReadIndexes>> {
        languages::reads::<ReferenceItem>()
    }

    fn scope(&self) -> Support<Reader<languages::Scope, ReadIndexes>> {
        languages::reads::<ScopeItem>()
    }

    fn static_call(&self) -> Support<Reader<languages::StaticCall, ReadIndexes>> {
        languages::reads::<StaticCallItem>()
    }

    fn call_argument(&self) -> Support<Reader<languages::CallArgument, ReadIndexes>> {
        languages::reads::<CallArgumentItem>()
    }

    fn semantic_syntax(&self) -> Support<Reader<languages::SemanticSyntax, ReadIndexes>> {
        languages::native_semantic_syntax()
    }

    fn semantic_child(&self) -> Support<Reader<languages::SemanticChild, ReadIndexes>> {
        languages::native_semantic_child()
    }

    fn resolves(&self) -> Support<Reader<languages::Resolves, ReadIndexes>> {
        languages::reads::<ResolvesItem>()
    }

    fn static_import(&self) -> Support<Reader<languages::StaticImport, ReadIndexes>> {
        languages::reads::<StaticImportItem>()
    }

    fn string_literal(&self) -> Support<Reader<languages::StringLiteral, ReadIndexes>> {
        languages::reads::<StringLiteralItem>()
    }

    fn prose(&self) -> Support<Reader<languages::Prose, ReadIndexes>> {
        languages::absent(AbsenceCause::NotADocument)
    }

    fn syntax_child(&self) -> Support<Reader<languages::SyntaxChild, ReadIndexes>> {
        languages::reads::<SyntaxChildItem>()
    }

    fn syntax_error(&self) -> Support<Reader<languages::SyntaxError, ReadIndexes>> {
        languages::reads::<SyntaxErrorItem>()
    }

    fn source_location(&self) -> Support<Reader<languages::SourceLocation, ReadIndexes>> {
        languages::source_locations()
    }

    fn indent_region(&self) -> Support<Reader<languages::IndentRegion, ReadIndexes>> {
        // standard: Rust Reference, Whitespace: nonempty whitespace separates tokens;
        // lexical INDENT/DEDENT suite regions are not part of its whitespace grammar.
        // https://doc.rust-lang.org/reference/whitespace.html
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn syntax_tree(&self) -> Support<Reader<languages::SyntaxTree, ReadIndexes>> {
        languages::reads::<SyntaxTreeItem>()
    }

    fn mapping(&self) -> Support<Reader<languages::Mapping, ReadIndexes>> {
        languages::absent(AbsenceCause::NotAKeyedDocument)
    }

    fn entry(&self) -> Support<Reader<languages::Entry, ReadIndexes>> {
        languages::absent(AbsenceCause::NotAKeyedDocument)
    }

    fn sequence_element(&self) -> Support<Reader<languages::SequenceElement, ReadIndexes>> {
        languages::absent(AbsenceCause::NotAKeyedDocument)
    }
}

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
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn implicit_symbol(&self) -> Support<Reader<languages::ImplicitSymbol, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_event(&self) -> Support<Reader<languages::BindingEvent, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_read(&self) -> Support<Reader<languages::BindingRead, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_use(&self) -> Support<Reader<languages::BindingUse, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_shadow(&self) -> Support<Reader<languages::BindingShadow, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn identifier(&self) -> Support<Reader<languages::Identifier, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn qualified_name(&self) -> Support<Reader<languages::QualifiedName, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn standard_library_import(
        &self,
    ) -> Support<Reader<languages::StandardLibraryImport, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn pattern_parse(&self) -> Support<Reader<languages::PatternParse, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn pattern_syntax(&self) -> Support<Reader<languages::PatternSyntax, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn pattern_child(&self) -> Support<Reader<languages::PatternChild, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }
}
