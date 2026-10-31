//! The `parse` contract: it implements the contract another crate declares, with nothing of its own beside it. It says which files
//! are Python, builds the file's tree in the worker's element table through a builder, works out the file's `global` and `nonlocal`
//! names once, notes the imports of the file when the run asks for its notes, gives each element its role in the fields of
//! `Python.asdl` when the run asks for roles, and answers, for every syntax relation of ryo's catalog, how Python reads it or that it
//! is not in the language: Python has no macros, so nothing is a mention, and it is code, so it has no keyed documents.

use std::sync::Arc;

use ryo::languages::{self, AbsenceCause, Parse, Reader, Support, UnknownName};

use crate::domain::fact_notes::service::noted;
use crate::domain::fields::service::role_of;
use crate::domain::labels::service::{field_bits, field_name, fields};
use crate::domain::language::constants::UNKNOWN_REASONS;
use crate::domain::language::service::reads;
use crate::domain::model::types::Declarations;
use crate::domain::relations::collections::relations::annotates::item::AnnotatesItem;
use crate::domain::relations::collections::relations::assigns::item::AssignsItem;
use crate::domain::relations::collections::relations::comment::item::CommentItem;
use crate::domain::relations::collections::relations::decorates::item::DecoratesItem;
use crate::domain::relations::collections::relations::defines::item::DefinesItem;
use crate::domain::relations::collections::relations::function::item::FunctionItem;
use crate::domain::relations::collections::relations::inherits::item::InheritsItem;
use crate::domain::relations::collections::relations::parameter::item::ParameterItem;
use crate::domain::relations::collections::relations::reference::item::ReferenceItem;
use crate::domain::relations::collections::relations::scope::item::ScopeItem;
use crate::domain::relations::collections::relations::static_call::item::StaticCallItem;
use crate::domain::relations::collections::relations::static_import::item::StaticImportItem;
use crate::domain::relations::collections::relations::string_literal::item::StringLiteralItem;
use crate::domain::relations::collections::relations::syntax_child::item::SyntaxChildItem;
use crate::domain::relations::collections::relations::syntax_error::item::SyntaxErrorItem;
use crate::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeItem;
use crate::domain::scopes::service::declare;
use crate::domain::syntax::service::{kind_bits, kind_name, kinds};
use crate::outbound::notes::interface::NotesPort;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;

/// `parse`, implementing `ryo::languages::Parse`.
pub(crate) struct ParseContract {
    /// The parser that reads Python text into the tree.
    syntax: Arc<dyn SyntaxPort>,
    /// What writes the notes of a file as bytes.
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
    type FileModel = Declarations;

    fn file_model(&self, tree: &languages::Tree<'_>, model: &mut Declarations) {
        crate::domain::indentation::service::clear(model.indentation.get_mut());
        declare(tree, model);
        crate::domain::bindings::service::index(tree, model);
    }

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
                self.notes.write(&noted(tree), out);
            });
        }
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

    fn resolves(&self) -> Support<Reader<languages::Resolves, Declarations>> {
        languages::reads::<
            crate::domain::relations::collections::relations::resolves::item::ResolvesItem,
        >()
    }

    fn call_argument(&self) -> Support<Reader<languages::CallArgument, Declarations>> {
        languages::reads::<
            crate::domain::relations::collections::relations::call_argument::item::CallArgumentItem,
        >()
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
        languages::reads::<
            crate::domain::relations::collections::relations::indent_region::item::IndentRegionItem,
        >()
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
