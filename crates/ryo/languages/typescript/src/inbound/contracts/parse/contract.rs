//! The `parse` contract: the language's implementation of `ryo::languages::Parse`. It says which files are the language's, builds the file's
//! tree in the worker's element table through the syntax port, notes the modules the file imports when the run asks for its notes,
//! and answers, for every syntax relation of ryo's catalog, how the language reads it or that it is not in the language yet.

use crate::domain::fact_notes::service::noted;
use crate::domain::fields::service::{field_bits, field_name, fields, role_of};
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
use crate::domain::relations::collections::relations::visibility::item::VisibilityItem;
use std::sync::Arc;

use ryo::languages::{self, AbsenceCause, Parse, Reader, Support, UnknownName};

use crate::domain::language::service::{dialect, reads};
use crate::domain::relations::collections::relations::syntax_child::item::SyntaxChildItem;
use crate::domain::relations::collections::relations::syntax_error::item::SyntaxErrorItem;
use crate::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeItem;
use crate::domain::syntax::service::{kind_bits, kind_name, kinds};
use crate::outbound::embedded::interface::EmbeddedPort;
use crate::outbound::embedded::types::{BlockLanguage, Blocks, Markup};
use crate::outbound::notes::interface::NotesPort;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;

/// `parse`, implementing `ryo::languages::Parse`.
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
    fn blocks_of<'a>(&self, request: &languages::ParseRequest<'a>) -> Option<Blocks<'a>> {
        let adapter = self
            .embedded
            .iter()
            .find(|adapter| adapter.claims(request.path) == languages::Recognition::Reads)?;
        Some(adapter.blocks(Markup(
            std::str::from_utf8(request.bytes.0).unwrap_or_default(),
        )))
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
        crate::domain::language::constants::UNKNOWN_REASONS
    }

    fn recognizes(&self, request: &languages::ParseRequest<'_>) -> languages::Recognition {
        if reads(request.path) == languages::Recognition::Reads {
            return languages::Recognition::Reads;
        }
        // The two languages share the files that carry blocks: this one reads a file with a typed block, the script language the rest.
        match self.blocks_of(request) {
            Some(blocks)
                if blocks
                    .0
                    .iter()
                    .any(|block| block.dialect != BlockLanguage::Script) =>
            {
                languages::Recognition::Reads
            }
            Some(_) | None => languages::Recognition::Declines,
        }
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
        let Some(blocks) = self
            .blocks_of(request)
            .filter(|_| reads(request.path) == languages::Recognition::Declines)
        else {
            self.syntax
                .read(SourceText(request.bytes.0), dialect(request.path), builder);
            return Ok(());
        };
        // Everything outside the blocks reads as white space, line ends kept, so every offset and line of the tree is the file's and a
        // line comment ends at its own line, not at the next block.
        let mut script: Vec<u8> = request
            .bytes
            .0
            .iter()
            .map(|byte| {
                if matches!(byte, b'\n' | b'\r') {
                    *byte
                } else {
                    b' '
                }
            })
            .collect();
        for block in &blocks.0 {
            // A block is a stretch of these same bytes, so its place is always inside the script.
            if let Some(place) = script.get_mut(block.start.0..block.start.0 + block.text.0.len()) {
                place.copy_from_slice(block.text.0.as_bytes());
            }
        }
        // The blocks are read together as TypeScript with JSX when one is, and as plain TypeScript when none is.
        let read_as = if blocks
            .0
            .iter()
            .any(|block| block.dialect == BlockLanguage::Tsx)
        {
            crate::domain::syntax::types::Dialect::Tsx
        } else {
            crate::domain::syntax::types::Dialect::TypeScript
        };
        self.syntax.read(SourceText(&script), read_as, builder);
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
        languages::reads::<
            crate::domain::relations::collections::relations::directive::item::DirectiveItem,
        >()
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
        languages::reads::<
            crate::domain::relations::collections::relations::call_argument::item::CallArgumentItem,
        >()
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

    fn interpolated_string_literal(
        &self,
    ) -> Support<Reader<languages::InterpolatedStringLiteral, Self::FileModel>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn literal_properties(&self) -> Support<Reader<languages::LiteralProperties, Self::FileModel>> {
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
        languages::reads::<
            crate::domain::relations::collections::relations::resolves::item::ResolvesItem,
        >()
    }

    fn static_import(&self) -> Support<Reader<languages::StaticImport, ()>> {
        languages::reads::<StaticImportItem>()
    }
}
