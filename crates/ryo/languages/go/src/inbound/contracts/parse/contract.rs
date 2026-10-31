//! The `parse` contract: the language's implementation of `ryo::languages::Parse`. It says which files are the language's, builds the file's
//! tree in the worker's element table through the syntax port, gives each element its role, the field of `go/ast` that holds it, when
//! the run asks for the roles, notes the packages the file imports when the run asks for its notes,
//! and answers, for every syntax relation of ryo's catalog, how the language reads it or that it is not in the language yet.

use std::sync::Arc;

use ryo::languages::{self, AbsenceCause, Parse, Reader, Support, UnknownName};

use crate::domain::fact_notes::service::noted;
use crate::domain::language::constants::UNKNOWN_REASONS;
use crate::domain::language::service::{file_of, reads};
use crate::domain::language::types::GoFile;
use crate::domain::modfile::service as modfile;
use crate::domain::relations::collections::relations::annotates::item::AnnotatesItem;
use crate::domain::relations::collections::relations::assigns::item::AssignsItem;
use crate::domain::relations::collections::relations::comment::item::CommentItem;
use crate::domain::relations::collections::relations::defines::item::DefinesItem;
use crate::domain::relations::collections::relations::directive::item::DirectiveItem;
use crate::domain::relations::collections::relations::entry::item::EntryItem;
use crate::domain::relations::collections::relations::function::item::FunctionItem;
use crate::domain::relations::collections::relations::inherits::item::InheritsItem;
use crate::domain::relations::collections::relations::mapping::item::MappingItem;
use crate::domain::relations::collections::relations::parameter::item::ParameterItem;
use crate::domain::relations::collections::relations::reference::item::ReferenceItem;
use crate::domain::relations::collections::relations::scope::item::ScopeItem;
use crate::domain::relations::collections::relations::static_call::item::StaticCallItem;
use crate::domain::relations::collections::relations::static_import::item::StaticImportItem;
use crate::domain::relations::collections::relations::string_literal::item::StringLiteralItem;
use crate::domain::relations::collections::relations::syntax_child::item::SyntaxChildItem;
use crate::domain::relations::collections::relations::syntax_error::item::SyntaxErrorItem;
use crate::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeItem;
use crate::domain::roles::service::{field_bits, field_name, fields, role_of};
use crate::domain::syntax::service::{kind_bits, kind_name, kinds};
use crate::outbound::notes::interface::NotesPort;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;

/// `parse`, implementing `ryo::languages::Parse`.
pub(crate) struct ParseContract {
    /// The syntax port the contract reads each go source's tree through.
    syntax: Arc<dyn SyntaxPort>,
    /// The notes port the contract writes the notes of each file through.
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
        // A `go.mod` is no go source: the language reads it itself, as `golang.org/x/mod/modfile` does.
        match file_of(request.path) {
            GoFile::Module => modfile::read(request.bytes, builder),
            GoFile::Source | GoFile::Other => {
                self.syntax.read(SourceText(request.bytes.0), builder);
            }
        }
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
        // A `go.mod` imports no package: it leaves no note, so it is no source whose imports are examined.
        if request.notes == languages::NoteRequest::Wanted
            && file_of(request.path) != GoFile::Module
        {
            languages::write_notes(completion, |tree, out| {
                self.notes.write(&noted(tree), out);
            });
        }
    }

    fn source_location(&self) -> Support<Reader<languages::SourceLocation, ()>> {
        languages::source_locations()
    }

    fn syntax_tree(&self) -> Support<Reader<languages::SyntaxTree, ()>> {
        languages::reads::<SyntaxTreeItem>()
    }

    fn comment(&self) -> Support<Reader<languages::Comment, ()>> {
        languages::reads::<CommentItem>()
    }

    fn string_literal(&self) -> Support<Reader<languages::StringLiteral, ()>> {
        languages::reads::<StringLiteralItem>()
    }

    fn directive(&self) -> Support<Reader<languages::Directive, ()>> {
        languages::reads::<DirectiveItem>()
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

    fn mapping(&self) -> Support<Reader<languages::Mapping, ()>> {
        // A go source has no mappings; a module's `go.mod` has the file and each `require` block.
        languages::reads::<MappingItem>()
    }

    fn entry(&self) -> Support<Reader<languages::Entry, ()>> {
        // A go source has no keys; a module's `go.mod` has one for each module it requires.
        languages::reads::<EntryItem>()
    }

    fn sequence_element(&self) -> Support<Reader<languages::SequenceElement, ()>> {
        // standard: Go Modules Reference, "go.mod files" (go.dev/ref/mod#go-mod-file-grammar): a `go.mod` writes no list of values, and a go source is no keyed document.
        languages::absent(AbsenceCause::NoSuchConstruct)
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

    fn decorates(&self) -> Support<Reader<languages::Decorates, ()>> {
        // standard: the Go specification (go.dev/ref/spec), Declarations and scope: it has no decorator or annotation that applies to a declaration, and `go/ast` has no node for one.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn defines(&self) -> Support<Reader<languages::Defines, ()>> {
        languages::reads::<DefinesItem>()
    }

    fn visibility(&self) -> Support<Reader<languages::Visibility, ()>> {
        // standard: The Go Programming Language Specification, Exported identifiers: an identifier is exported when its first character is an upper case letter and it is declared in the package block or is a field or method name; that is a property of the name, and no syntax writes it.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn inherits(&self) -> Support<Reader<languages::Inherits, ()>> {
        languages::reads::<InheritsItem>()
    }

    fn mention(&self) -> Support<Reader<languages::Mention, ()>> {
        // standard: the Go specification (go.dev/ref/spec), Comments: it has no construct a name is mentioned in outside the code; a comment is a comment.
        languages::absent(AbsenceCause::NoSuchConstruct)
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
        languages::reads::<
            crate::domain::relations::collections::relations::call_argument::item::CallArgumentItem,
        >()
    }

    fn resolves(&self) -> Support<Reader<languages::Resolves, ()>> {
        languages::reads::<
            crate::domain::relations::collections::relations::resolves::item::ResolvesItem,
        >()
    }

    fn static_import(&self) -> Support<Reader<languages::StaticImport, ()>> {
        languages::reads::<StaticImportItem>()
    }
}
