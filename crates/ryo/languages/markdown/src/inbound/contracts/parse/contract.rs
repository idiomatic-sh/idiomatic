//! The `parse` contract: it implements the contract another crate declares, with nothing of its
//! own beside it. It says which files are Markdown, builds the file's tree in the worker's element table through a
//! builder, and answers, for every syntax relation of ryo's catalog, how Markdown reads it or that it is not in the
//! language: Markdown is a format of documents, so every relation of code is not in it, and it holds no keyed document.

use ryo::languages::SourcePath;
use std::sync::Arc;

use ryo::languages::{self, AbsenceCause, Parse, Reader, Support, UnknownName};

use crate::domain::language::service::reads;
use crate::domain::model::service::settle;
use crate::domain::model::types::Sections;
use crate::domain::relations::collections::relations::comment::item::CommentItem;
use crate::domain::relations::collections::relations::defines::item::DefinesItem;
use crate::domain::relations::collections::relations::prose::item::ProseItem;
use crate::domain::relations::collections::relations::reference::item::ReferenceItem;
use crate::domain::relations::collections::relations::scope::item::ScopeItem;
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
    type FileModel = Sections;

    fn capabilities(&self) -> &'static [languages::OptionalCapability] {
        &[languages::OptionalCapability::QualifiedName]
    }

    fn configuration_identity(&self) -> languages::ParseConfigurationIdentity<'_> {
        languages::ParseConfigurationIdentity("")
    }

    fn file_model(&self, tree: &languages::Tree<'_>, model: &mut Sections) {
        settle(tree, model);
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
        self.syntax.read(
            SourceText(request.bytes.0),
            SourcePath(request.path.0),
            builder,
        );
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

    fn comment(&self) -> Support<Reader<languages::Comment, Sections>> {
        languages::reads::<CommentItem>()
    }

    fn directive(&self) -> Support<Reader<languages::Directive, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no node for an instruction to a build tool.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn string_literal(&self) -> Support<Reader<languages::StringLiteral, Sections>> {
        languages::reads::<StringLiteralItem>()
    }

    fn syntax_child(&self) -> Support<Reader<languages::SyntaxChild, Sections>> {
        languages::reads::<SyntaxChildItem>()
    }

    fn syntax_error(&self) -> Support<Reader<languages::SyntaxError, Sections>> {
        languages::reads::<SyntaxErrorItem>()
    }

    fn source_location(&self) -> Support<Reader<languages::SourceLocation, Sections>> {
        languages::source_locations()
    }

    fn indent_region(&self) -> Support<Reader<languages::IndentRegion, Sections>> {
        // standard: CommonMark 0.31.2 sections 4.4, 5.2 and Appendix parsing strategy:
        // indentation affects code blocks and list continuation, but block-specific line rules
        // do not define a lexical INDENT/DEDENT token stack. This absence is not a claim that
        // Markdown ignores indentation. https://spec.commonmark.org/0.31.2/#phase-1-block-structure
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn syntax_tree(&self) -> Support<Reader<languages::SyntaxTree, Sections>> {
        languages::reads::<SyntaxTreeItem>()
    }

    fn prose(&self) -> Support<Reader<languages::Prose, Sections>> {
        languages::reads::<ProseItem>()
    }

    fn mapping(&self) -> Support<Reader<languages::Mapping, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no node that maps keys to values; the front matter nodes `yaml` and `toml` hold text the language does not read as a mapping.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn entry(&self) -> Support<Reader<languages::Entry, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no key and value node.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn sequence_element(&self) -> Support<Reader<languages::SequenceElement, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no node for the element of a sequence of values; a list item is content.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn function(&self) -> Support<Reader<languages::Function, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no function, only the content nodes of a document.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn annotates(&self) -> Support<Reader<languages::Annotates, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no annotation of a declaration.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn assigns(&self) -> Support<Reader<languages::Assigns, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no assignment.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn decorates(&self) -> Support<Reader<languages::Decorates, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no decorator.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn defines(&self) -> Support<Reader<languages::Defines, Sections>> {
        languages::reads::<DefinesItem>()
    }

    fn visibility(&self) -> Support<Reader<languages::Visibility, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no node for the visibility of a section or a label.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn inherits(&self) -> Support<Reader<languages::Inherits, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no type, so nothing inherits.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn mention(&self) -> Support<Reader<languages::Mention, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no mention of a name, only links and references to definitions.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn parameter(&self) -> Support<Reader<languages::Parameter, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no parameter.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn reference(&self) -> Support<Reader<languages::Reference, Sections>> {
        languages::reads::<ReferenceItem>()
    }

    fn scope(&self) -> Support<Reader<languages::Scope, Sections>> {
        languages::reads::<ScopeItem>()
    }

    fn static_call(&self) -> Support<Reader<languages::StaticCall, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no call.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn call_argument(&self) -> Support<Reader<languages::CallArgument, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no call, so no argument of one.
        languages::absent(AbsenceCause::NoSuchConstruct)
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

    fn expression_shape(&self) -> Support<Reader<languages::ExpressionShape, Sections>> {
        // Parsed expression identity is not yet implemented by this provider.
        languages::absent(AbsenceCause::NotYetRead)
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

    fn token_link(&self) -> Support<Reader<languages::TokenLink, Self::FileModel>> {
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

    fn format_specification(
        &self,
    ) -> Support<Reader<languages::FormatSpecification, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn bytes_literal(&self) -> Support<Reader<languages::BytesLiteral, Self::FileModel>> {
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

    fn numeric_literal(&self) -> Support<Reader<languages::NumericLiteral, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn compiler_feature(&self) -> Support<Reader<languages::CompilerFeature, Self::FileModel>> {
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

    fn identifier(&self) -> Support<Reader<languages::Identifier, Sections>> {
        // Canonical identifier spelling is not yet exposed by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn qualified_name(&self) -> Support<Reader<languages::QualifiedName, Sections>> {
        // standard: CommonMark 0.31.2, Blocks and inlines, https://spec.commonmark.org/0.31.2/#blocks-and-inlines defines document/link structure, not lexical import or builtin namespaces; embedded code is read by its own language.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn resolves(&self) -> Support<Reader<languages::Resolves, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no type, so no type name is written in a type position.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn static_import(&self) -> Support<Reader<languages::StaticImport, Sections>> {
        // standard: mdast.md, Nodes: mdast defines no import; a link names a file but does not bring it in.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn standard_library_import(&self) -> Support<Reader<languages::StandardLibraryImport, Sections>> {
        // Markdown links are resources, not language standard-library module imports.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }
}
