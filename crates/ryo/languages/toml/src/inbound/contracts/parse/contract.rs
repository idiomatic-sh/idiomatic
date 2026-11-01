//! The `parse` contract: it implements the contract another crate declares, with nothing of its
//! own beside it. It says which files are TOML, builds the file's tree in the worker's element table through a
//! builder, and answers, for every syntax relation of ryo's catalog, how TOML reads it or that it is not in the
//! language: TOML is a format of data, so every relation of code is not in it.

use std::sync::Arc;

use ryo::languages::{self, AbsenceCause, Parse, Reader, Support, UnknownName};

use crate::domain::documents::service::build;
use crate::domain::language::service::reads;
use crate::domain::model::types::Model;
use crate::domain::relations::collections::relations::comment::item::CommentItem;
use crate::domain::relations::collections::relations::entry::item::EntryItem;
use crate::domain::relations::collections::relations::mapping::item::MappingItem;
use crate::domain::relations::collections::relations::sequence_element::item::SequenceElementItem;
use crate::domain::relations::collections::relations::string_literal::item::StringLiteralItem;
use crate::domain::relations::collections::relations::syntax_child::item::SyntaxChildItem;
use crate::domain::relations::collections::relations::syntax_error::item::SyntaxErrorItem;
use crate::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeItem;
use crate::domain::syntax::service::{
    field_bits, field_name, fields, kind_bits, kind_name, kinds, role_of,
};
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;

/// `parse`, implementing `ryo::languages::Parse`.
pub(crate) struct ParseContract {
    /// The syntax port the contract reads each file's tree through.
    syntax: Arc<dyn SyntaxPort>,
}

impl ParseContract {
    /// `parse` over the ports it calls, which the edge injects.
    #[must_use]
    pub(crate) fn new(syntax: Arc<dyn SyntaxPort>) -> Self {
        Self { syntax }
    }
}

impl Parse for ParseContract {
    type FileModel = Model;

    fn capabilities(&self) -> &'static [languages::OptionalCapability] {
        &[
            languages::OptionalCapability::Identifier,
            languages::OptionalCapability::QualifiedName,
        ]
    }

    fn configuration_identity(&self) -> languages::ParseConfigurationIdentity<'_> {
        languages::ParseConfigurationIdentity("")
    }

    fn file_model(&self, tree: &languages::Tree<'_>, model: &mut Model) {
        build(tree, model);
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
    }

    fn recognizes(&self, request: &languages::ParseRequest<'_>) -> languages::Recognition {
        reads(request.path)
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
        self.syntax.read(SourceText(request.bytes.0), builder);
        Ok(())
    }

    fn complete(
        &self,
        request: &languages::ParseRequest<'_>,
        completion: &mut languages::Completion<'_>,
    ) {
        for kind in kinds() {
            languages::name_kind(completion, kind_bits(kind), kind_name(kind));
        }
        if request.fields == languages::FieldRequest::Wanted {
            for field in fields() {
                languages::name_field(completion, field_bits(field), field_name(field));
            }
            languages::write_fields(completion, role_of);
        }
    }

    fn comment(&self) -> Support<Reader<languages::Comment, Model>> {
        languages::reads::<CommentItem>()
    }

    fn directive(&self) -> Support<Reader<languages::Directive, Model>> {
        // standard: TOML v1.0.0: the format defines keys, tables and comments, and no instruction to a build tool.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn string_literal(&self) -> Support<Reader<languages::StringLiteral, Model>> {
        languages::reads::<StringLiteralItem>()
    }

    fn prose(&self) -> Support<Reader<languages::Prose, Model>> {
        languages::absent(AbsenceCause::NotADocument)
    }

    fn syntax_child(&self) -> Support<Reader<languages::SyntaxChild, Model>> {
        languages::reads::<SyntaxChildItem>()
    }

    fn syntax_error(&self) -> Support<Reader<languages::SyntaxError, Model>> {
        languages::reads::<SyntaxErrorItem>()
    }

    fn source_location(&self) -> Support<Reader<languages::SourceLocation, Model>> {
        languages::source_locations()
    }

    fn indent_region(&self) -> Support<Reader<languages::IndentRegion, Model>> {
        // standard: TOML 1.0.0, Arrays and Table: indentation is ignored as whitespace;
        // table headers and array brackets do not form lexical INDENT/DEDENT regions.
        // https://toml.io/en/v1.0.0#table
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn syntax_tree(&self) -> Support<Reader<languages::SyntaxTree, Model>> {
        languages::reads::<SyntaxTreeItem>()
    }

    fn mapping(&self) -> Support<Reader<languages::Mapping, Model>> {
        languages::reads::<MappingItem>()
    }

    fn entry(&self) -> Support<Reader<languages::Entry, Model>> {
        languages::reads::<EntryItem>()
    }

    fn sequence_element(&self) -> Support<Reader<languages::SequenceElement, Model>> {
        languages::reads::<SequenceElementItem>()
    }

    fn function(&self) -> Support<Reader<languages::Function, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn annotates(&self) -> Support<Reader<languages::Annotates, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn assigns(&self) -> Support<Reader<languages::Assigns, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn decorates(&self) -> Support<Reader<languages::Decorates, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn defines(&self) -> Support<Reader<languages::Defines, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn visibility(&self) -> Support<Reader<languages::Visibility, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn inherits(&self) -> Support<Reader<languages::Inherits, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn mention(&self) -> Support<Reader<languages::Mention, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn parameter(&self) -> Support<Reader<languages::Parameter, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn reference(&self) -> Support<Reader<languages::Reference, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn scope(&self) -> Support<Reader<languages::Scope, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn static_call(&self) -> Support<Reader<languages::StaticCall, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn call_argument(&self) -> Support<Reader<languages::CallArgument, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn embedded_expression(
        &self,
    ) -> Support<Reader<languages::EmbeddedExpression, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn semantic_syntax(&self) -> Support<Reader<languages::SemanticSyntax, Self::FileModel>> {
        languages::native_semantic_syntax()
    }

    fn semantic_child(&self) -> Support<Reader<languages::SemanticChild, Self::FileModel>> {
        languages::native_semantic_child()
    }

    fn expression_shape(&self) -> Support<Reader<languages::ExpressionShape, Model>> {
        // Data documents do not contain executable source expressions.
        languages::absent(AbsenceCause::NotCode)
    }

    fn constant_key(&self) -> Support<Reader<languages::ConstantKey, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn format_parse(&self) -> Support<Reader<languages::FormatParse, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn format_field(&self) -> Support<Reader<languages::FormatField, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn literal_expression_parse(
        &self,
    ) -> Support<Reader<languages::LiteralExpressionParse, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn raw_format_segment(&self) -> Support<Reader<languages::RawFormatSegment, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn raw_format_field(&self) -> Support<Reader<languages::RawFormatField, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn brace_format_parse(&self) -> Support<Reader<languages::BraceFormatParse, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn brace_format_field(&self) -> Support<Reader<languages::BraceFormatField, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn function_terminal(&self) -> Support<Reader<languages::FunctionTerminal, Self::FileModel>> {
        // This provider does not yet implement this optional semantic relation.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn expression_type(&self) -> Support<Reader<languages::ExpressionType, Self::FileModel>> {
        // This provider does not yet implement this optional semantic relation.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn interpolated_string_literal(
        &self,
    ) -> Support<Reader<languages::InterpolatedStringLiteral, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn literal_properties(&self) -> Support<Reader<languages::LiteralProperties, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn language_target(&self) -> Support<Reader<languages::LanguageTarget, Self::FileModel>> {
        // This provider does not expose a parser target version.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn implicit_symbol(&self) -> Support<Reader<languages::ImplicitSymbol, Self::FileModel>> {
        // This optional provider catalog is not yet implemented or advertised here.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_event(&self) -> Support<Reader<languages::BindingEvent, Self::FileModel>> {
        // This optional analysis is not yet implemented or advertised by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_read(&self) -> Support<Reader<languages::BindingRead, Self::FileModel>> {
        // This optional analysis is not yet implemented or advertised by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_use(&self) -> Support<Reader<languages::BindingUse, Self::FileModel>> {
        // This optional analysis is not yet implemented or advertised by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_shadow(&self) -> Support<Reader<languages::BindingShadow, Self::FileModel>> {
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

    fn resolves(&self) -> Support<Reader<languages::Resolves, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }

    fn static_import(&self) -> Support<Reader<languages::StaticImport, Model>> {
        languages::absent(AbsenceCause::NotCode)
    }
}
