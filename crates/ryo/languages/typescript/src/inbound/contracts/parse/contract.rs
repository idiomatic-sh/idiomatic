//! The `parse` contract: the language's implementation of `ryo::Parse`. It says which files are the language's, builds the file's
//! tree in the worker's element table through the syntax port, notes the modules the file imports when the run asks for its notes,
//! and answers, for every syntax relation of ryo's catalog, how the language reads it or that it is not in the language yet.

use crate::domain::fact_notes::service::noted;
use crate::domain::fields::service::name_vocabulary;
use crate::domain::relations::collections::relations::{
    annotates::item::AnnotatesItem, assigns::item::AssignsItem,
    call_argument::item::CallArgumentItem, comment::item::CommentItem,
    decorates::item::DecoratesItem, defines::item::DefinesItem, directive::item::DirectiveItem,
    function::item::FunctionItem, inherits::item::InheritsItem, parameter::item::ParameterItem,
    reference::item::ReferenceItem, resolves::item::ResolvesItem, scope::item::ScopeItem,
    static_call::item::StaticCallItem, static_import::item::StaticImportItem,
    string_literal::item::StringLiteralItem, syntax_child::item::SyntaxChildItem,
    syntax_error::item::SyntaxErrorItem, syntax_tree::item::SyntaxTreeItem,
    visibility::item::VisibilityItem,
};
use std::sync::Arc;

use ryo::languages::{
    self, AbsenceCause, ByteSlice, Completion, CompletionRequirement, Count, Elements, KindNames,
    NoteRequest, OptionalCapability, Parse, ParseConfigurationIdentity, ParseError, ParseRequest,
    Position, Reader, Recognition, SemanticSlot, SkippedDirectory, Support, TableBuilder, Tree,
    UnknownName,
};

use crate::domain::language::service::{reading_of, reads, recognized};
use crate::domain::syntax::constants::NO_CONFIGURATION;
use crate::domain::syntax::service::{kind_name, kinds};
use crate::domain::syntax::types::{Scripting, Stretch, Written};
use crate::outbound::embedded::interface::EmbeddedPort;
use crate::outbound::embedded::types::{BlockLanguage, Markup};
use crate::outbound::notes::interface::NotesPort;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;

/// `parse`, implementing `ryo::Parse`.
pub(crate) struct ParseContract {
    /// The parser that reads TypeScript text into the tree.
    syntax: Arc<dyn SyntaxPort>,
    /// The adapters that find TypeScript blocks in files of other formats, such as `.vue`.
    embedded: Vec<Arc<dyn EmbeddedPort>>,
    /// What writes the notes of a file as the bytes the project of the run reads.
    notes: Arc<dyn NotesPort>,
}

impl ParseContract {
    /// `parse` over the ports it calls, which the edge injects.
    #[must_use]
    pub(crate) fn new(
        syntax: Arc<dyn SyntaxPort>,
        embedded: Vec<Arc<dyn EmbeddedPort>>,
        notes: Arc<dyn NotesPort>,
    ) -> Self {
        Self {
            syntax,
            embedded,
            notes,
        }
    }

    /// The blocks of typescript a file of another format carries, when an embedded adapter claims the file.
    fn stretches<'a>(&self, request: &ParseRequest<'a>) -> Option<Vec<Stretch<'a>>> {
        let adapter = self
            .embedded
            .iter()
            .find(|adapter| adapter.claims(request.path) == Recognition::Reads)?;
        let blocks = adapter.blocks(Markup(
            std::str::from_utf8(request.bytes.0).unwrap_or_default(),
        ));
        Some(
            blocks
                .0
                .iter()
                .map(|block| Stretch {
                    start: Count(block.start.0),
                    text: ByteSlice(block.text.0.as_bytes()),
                    written: match block.dialect {
                        BlockLanguage::Script => Written::Script,
                        BlockLanguage::TypeScript => Written::TypeScript,
                        BlockLanguage::Tsx => Written::Tsx,
                    },
                })
                .collect(),
        )
    }
}

impl Parse for ParseContract {
    type FileModel = ();

    fn begin_semantics(&self, _: &ParseRequest<'_>, _: &Elements, _: &mut SemanticSlot) {}

    fn semantics(&self) -> Option<&dyn languages::Semantics> {
        None
    }

    fn completion_requirement(&self) -> CompletionRequirement {
        CompletionRequirement::Independent
    }

    fn extend_file_model(&self, _: &Tree<'_>, _: Position, _: &mut Self::FileModel) {}

    fn recognizes_explicit(&self, _: &ParseRequest<'_>) -> Recognition {
        Recognition::Declines
    }

    fn capabilities(&self) -> &'static [OptionalCapability] {
        &[]
    }

    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        ParseConfigurationIdentity(NO_CONFIGURATION)
    }

    fn file_model(&self, _tree: &Tree<'_>, _model: &mut ()) {}

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        crate::domain::language::constants::UNKNOWN_REASONS
    }

    fn recognizes(&self, request: &ParseRequest<'_>) -> Recognition {
        // The two languages share the files that carry blocks: this one reads a file with a typed block, the script language the rest.
        recognized(reads(request.path), self.stretches(request).as_deref())
    }

    fn skips(&self) -> &'static [SkippedDirectory] {
        crate::domain::language::constants::SKIPS
    }

    fn kind_count(&self) -> Count {
        Count(kinds().count())
    }

    fn kind_names(&self) -> KindNames {
        KindNames(kinds().map(kind_name).collect())
    }

    fn read_tree(
        &self,
        request: &ParseRequest<'_>,
        builder: &mut TableBuilder<'_>,
    ) -> Result<(), ParseError> {
        let bytes = ByteSlice(request.bytes.0);
        match reading_of(request.path, bytes, self.stretches(request).as_deref()) {
            Scripting::Own(dialect) => self.syntax.read(SourceText(bytes.0), dialect, builder),
            Scripting::Script(script, dialect) => {
                self.syntax.read(SourceText(&script.0), dialect, builder);
            }
        }
        Ok(())
    }

    fn complete(&self, request: &ParseRequest<'_>, completion: &mut Completion<'_>) {
        name_vocabulary(completion, request.fields);
        if request.notes == NoteRequest::Wanted {
            languages::write_notes(completion, |tree, out| self.notes.write(&noted(tree), out));
        }
    }

    fn source_location(&self) -> Support<Reader<languages::SourceLocation, ()>> {
        languages::source_locations()
    }

    fn indent_region(&self) -> Support<Reader<languages::IndentRegion, ()>> {
        // standard: TypeScript 5.9.3 reference compiler scanner, scan(): horizontal
        // whitespace and newlines are trivia, with no lexical INDENT/DEDENT suite tokens.
        // https://github.com/microsoft/TypeScript/blob/v5.9.3/src/compiler/scanner.ts
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn syntax_tree(&self) -> Support<Reader<languages::SyntaxTree, ()>> {
        languages::reads::<SyntaxTreeItem>()
    }

    fn comment(&self) -> Support<Reader<languages::Comment, ()>> {
        languages::reads::<CommentItem>()
    }

    fn directive(&self) -> Support<Reader<languages::Directive, ()>> {
        languages::reads::<DirectiveItem>()
    }

    fn string_literal(&self) -> Support<Reader<languages::StringLiteral, ()>> {
        languages::reads::<StringLiteralItem>()
    }

    fn prose(&self) -> Support<Reader<languages::Prose, ()>> {
        // TypeScript AST specification models source code, not document prose or keyed data.
        languages::absent(AbsenceCause::NotADocument)
    }

    fn syntax_child(&self) -> Support<Reader<languages::SyntaxChild, ()>> {
        languages::reads::<SyntaxChildItem>()
    }

    fn syntax_error(&self) -> Support<Reader<languages::SyntaxError, ()>> {
        languages::reads::<SyntaxErrorItem>()
    }

    fn mapping(&self) -> Support<Reader<languages::Mapping, ()>> {
        // TypeScript AST specification models source code, not document prose or keyed data.
        languages::absent(AbsenceCause::NotAKeyedDocument)
    }

    fn entry(&self) -> Support<Reader<languages::Entry, ()>> {
        // TypeScript AST specification models source code, not document prose or keyed data.
        languages::absent(AbsenceCause::NotAKeyedDocument)
    }

    fn sequence_element(&self) -> Support<Reader<languages::SequenceElement, ()>> {
        // TypeScript AST specification models source code, not document prose or keyed data.
        languages::absent(AbsenceCause::NotAKeyedDocument)
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
        // standard: @typescript-eslint/ast-spec AST_NODE_TYPES has no macro invocation or token-tree construct.
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
        languages::reads::<CallArgumentItem>()
    }

    fn semantic_syntax(&self) -> Support<Reader<languages::SemanticSyntax, ()>> {
        languages::native_semantic_syntax()
    }

    fn semantic_child(&self) -> Support<Reader<languages::SemanticChild, ()>> {
        languages::native_semantic_child()
    }

    fn resolves(&self) -> Support<Reader<languages::Resolves, ()>> {
        languages::reads::<ResolvesItem>()
    }

    fn static_import(&self) -> Support<Reader<languages::StaticImport, ()>> {
        languages::reads::<StaticImportItem>()
    }
}
