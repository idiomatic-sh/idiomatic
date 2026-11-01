//! The `parse` contract: it implements the contract another crate declares, with nothing of its
//! own beside it. It says which files are Rust, builds the file's tree in the worker's element table through a
//! builder, and answers, for every syntax relation of ryo's catalog, how Rust reads it or that it is not in the
//! language. The engine reads the tree with the readers answered here; no relation is left unanswered, and a
//! relation added to the catalog stops this from compiling until it is.

use crate::domain::fields::service::{field_bits, field_name, fields, role_of};
use crate::outbound::syntax::types::SourceText;
use ryo::languages::UnknownName;
use std::sync::Arc;

use ryo::languages::{self, AbsenceCause, Parse, Reader, Support};

use crate::domain::fact_extraction::service::note_of;
use crate::domain::language::service::reads;
use crate::domain::relations::collections::relations::annotates::item::AnnotatesItem;
use crate::domain::relations::collections::relations::assigns::item::AssignsItem;
use crate::domain::relations::collections::relations::call_argument::item::CallArgumentItem;
use crate::domain::relations::collections::relations::comment::item::CommentItem;
use crate::domain::relations::collections::relations::decorates::item::DecoratesItem;
use crate::domain::relations::collections::relations::defines::item::DefinesItem;
use crate::domain::relations::collections::relations::function::item::FunctionItem;
use crate::domain::relations::collections::relations::inherits::item::InheritsItem;
use crate::domain::relations::collections::relations::mention::item::MentionItem;
use crate::domain::relations::collections::relations::parameter::item::ParameterItem;
use crate::domain::relations::collections::relations::reference::item::ReferenceItem;
use crate::domain::relations::collections::relations::resolves::item::ResolvesItem;
use crate::domain::relations::collections::relations::scope::item::ScopeItem;
use crate::domain::relations::collections::relations::static_call::item::StaticCallItem;
use crate::domain::relations::collections::relations::static_import::item::StaticImportItem;
use crate::domain::relations::collections::relations::string_literal::item::StringLiteralItem;
use crate::domain::relations::collections::relations::syntax_child::item::SyntaxChildItem;
use crate::domain::relations::collections::relations::syntax_error::item::SyntaxErrorItem;
use crate::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeItem;
use crate::domain::relations::collections::relations::visibility::item::VisibilityItem;
use crate::domain::syntax::service::{kind_bits, kind_name, kinds};
use crate::outbound::notes::interface::NotesPort;
use crate::outbound::syntax::interface::SyntaxPort;

/// `parse`, implementing `ryo::languages::Parse`.
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
    type FileModel = ();

    fn capabilities(&self) -> &'static [languages::OptionalCapability] {
        &[]
    }

    fn file_model(&self, _tree: &languages::Tree<'_>, _model: &mut ()) {}

    fn recognizes(&self, request: &languages::ParseRequest<'_>) -> languages::Recognition {
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
        request: &languages::ParseRequest<'_>,
        builder: &mut languages::TableBuilder<'_>,
    ) -> Result<(), languages::ParseError> {
        // Bytes that are not text are read, not refused: the adapter reads each byte that is not part of a character as one the lexer
        // reports.
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
        if request.notes == languages::NoteRequest::Wanted {
            languages::write_notes(completion, |tree, out| {
                self.notes.write(&note_of(tree), out);
            });
        }
    }

    fn function(&self) -> Support<Reader<languages::Function, ()>> {
        languages::reads::<FunctionItem>()
    }

    fn annotates(&self) -> Support<Reader<languages::Annotates, ()>> {
        languages::reads::<AnnotatesItem>()
    }

    fn assigns(&self) -> Support<Reader<languages::Assigns, ()>> {
        languages::reads::<AssignsItem>()
    }

    fn comment(&self) -> Support<Reader<languages::Comment, ()>> {
        languages::reads::<CommentItem>()
    }

    fn directive(&self) -> Support<Reader<languages::Directive, ()>> {
        // standard: The Rust Reference, Attributes and Conditional compilation: conditions are attributes of items, which `cfg` reads as syntax, not instructions to a build tool in a comment or header.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn decorates(&self) -> Support<Reader<languages::Decorates, ()>> {
        languages::reads::<DecoratesItem>()
    }

    fn defines(&self) -> Support<Reader<languages::Defines, ()>> {
        languages::reads::<DefinesItem>()
    }

    fn visibility(&self) -> Support<Reader<languages::Visibility, ()>> {
        languages::reads::<VisibilityItem>()
    }

    fn inherits(&self) -> Support<Reader<languages::Inherits, ()>> {
        languages::reads::<InheritsItem>()
    }

    fn mention(&self) -> Support<Reader<languages::Mention, ()>> {
        languages::reads::<MentionItem>()
    }

    fn parameter(&self) -> Support<Reader<languages::Parameter, ()>> {
        languages::reads::<ParameterItem>()
    }

    fn reference(&self) -> Support<Reader<languages::Reference, ()>> {
        languages::reads::<ReferenceItem>()
    }

    fn scope(&self) -> Support<Reader<languages::Scope, ()>> {
        languages::reads::<ScopeItem>()
    }

    fn static_call(&self) -> Support<Reader<languages::StaticCall, ()>> {
        languages::reads::<StaticCallItem>()
    }

    fn call_argument(&self) -> Support<Reader<languages::CallArgument, ()>> {
        languages::reads::<CallArgumentItem>()
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
        languages::reads::<ResolvesItem>()
    }

    fn static_import(&self) -> Support<Reader<languages::StaticImport, ()>> {
        languages::reads::<StaticImportItem>()
    }

    fn string_literal(&self) -> Support<Reader<languages::StringLiteral, ()>> {
        languages::reads::<StringLiteralItem>()
    }

    fn prose(&self) -> Support<Reader<languages::Prose, ()>> {
        languages::absent(AbsenceCause::NotADocument)
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
        // standard: Rust Reference, Whitespace: nonempty whitespace separates tokens;
        // lexical INDENT/DEDENT suite regions are not part of its whitespace grammar.
        // https://doc.rust-lang.org/reference/whitespace.html
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn syntax_tree(&self) -> Support<Reader<languages::SyntaxTree, ()>> {
        languages::reads::<SyntaxTreeItem>()
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
}
