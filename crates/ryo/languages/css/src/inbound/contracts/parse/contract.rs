//! The `parse` contract: it implements the contract another crate declares, with nothing of its own beside it. It says which files
//! are CSS, builds the file's tree in the worker's element table through a builder, and answers, for every syntax relation of ryo's
//! catalog, how CSS reads it or that it is not in the language: a stylesheet imports other stylesheets and points at files, which it
//! reads, and has no function, parameter, call, class or binding, which it does not; it is a list of rules, not a document of keys,
//! so neither are the keyed relations.

use ryo::languages::SourcePath;
use std::sync::Arc;

use ryo::languages::{self, AbsenceCause, Parse, Reader, Support, UnknownName};

use crate::domain::language::constants::UNKNOWN_REASONS;
use crate::domain::language::service::reads;
use crate::domain::relations::collections::relations::comment::item::CommentItem;
use crate::domain::relations::collections::relations::reference::item::ReferenceItem;
use crate::domain::relations::collections::relations::static_import::item::StaticImportItem;
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
    type FileModel = ();

    fn capabilities(&self) -> &'static [languages::OptionalCapability] {
        &[]
    }

    fn configuration_identity(&self) -> languages::ParseConfigurationIdentity<'_> {
        languages::ParseConfigurationIdentity("")
    }

    fn file_model(&self, _tree: &languages::Tree<'_>, _model: &mut ()) {}

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        UNKNOWN_REASONS
    }

    fn recognizes(&self, request: &languages::ParseRequest<'_>) -> languages::Recognition {
        reads(request.path)
    }

    fn skips(&self) -> &'static [languages::SkippedDirectory] {
        crate::domain::language::constants::SKIPS
    }

    fn kind_names(&self) -> languages::KindNames {
        languages::KindNames(kinds().map(kind_name).collect())
    }

    fn kind_count(&self) -> languages::Count {
        languages::Count(kinds().count())
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

    fn comment(&self) -> Support<Reader<languages::Comment, ()>> {
        languages::reads::<CommentItem>()
    }

    fn directive(&self) -> Support<Reader<languages::Directive, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: its nodes are rules and declarations, and none is an instruction to a build tool.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn string_literal(&self) -> Support<Reader<languages::StringLiteral, ()>> {
        languages::reads::<StringLiteralItem>()
    }

    fn syntax_child(&self) -> Support<Reader<languages::SyntaxChild, ()>> {
        languages::reads::<SyntaxChildItem>()
    }

    fn syntax_error(&self) -> Support<Reader<languages::SyntaxError, ()>> {
        languages::reads::<SyntaxErrorItem>()
    }

    fn source_location(&self) -> Support<Reader<languages::SourceLocation, ()>> {
        languages::source_locations()
    }

    fn indent_region(&self) -> Support<Reader<languages::IndentRegion, ()>> {
        // standard: CSS Syntax Level 3, Tokenization: whitespace produces whitespace tokens;
        // blocks use delimiter tokens, with no lexical INDENT/DEDENT suite regions.
        // https://www.w3.org/TR/css-syntax-3/#tokenization
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn syntax_tree(&self) -> Support<Reader<languages::SyntaxTree, ()>> {
        languages::reads::<SyntaxTreeItem>()
    }

    fn static_import(&self) -> Support<Reader<languages::StaticImport, ()>> {
        languages::reads::<StaticImportItem>()
    }

    fn reference(&self) -> Support<Reader<languages::Reference, ()>> {
        languages::reads::<ReferenceItem>()
    }

    fn prose(&self) -> Support<Reader<languages::Prose, ()>> {
        languages::absent(AbsenceCause::NotADocument)
    }

    fn mapping(&self) -> Support<Reader<languages::Mapping, ()>> {
        languages::absent(AbsenceCause::NotAKeyedDocument)
    }

    fn entry(&self) -> Support<Reader<languages::Entry, ()>> {
        languages::absent(AbsenceCause::NotAKeyedDocument)
    }

    fn sequence_element(&self) -> Support<Reader<languages::SequenceElement, ()>> {
        languages::absent(AbsenceCause::NotAKeyedDocument)
    }

    fn function(&self) -> Support<Reader<languages::Function, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: its nodes are the stylesheet, rules, at-rules, qualified rules, declarations, functions as component values and simple blocks, and none declares a function.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn annotates(&self) -> Support<Reader<languages::Annotates, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: it has no type, so nothing annotates a parameter, a function result or a binding with one.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn assigns(&self) -> Support<Reader<languages::Assigns, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: it has no `let` or other binding of a name to a value.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn decorates(&self) -> Support<Reader<languages::Decorates, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: it has no attribute or decorator that a declaration carries.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn defines(&self) -> Support<Reader<languages::Defines, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: it declares no name a file brings in: a rule or a declaration is not a definition in the standard, only a node.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn visibility(&self) -> Support<Reader<languages::Visibility, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: a rule or a declaration is no declaration of a name, and none is written with a visibility.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn inherits(&self) -> Support<Reader<languages::Inherits, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: it has no type, so no base for a type to extend or implement.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn mention(&self) -> Support<Reader<languages::Mention, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: it has no macro call, so no token tree of one.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn parameter(&self) -> Support<Reader<languages::Parameter, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: it has no function declaration, so no parameter of one.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn scope(&self) -> Support<Reader<languages::Scope, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: it defines no scope: a block is a simple block of component values, not a scope of names.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn static_call(&self) -> Support<Reader<languages::StaticCall, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: it has no call of a path written out: a function is a component value with a name, not a path.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn call_argument(&self) -> Support<Reader<languages::CallArgument, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: it has no call of a path written out, so it has no argument of one.
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

    fn expression_shape(&self) -> Support<Reader<languages::ExpressionShape, ()>> {
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

    fn identifier(&self) -> Support<Reader<languages::Identifier, ()>> {
        // Canonical identifier spelling is not yet exposed by this provider.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn qualified_name(&self) -> Support<Reader<languages::QualifiedName, ()>> {
        // Qualified source identities are applicable, but this provider has not implemented their reader.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn resolves(&self) -> Support<Reader<languages::Resolves, ()>> {
        // standard: the CSS Syntax Module Level 3, §5.2 CSS Parsing Results: it has no type, so no type name is written in a type position.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }
}
