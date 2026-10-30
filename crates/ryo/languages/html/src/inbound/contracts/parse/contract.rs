//! The `parse` contract: the language's implementation of `ryo::languages::Parse`. It says which files are the language's, builds the file's
//! tree in the worker's element table through the syntax port, and answers, for every syntax relation of ryo's catalog, how the
//! language reads it or that it is not in the language yet.

use std::sync::Arc;

use ryo::languages::{self, AbsenceCause, Parse, Reader, Support, UnknownName};

use crate::domain::language::service::reads;
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
    type FileModel = ();

    fn file_model(&self, _tree: &languages::Tree<'_>, _model: &mut ()) {}

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
            languages::SourcePath(request.path.0),
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

    fn syntax_tree(&self) -> Support<Reader<languages::SyntaxTree, ()>> {
        languages::reads::<SyntaxTreeItem>()
    }

    fn comment(&self) -> Support<Reader<languages::Comment, ()>> {
        languages::reads::<CommentItem>()
    }

    fn directive(&self) -> Support<Reader<languages::Directive, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not a build instruction.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn string_literal(&self) -> Support<Reader<languages::StringLiteral, ()>> {
        languages::reads::<StringLiteralItem>()
    }

    fn prose(&self) -> Support<Reader<languages::Prose, ()>> {
        languages::reads::<ProseItem>()
    }

    fn syntax_child(&self) -> Support<Reader<languages::SyntaxChild, ()>> {
        languages::reads::<SyntaxChildItem>()
    }

    fn syntax_error(&self) -> Support<Reader<languages::SyntaxError, ()>> {
        languages::reads::<SyntaxErrorItem>()
    }

    fn mapping(&self) -> Support<Reader<languages::Mapping, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn entry(&self) -> Support<Reader<languages::Entry, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn sequence_element(&self) -> Support<Reader<languages::SequenceElement, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn function(&self) -> Support<Reader<languages::Function, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn annotates(&self) -> Support<Reader<languages::Annotates, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn assigns(&self) -> Support<Reader<languages::Assigns, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn decorates(&self) -> Support<Reader<languages::Decorates, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn defines(&self) -> Support<Reader<languages::Defines, ()>> {
        languages::reads::<DefinesItem>()
    }

    fn visibility(&self) -> Support<Reader<languages::Visibility, ()>> {
        // standard: WHATWG HTML, The elements of HTML: an element or an attribute is not written with a visibility; every id of a document is the document's.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn inherits(&self) -> Support<Reader<languages::Inherits, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn mention(&self) -> Support<Reader<languages::Mention, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn parameter(&self) -> Support<Reader<languages::Parameter, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn reference(&self) -> Support<Reader<languages::Reference, ()>> {
        languages::reads::<ReferenceItem>()
    }

    fn scope(&self) -> Support<Reader<languages::Scope, ()>> {
        languages::reads::<ScopeItem>()
    }

    fn static_call(&self) -> Support<Reader<languages::StaticCall, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn call_argument(&self) -> Support<Reader<languages::CallArgument, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn resolves(&self) -> Support<Reader<languages::Resolves, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn static_import(&self) -> Support<Reader<languages::StaticImport, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document nodes, not this programming-language or package construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
    }
}
