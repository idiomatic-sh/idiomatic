//! The fillers of the `walk` contract: for every relation of ryo's catalog the engine fills, how Python fills it from a walked
//! file (its `fills!` item, whole or for one key), or that it is not in the language, with the cause; and the names
//! Python states for the whole run: its kinds, its fields and the fixed names its fillers write by handle.

use crate::domain::file_view::types::{PythonFile, PythonWalk};
use crate::domain::labels::service::{field_name, fields};
use crate::domain::qualification::constants::{
    AMBIGUOUS, BUILTIN, BUILTIN_NAMES, LOCAL, UNRESOLVED,
};
use crate::domain::relations::collections::relations::implicit_symbol::item::{
    BUILTIN_ATTRIBUTES, MODULE_GLOBALS,
};
use crate::domain::rendering::constants::UNRENDERED;
use crate::domain::rendering::service::{render, target};
use crate::domain::syntax::service::{kind_name, kinds};
use ryo::languages::{
    self, AbsenceCause, CatalogRelation, FieldNames, FileCompleteness, FileWords, Fill, Fillers, FixedNames, Flag,
    NameText, RenderContext, RenderOutcome, RenderRequest, RenderedNode, Rows, WalkKinds,
    WalkVocabulary, Wanted, fillers,
};
use std::collections::HashSet;
use std::sync::Arc;

/// The filler of each relation whose `fills!` item writes it whole alone: the item's run. Asked for a key, it writes the whole
/// relation and says so; `fillers::filled` is the one place that answer is written.
macro_rules! written {
    ($( $module:ident: $relation:ident => $item:path; )*) => {
        $(
            fn $module(
                file: &mut PythonFile<'_>,
                words: &mut FileWords,
                wanted: Wanted,
                rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                fillers::filled::<$item>(file, words, wanted, rows)
            }
        )*
    };
}

/// The filler of each relation whose `fills!` item has a key clause for every column the catalog declares among the relation's
/// keys: the item's run, whole or for one key.
macro_rules! keyed {
    ($( $module:ident: $relation:ident => $item:path; )*) => {
        $(
            fn $module(
                file: &mut PythonFile<'_>,
                words: &mut FileWords,
                wanted: Wanted,
                rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                fillers::filled_keyed::<$item>(file, words, wanted, rows)
            }
        )*
    };
}

/// The filler of each relation Python does not fill, with the cause.
macro_rules! absent {
    ($( $module:ident: $relation:ident => $cause:ident; )*) => {
        $(
            fn $module(
                _file: &mut PythonFile<'_>,
                _words: &mut FileWords,
                _wanted: Wanted,
                _rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                Fill::Absent(AbsenceCause::$cause)
            }
        )*
    };
}

impl<P: Send + Sync + 'static> Fillers for PythonWalk<P> {
    type File<'f>
        = PythonFile<'f>
    where
        Self: 'f;

    written! {
        binding_event: BindingEvent => crate::domain::relations::collections::relations::binding_event::item::BindingEventItem;
        binding_resolution: BindingResolution => crate::domain::relations::collections::relations::binding_resolution::item::BindingResolutionItem;
        binding_use: BindingUse => crate::domain::relations::collections::relations::binding_use::item::BindingUseItem;
        brace_format_field: BraceFormatField => crate::inbound::contracts::parse::readers::brace_format::composition::BraceFormatFieldItem;
        brace_format_parse: BraceFormatParse => crate::inbound::contracts::parse::readers::brace_format::composition::BraceFormatParseItem;
        bytes_literal: BytesLiteral => crate::domain::relations::collections::relations::bytes_literal::item::BytesLiteralItem;
        call_argument: CallArgument => crate::domain::relations::collections::relations::call_argument::item::CallArgumentItem;
        comment: Comment => crate::domain::relations::collections::relations::comment::item::CommentItem;
        compiler_feature: CompilerFeature => crate::domain::relations::collections::relations::compiler_feature::item::CompilerFeatureItem;
        decorates: Decorates => crate::domain::relations::collections::relations::decorates::item::DecoratesItem;
        defines: Defines => crate::domain::relations::collections::relations::defines::item::DefinesItem;
        embedded_expression: EmbeddedExpression => crate::domain::relations::collections::relations::embedded_expression::item::EmbeddedExpressionItem;
        format_field: FormatField => crate::inbound::contracts::parse::readers::format::composition::FormatFieldItem;
        format_parse: FormatParse => crate::inbound::contracts::parse::readers::format::composition::FormatParseItem;
        format_specification: FormatSpecification => crate::inbound::contracts::parse::readers::format_specification::composition::FormatSpecificationItem;
        function_terminal: FunctionTerminal => crate::domain::relations::collections::relations::function_terminal::item::FunctionTerminalItem;
        identifier: Identifier => crate::domain::relations::collections::relations::identifier::item::IdentifierItem;
        implicit_symbol: ImplicitSymbol => crate::domain::relations::collections::relations::implicit_symbol::item::ImplicitSymbolItem;
        indent_region: IndentRegion => crate::domain::relations::collections::relations::indent_region::item::IndentRegionItem;
        interpolated_string_literal: InterpolatedStringLiteral => crate::domain::relations::collections::relations::interpolated_string_literal::item::InterpolatedStringLiteralItem;
        language_target: LanguageTarget => crate::domain::relations::collections::relations::language_target::item::LanguageTargetItem;
        literal_expression_parse: LiteralExpressionParse => crate::inbound::contracts::parse::readers::literal_expression::composition::LiteralExpressionParseItem;
        literal_properties: LiteralProperties => crate::domain::relations::collections::relations::literal_properties::item::LiteralPropertiesItem;
        numeric_literal: NumericLiteral => crate::domain::relations::collections::relations::numeric_literal::item::NumericLiteralItem;
        parameter: Parameter => crate::domain::relations::collections::relations::parameter::item::ParameterItem;
        qualified_name: QualifiedName => crate::domain::relations::collections::relations::qualified_name::item::QualifiedNameItem;
        raw_format_field: RawFormatField => crate::inbound::contracts::parse::readers::raw_format::composition::RawFormatFieldItem;
        source_location: SourceLocation => super::source_location::SourceLocationItem;
        standard_library_import: StandardLibraryImport => crate::domain::relations::collections::relations::standard_library_import::item::StandardLibraryImportItem;
        static_import: StaticImport => crate::domain::relations::collections::relations::static_import::item::StaticImportItem;
        string_literal: StringLiteral => crate::domain::relations::collections::relations::string_literal::item::StringLiteralItem;
        syntax_error: SyntaxError => crate::domain::relations::collections::relations::syntax_error::item::SyntaxErrorItem;
    }

    keyed! {
        binding_read: BindingRead => crate::domain::relations::collections::relations::binding_read::item::BindingReadItem;
        binding_shadow: BindingShadow => crate::domain::relations::collections::relations::binding_shadow::item::BindingShadowItem;
        constant_key: ConstantKey => crate::domain::relations::collections::relations::constant_key::item::ConstantKeyItem;
        expression_shape: ExpressionShape => crate::domain::relations::collections::relations::expression_shape::item::ExpressionShapeItem;
        expression_type: ExpressionType => crate::domain::relations::collections::relations::expression_type::item::ExpressionTypeItem;
        function: Function => crate::domain::relations::collections::relations::function::item::FunctionItem;
        inherits: Inherits => crate::domain::relations::collections::relations::inherits::item::InheritsItem;
        lookup_scope: LookupScope => crate::domain::relations::collections::relations::lookup_scope::item::LookupScopeItem;
        raw_format_segment: RawFormatSegment => crate::inbound::contracts::parse::readers::raw_format::composition::RawFormatSegmentItem;
        reference: Reference => crate::domain::relations::collections::relations::reference::item::ReferenceItem;
        resolves: Resolves => crate::domain::relations::collections::relations::resolves::item::ResolvesItem;
        scope: Scope => crate::domain::relations::collections::relations::scope::item::ScopeItem;
        token_link: TokenLink => crate::inbound::contracts::parse::readers::token_link::composition::TokenLinkItem;
    }

    absent! {
        analyzed_source: AnalyzedSource => NotYetRead;
        annotates: Annotates => NotYetRead;
        assigns: Assigns => NotYetRead;
        attribute_path: AttributePath => NoSuchConstruct;
        directive: Directive => NoSuchConstruct;
        entry: Entry => NotAKeyedDocument;
        file_package: FilePackage => NotYetRead;
        filesystem_executable: FilesystemExecutable => NotYetRead;
        filesystem_file: FilesystemFile => NotYetRead;
        filesystem_line_count: FilesystemLineCount => NotYetRead;
        filesystem_read_failure: FilesystemReadFailure => NotYetRead;
        flow_block: FlowBlock => NotYetRead;
        flow_edge: FlowEdge => NotYetRead;
        flow_member: FlowMember => NotYetRead;
        flow_step: FlowStep => NotYetRead;
        manifest_diagnostic: ManifestDiagnostic => NotYetRead;
        mapping: Mapping => NotAKeyedDocument;
        mention: Mention => NoSuchConstruct;
        module_identity: ModuleIdentity => NotYetRead;
        package_dependency: PackageDependency => NotYetRead;
        pattern_child: PatternChild => NotYetRead;
        pattern_parse: PatternParse => NotYetRead;
        pattern_syntax: PatternSyntax => NotYetRead;
        project_namespace_membership: ProjectNamespaceMembership => NotYetRead;
        prose: Prose => NotADocument;
        sequence_element: SequenceElement => NotAKeyedDocument;
        source_dependency: SourceDependency => NotYetRead;
        static_call: StaticCall => NotYetRead;
        visibility: Visibility => NoSuchConstruct;
        workspace_member: WorkspaceMember => NotYetRead;
    }

    /// The node's typed expression rendered in the role: not applicable as an assignment target when it is not stored to;
    /// unsupported for a node that is no expression of the file's own tree, or a form the tree does not prove.
    fn render(file: &mut PythonFile<'_>, request: RenderRequest) -> RenderOutcome {
        let place = usize::try_from(languages::node_position(request.node).0).unwrap_or(usize::MAX);
        let Some(expr) = file
            .syntax
            .nodes
            .get(place)
            .and_then(|node| node.as_expr_ref())
        else {
            return RenderOutcome::Unsupported(UNRENDERED);
        };
        if request.context == RenderContext::AssignmentTarget && target(expr) == Flag::Lowered {
            return RenderOutcome::NotApplicable;
        }
        match render(expr, request.context, file.syntax.text) {
            Some(text) => RenderOutcome::Known(RenderedNode(Arc::from(text))),
            None => RenderOutcome::Unsupported(UNRENDERED),
        }
    }

    /// Python's names for the run: every kind at its code and every field at its code, as the walk writes them into the node
    /// table, and the fixed names its fillers write by handle.
    fn completeness(file: &PythonFile<'_>) -> FileCompleteness {
        FileCompleteness {
            syntax: file.syntax.completeness.syntax,
            lexical: file.syntax.completeness.lexical,
        }
    }

    /// The syntax was read whole, the flow of the model could be interpreted and the standard library of the target release
    /// is known: the three causes of a partly read relation.
    fn fills_are_complete(file: &mut PythonFile<'_>) -> bool {
        use crate::domain::model::view::types::ModelPart;
        use crate::domain::model::flow::types::FlowState;
        use crate::domain::standard_library::{
            service::contains,
            types::{ModulePath, StandardLibraryMembership},
        };
        use crate::domain::model::view::types::ModelView as _;
        file.syntax.completeness.syntax == languages::SyntaxCompleteness::Complete
            && contains(file.target, ModulePath("os")) != StandardLibraryMembership::UnverifiedRelease
            && file.semantics(ModelPart::Flow).model.flow.state != FlowState::Incomplete
    }

    fn notes(
        file: &mut PythonFile<'_>,
        table: &languages::NodeTable,
        source: languages::ByteSlice<'_>,
        out: &mut languages::Bytes,
    ) {
        use crate::outbound::notes::interface::NotesPort as _;
        let note = crate::fixtures::columns::ColumnFile::noted(file, table, source);
        crate::outbound::notes::factory::postcard().write(&note, out);
    }

    fn spells(kind: languages::KindBits) -> bool {
        matches!(
            crate::domain::syntax::service::kind_of(kind),
            crate::domain::syntax::types::Kind::Token(_)
                | crate::domain::syntax::types::Kind::Extra(crate::domain::extras::types::Extra::Whitespace)
        )
    }

    fn named(
        file: &mut PythonFile<'_>,
        request: languages::NamedRequest<'_, '_>,
    ) -> Vec<Result<languages::FileRows, languages::ParseError>> {
        crate::fixtures::columns::ColumnFile::named(file, request)
    }

    fn vocabulary() -> WalkVocabulary {
        WalkVocabulary {
            kinds: WalkKinds(kinds().map(kind_name).collect()),
            fields: FieldNames(fields().map(field_name).collect()),
            fixed: fixed_names(),
        }
    }
}

/// Where a name comes from, as the fillers write it: the same in every file.
const PROVENANCE: &[&str] = &[BUILTIN, LOCAL, UNRESOLVED, AMBIGUOUS];

/// The texts Python's fillers write that are the same in every file: the fixed names of the run, by place. Each concept states
/// its own beside its constants; the names the language supplies to every module are read from their catalogs. A fixed name's
/// word is its place, so each text is stated once, at the place of the first concept that writes it, and the empty text, which
/// is word 0 of every run, is no fixed name.
pub(crate) fn fixed_names() -> FixedNames {
    let concepts: [&[&'static str]; 17] = [
        crate::domain::token_links::constants::FIXED_NAMES,
        crate::domain::relations::collections::relations::expression_type::constants::FIXED_NAMES,
        crate::domain::expression_types::constants::FIXED_NAMES,
        crate::domain::literal_properties::constants::FIXED_NAMES,
        crate::domain::semantic_syntax::constants::FIXED_NAMES,
        crate::domain::relations::collections::relations::compiler_feature::constants::FIXED_NAMES,
        crate::domain::format_specification::constants::FIXED_NAMES,
        crate::inbound::contracts::parse::readers::format_specification::constants::FIXED_NAMES,
        crate::inbound::contracts::parse::readers::format::constants::FIXED_NAMES,
        crate::inbound::contracts::parse::readers::literal_expression::constants::FIXED_NAMES,
        crate::domain::literal_expression_parsing::constants::FIXED_NAMES,
        &crate::domain::format_syntax::constants::FLAG_SPELLINGS,
        crate::domain::definitions::constants::FIXED_NAMES,
        crate::domain::terminal_flow::constants::FIXED_NAMES,
        crate::inbound::contracts::parse::readers::brace_format::constants::FIXED_NAMES,
        crate::domain::binding_results::constants::FIXED_NAMES,
        PROVENANCE,
    ];
    let implicit =
        crate::domain::relations::collections::relations::implicit_symbol::constants::FIXED_NAMES;
    let catalogs = BUILTIN_ATTRIBUTES
        .iter()
        .chain(MODULE_GLOBALS)
        .map(|entry| entry.name);
    let mut stated = HashSet::new();
    FixedNames(
        concepts
            .into_iter()
            .chain([implicit])
            .flatten()
            .map(|text| NameText(text))
            .chain(BUILTIN_NAMES.iter().copied())
            .chain(catalogs)
            .filter(|name| !name.0.is_empty() && stated.insert(name.0))
            .collect(),
    )
}
