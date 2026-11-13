// ignore: idiomatic.arch/file-length - one impl of the generated parse trait, one method for each relation of the catalog: the impl is one block and the catalog is its list, so no placed role splits it
//! The `parse` contract: the language's implementation of `ryo::Parse`. It says which files are the language's, builds the file's
//! tree in the worker's element table through the syntax port, notes the modules the file imports when the run asks for its notes,
//! and answers, for every syntax relation of ryo's catalog, how the language reads it or that it is not in the language yet.

use crate::domain::fact_notes::service::noted;
use crate::domain::fields::service::name_vocabulary;
use crate::domain::model::{service::clear, types::ReadIndexes};
use crate::domain::relations::collections::relations::{
    annotates::item::AnnotatesItem, assigns::item::AssignsItem,
    binding_read::item::BindingReadItem, binding_use::item::BindingUseItem,
    call_argument::item::CallArgumentItem, comment::item::CommentItem,
    decorates::item::DecoratesItem, defines::item::DefinesItem, directive::item::DirectiveItem,
    expression_shape::item::ExpressionShapeItem, function::item::FunctionItem,
    inherits::item::InheritsItem, numeric_literal::item::NumericLiteralItem,
    parameter::item::ParameterItem, reference::item::ReferenceItem, resolves::item::ResolvesItem,
    scope::item::ScopeItem, static_call::item::StaticCallItem,
    static_import::item::StaticImportItem, string_literal::item::StringLiteralItem,
    syntax_child::item::SyntaxChildItem, syntax_error::item::SyntaxErrorItem,
    syntax_tree::item::SyntaxTreeItem, visibility::item::VisibilityItem,
};
use std::sync::Arc;

use ryo::languages::{
    self, AbsenceCause, ByteSlice, Completion, CompletionRequirement, Count, Elements, KindNames,
    NoteRequest, OptionalCapability, Parse, ParseConfigurationIdentity, ParseError, ParseRequest,
    Position, Reader, Recognition, SemanticSlot, SkippedDirectory, Support, TableBuilder, Tree,
    UnknownName,
};

use crate::domain::language::service::{reading_of, reads, recognized};
use crate::domain::model::{service::clear, types::ReadIndexes};
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

    fn capabilities(&self) -> &'static [OptionalCapability] {
        &[]
    }

    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        ParseConfigurationIdentity(NO_CONFIGURATION)
    }

    fn file_model(&self, _tree: &Tree<'_>, model: &mut ReadIndexes) {
        clear(model);
    }

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

    fn source_location(&self) -> Support<Reader<languages::SourceLocation, ReadIndexes>> {
        languages::source_locations()
    }

    fn indent_region(&self) -> Support<Reader<languages::IndentRegion, ReadIndexes>> {
        // standard: TypeScript 5.9.3 reference compiler scanner, scan(): horizontal
        // whitespace and newlines are trivia, with no lexical INDENT/DEDENT suite tokens.
        // https://github.com/microsoft/TypeScript/blob/v5.9.3/src/compiler/scanner.ts
        languages::absent(AbsenceCause::NoSuchConstruct)
    }

    fn syntax_tree(&self) -> Support<Reader<languages::SyntaxTree, ReadIndexes>> {
        languages::reads::<SyntaxTreeItem>()
    }

    fn comment(&self) -> Support<Reader<languages::Comment, ReadIndexes>> {
        languages::reads::<CommentItem>()
    }

    fn directive(&self) -> Support<Reader<languages::Directive, ReadIndexes>> {
        languages::reads::<DirectiveItem>()
    }

    fn string_literal(&self) -> Support<Reader<languages::StringLiteral, ReadIndexes>> {
        languages::reads::<StringLiteralItem>()
    }

    fn prose(&self) -> Support<Reader<languages::Prose, ReadIndexes>> {
        // TypeScript AST specification models source code, not document prose or keyed data.
        languages::absent(AbsenceCause::NotADocument)
    }

    fn syntax_child(&self) -> Support<Reader<languages::SyntaxChild, ReadIndexes>> {
        languages::reads::<SyntaxChildItem>()
    }

    fn syntax_error(&self) -> Support<Reader<languages::SyntaxError, ReadIndexes>> {
        languages::reads::<SyntaxErrorItem>()
    }

    fn mapping(&self) -> Support<Reader<languages::Mapping, ReadIndexes>> {
        // TypeScript AST specification models source code, not document prose or keyed data.
        languages::absent(AbsenceCause::NotAKeyedDocument)
    }

    fn entry(&self) -> Support<Reader<languages::Entry, ReadIndexes>> {
        // TypeScript AST specification models source code, not document prose or keyed data.
        languages::absent(AbsenceCause::NotAKeyedDocument)
    }

    fn sequence_element(&self) -> Support<Reader<languages::SequenceElement, ReadIndexes>> {
        // TypeScript AST specification models source code, not document prose or keyed data.
        languages::absent(AbsenceCause::NotAKeyedDocument)
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
        // standard: @typescript-eslint/ast-spec AST_NODE_TYPES has no macro invocation or token-tree construct.
        languages::absent(AbsenceCause::NoSuchConstruct)
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

    fn literal_expression_parse(&self) -> Support<Reader<languages::LiteralExpressionParse, ReadIndexes>> {
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
        // This provider does not expose a parser target version.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn implicit_symbol(&self) -> Support<Reader<languages::ImplicitSymbol, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_event(&self) -> Support<Reader<languages::BindingEvent, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn binding_read(&self) -> Support<Reader<languages::BindingRead, ReadIndexes>> {
        languages::reads::<BindingReadItem>()
    }

    fn binding_use(&self) -> Support<Reader<languages::BindingUse, ReadIndexes>> {
        languages::reads::<BindingUseItem>()
    }

    fn binding_shadow(&self) -> Support<Reader<languages::BindingShadow, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn identifier(&self) -> Support<Reader<languages::Identifier, ReadIndexes>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn qualified_name(&self) -> Support<Reader<languages::QualifiedName, ReadIndexes>> {
        // Qualified source identities are applicable, but this provider has not implemented their reader.
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn standard_library_import(
        &self,
    ) -> Support<Reader<languages::StandardLibraryImport, ReadIndexes>> {
        // TypeScript module imports exist; a versioned host-library catalog is not read.
        languages::absent(AbsenceCause::NotYetRead)
    }
}
