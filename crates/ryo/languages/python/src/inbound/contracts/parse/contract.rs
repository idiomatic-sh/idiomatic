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
use crate::outbound::syntax::types::{PythonSourceKind, SourceText};
use crate::settings::PythonParserSettings;

/// `parse`, implementing `ryo::languages::Parse`.
pub(crate) struct ParseContract {
    /// Immutable parser settings shared with the syntax adapter and the cache identity.
    settings: PythonParserSettings,
    /// The parser that reads Python text into the tree.
    syntax: Arc<dyn SyntaxPort>,
    /// What writes the notes of a file as bytes.
    notes: Arc<dyn NotesPort>,
}

/// Select stubs by the exact lowercase `.pyi` suffix used by the existing parser contract.
fn source_kind(path: &str) -> PythonSourceKind {
    if std::path::Path::new(path)
        .extension()
        .is_some_and(|extension| extension == std::ffi::OsStr::new("pyi"))
    {
        PythonSourceKind::Stub
    } else {
        PythonSourceKind::Python
    }
}

impl ParseContract {
    /// `parse` over the ports it calls, which the edge injects.
    #[must_use]
    pub(crate) fn new(
        settings: PythonParserSettings,
        syntax: Arc<dyn SyntaxPort>,
        notes: Arc<dyn NotesPort>,
    ) -> Self {
        Self {
            settings,
            syntax,
            notes,
        }
    }
}

impl Parse for ParseContract {
    type FileModel = Declarations;

    fn capabilities(&self) -> &'static [languages::OptionalCapability] {
        &[
            languages::OptionalCapability::ModuleIdentity,
            languages::OptionalCapability::Identifier,
            languages::OptionalCapability::QualifiedName,
            languages::OptionalCapability::EmbeddedExpression,
            languages::OptionalCapability::ExpressionShape,
            languages::OptionalCapability::BindingEvent,
            languages::OptionalCapability::BindingRead,
            languages::OptionalCapability::BindingUse,
            languages::OptionalCapability::BindingShadow,
            languages::OptionalCapability::ImplicitSymbol,
            languages::OptionalCapability::LiteralProperties,
            languages::OptionalCapability::NumericLiteral,
            languages::OptionalCapability::InterpolatedStringLiteral,
            languages::OptionalCapability::BytesLiteral,
            languages::OptionalCapability::FormatSpecification,
            languages::OptionalCapability::ConstantKey,
            languages::OptionalCapability::FormatParse,
            languages::OptionalCapability::FormatField,
            languages::OptionalCapability::RawFormatSegment,
            languages::OptionalCapability::RawFormatField,
            languages::OptionalCapability::LiteralExpressionParse,
            languages::OptionalCapability::BraceFormatParse,
            languages::OptionalCapability::BraceFormatField,
            languages::OptionalCapability::LanguageTarget,
            languages::OptionalCapability::CompilerFeature,
            languages::OptionalCapability::FunctionTerminal,
            languages::OptionalCapability::ExpressionType,
        ]
    }

    fn configuration_identity(&self) -> languages::ParseConfigurationIdentity<'_> {
        languages::ParseConfigurationIdentity(self.settings.identity())
    }

    fn file_model(&self, tree: &languages::Tree<'_>, model: &mut Declarations) {
        model.language_target = self.settings.target_version().components();
        crate::domain::expression_shapes::service::clear(model.expression_shapes.get_mut());
        crate::domain::constant_keys::service::clear(model.constant_keys.get_mut());
        crate::domain::format_parsing::service::clear(model.format_parsing.get_mut());
        crate::domain::raw_format::service::clear(model.raw_format.get_mut());
        crate::domain::literal_expression_parsing::service::clear(
            model.literal_expression_parsing.get_mut(),
        );
        crate::domain::brace_format::service::clear(model.brace_format.get_mut());
        crate::domain::expression_types::service::clear(model.expression_types.get_mut());
        crate::domain::terminal_flow::service::clear(model.terminal_flow.get_mut());
        crate::domain::indentation::service::clear(model.indentation.get_mut());
        crate::domain::qualification::service::clear(model.qualification.get_mut());
        crate::domain::binding_analysis::service::clear(model.binding_analysis.get_mut());
        model.semantic.get_mut().roots.clear();
        model.semantic.get_mut().state = crate::domain::model::types::QualificationState::Fresh;
        declare(tree, model);
        crate::domain::bindings::service::index(tree, model);
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        UNKNOWN_REASONS
    }

    fn recognizes(&self, request: &languages::ParseRequest<'_>) -> languages::Recognition {
        reads(request.path)
    }

    fn recognizes_explicit(
        &self,
        _request: &languages::ParseRequest<'_>,
    ) -> languages::Recognition {
        languages::Recognition::Reads
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
            source_kind(request.path.0),
            &self.settings,
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

    fn embedded_expression(
        &self,
    ) -> Support<Reader<languages::EmbeddedExpression, Self::FileModel>> {
        languages::reads::<crate::domain::relations::collections::relations::embedded_expression::item::EmbeddedExpressionItem>()
    }

    fn semantic_syntax(&self) -> Support<Reader<languages::SemanticSyntax, Self::FileModel>> {
        languages::reads::<crate::domain::relations::collections::relations::semantic_syntax::item::SemanticSyntaxItem>()
    }

    fn semantic_child(&self) -> Support<Reader<languages::SemanticChild, Self::FileModel>> {
        languages::reads::<crate::domain::relations::collections::relations::semantic_child::item::SemanticChildItem>()
    }

    fn expression_shape(&self) -> Support<Reader<languages::ExpressionShape, Declarations>> {
        languages::reads::<crate::domain::relations::collections::relations::expression_shape::item::ExpressionShapeItem>()
    }

    fn constant_key(&self) -> Support<Reader<languages::ConstantKey, Self::FileModel>> {
        languages::reads::<
            crate::domain::relations::collections::relations::constant_key::item::ConstantKeyItem,
        >()
    }

    fn format_parse(&self) -> Support<Reader<languages::FormatParse, Self::FileModel>> {
        languages::reads::<super::format_readers::FormatParseItem>()
    }

    fn literal_expression_parse(
        &self,
    ) -> Support<Reader<languages::LiteralExpressionParse, Self::FileModel>> {
        languages::reads::<super::literal_expression_readers::LiteralExpressionParseItem>()
    }

    fn raw_format_segment(&self) -> Support<Reader<languages::RawFormatSegment, Self::FileModel>> {
        languages::reads::<super::raw_format_readers::RawFormatSegmentItem>()
    }

    fn raw_format_field(&self) -> Support<Reader<languages::RawFormatField, Self::FileModel>> {
        languages::reads::<super::raw_format_readers::RawFormatFieldItem>()
    }

    fn format_field(&self) -> Support<Reader<languages::FormatField, Self::FileModel>> {
        languages::reads::<super::format_readers::FormatFieldItem>()
    }

    fn brace_format_parse(&self) -> Support<Reader<languages::BraceFormatParse, Self::FileModel>> {
        languages::reads::<super::brace_format_readers::BraceFormatParseItem>()
    }

    fn brace_format_field(&self) -> Support<Reader<languages::BraceFormatField, Self::FileModel>> {
        languages::reads::<super::brace_format_readers::BraceFormatFieldItem>()
    }

    fn function_terminal(&self) -> Support<Reader<languages::FunctionTerminal, Self::FileModel>> {
        languages::reads::<crate::domain::relations::collections::relations::function_terminal::item::FunctionTerminalItem>()
    }

    fn expression_type(&self) -> Support<Reader<languages::ExpressionType, Self::FileModel>> {
        languages::reads::<crate::domain::relations::collections::relations::expression_type::item::ExpressionTypeItem>()
    }

    fn literal_properties(&self) -> Support<Reader<languages::LiteralProperties, Self::FileModel>> {
        languages::reads::<
            crate::domain::relations::collections::relations::literal_properties::item::LiteralPropertiesItem,
        >()
    }

    fn numeric_literal(&self) -> Support<Reader<languages::NumericLiteral, Self::FileModel>> {
        languages::reads::<
            crate::domain::relations::collections::relations::numeric_literal::item::NumericLiteralItem,
        >()
    }

    fn format_specification(
        &self,
    ) -> Support<Reader<languages::FormatSpecification, Self::FileModel>> {
        languages::reads::<super::format_specification_readers::FormatSpecificationItem>()
    }

    fn bytes_literal(&self) -> Support<Reader<languages::BytesLiteral, Self::FileModel>> {
        languages::reads::<
            crate::domain::relations::collections::relations::bytes_literal::item::BytesLiteralItem,
        >()
    }

    fn interpolated_string_literal(
        &self,
    ) -> Support<Reader<languages::InterpolatedStringLiteral, Self::FileModel>> {
        languages::reads::<
            crate::domain::relations::collections::relations::interpolated_string_literal::item::InterpolatedStringLiteralItem,
        >()
    }

    fn compiler_feature(&self) -> Support<Reader<languages::CompilerFeature, Self::FileModel>> {
        languages::reads::<crate::domain::relations::collections::relations::compiler_feature::item::CompilerFeatureItem>()
    }

    fn language_target(&self) -> Support<Reader<languages::LanguageTarget, Self::FileModel>> {
        languages::reads::<
            crate::domain::relations::collections::relations::language_target::item::LanguageTargetItem,
        >()
    }

    fn implicit_symbol(&self) -> Support<Reader<languages::ImplicitSymbol, Self::FileModel>> {
        languages::reads::<
            crate::domain::relations::collections::relations::implicit_symbol::item::ImplicitSymbolItem,
        >()
    }

    fn binding_event(&self) -> Support<Reader<languages::BindingEvent, Self::FileModel>> {
        languages::reads::<
            crate::domain::relations::collections::relations::binding_event::item::BindingEventItem,
        >()
    }

    fn binding_read(&self) -> Support<Reader<languages::BindingRead, Self::FileModel>> {
        languages::reads::<
            crate::domain::relations::collections::relations::binding_read::item::BindingReadItem,
        >()
    }

    fn binding_use(&self) -> Support<Reader<languages::BindingUse, Self::FileModel>> {
        languages::reads::<
            crate::domain::relations::collections::relations::binding_use::item::BindingUseItem,
        >()
    }

    fn binding_shadow(&self) -> Support<Reader<languages::BindingShadow, Self::FileModel>> {
        languages::reads::<
            crate::domain::relations::collections::relations::binding_shadow::item::BindingShadowItem,
        >()
    }

    fn identifier(&self) -> Support<Reader<languages::Identifier, Declarations>> {
        languages::reads::<
            crate::domain::relations::collections::relations::identifier::item::IdentifierItem,
        >()
    }

    fn qualified_name(&self) -> Support<Reader<languages::QualifiedName, Declarations>> {
        languages::reads::<crate::domain::relations::collections::relations::qualified_name::item::QualifiedNameItem>()
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
