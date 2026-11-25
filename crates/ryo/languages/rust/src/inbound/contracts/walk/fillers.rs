//! The fillers of the `walk` contract: for every relation of ryo's catalog the engine fills, how Rust fills it from a walked file
//! (its `fills!` item, whole or for one key), or that it is not in the language, with the cause; and the names Rust states for the
//! whole run: its kinds and its fields.

use crate::domain::fields::service::{field_name, fields};
use crate::domain::file_view::types::{RustFile, RustWalk};
use crate::domain::syntax::service::{kind_name, kinds};
use ryo::languages::{
    self, AbsenceCause, CatalogRelation, FieldNames, FileWords, Fill, Fillers, FixedNames, NameText, RenderOutcome,
    RenderRequest, Rows, WalkKinds, WalkVocabulary, Wanted, fillers,
};

/// The filler of each relation whose `fills!` item writes it whole alone: the item's run. Asked for a key, it writes the whole
/// relation and says so; `fillers::filled` is the one place that answer is written.
macro_rules! written {
    ($( $module:ident: $relation:ident => $item:path; )*) => {
        $(
            fn $module(
                file: &mut RustFile<'_>,
                words: &mut FileWords,
                wanted: Wanted,
                rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                fillers::filled::<$item>(file, words, wanted, rows)
            }
        )*
    };
}

/// The filler of each relation Rust does not fill yet, with the cause.
macro_rules! absent {
    ($( $module:ident: $relation:ident => $cause:ident; )*) => {
        $(
            fn $module(
                _file: &mut RustFile<'_>,
                _words: &mut FileWords,
                _wanted: Wanted,
                _rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                Fill::Absent(AbsenceCause::$cause)
            }
        )*
    };
}

impl<P: Send + Sync + 'static> Fillers for RustWalk<P> {
    type File<'f>
        = RustFile<'f>
    where
        Self: 'f;

    written! {
        annotates: Annotates => crate::domain::relations::collections::relations::annotates::item::AnnotatesItem;
        assigns: Assigns => crate::domain::relations::collections::relations::assigns::item::AssignsItem;
        attribute_path: AttributePath => crate::domain::relations::collections::relations::attribute_path::item::AttributePathItem;
        call_argument: CallArgument => crate::domain::relations::collections::relations::call_argument::item::CallArgumentItem;
        comment: Comment => crate::domain::relations::collections::relations::comment::item::CommentItem;
        decorates: Decorates => crate::domain::relations::collections::relations::decorates::item::DecoratesItem;
        defines: Defines => crate::domain::relations::collections::relations::defines::item::DefinesItem;
        expression_shape: ExpressionShape => crate::domain::relations::collections::relations::expression_shape::item::ExpressionShapeItem;
        function: Function => crate::domain::relations::collections::relations::function::item::FunctionItem;
        inherits: Inherits => crate::domain::relations::collections::relations::inherits::item::InheritsItem;
        mention: Mention => crate::domain::relations::collections::relations::mention::item::MentionItem;
        numeric_literal: NumericLiteral => crate::domain::relations::collections::relations::numeric_literal::item::NumericLiteralItem;
        parameter: Parameter => crate::domain::relations::collections::relations::parameter::item::ParameterItem;
        reference: Reference => crate::domain::relations::collections::relations::reference::item::ReferenceItem;
        resolves: Resolves => crate::domain::relations::collections::relations::resolves::item::ResolvesItem;
        scope: Scope => crate::domain::relations::collections::relations::scope::item::ScopeItem;
        static_call: StaticCall => crate::domain::relations::collections::relations::static_call::item::StaticCallItem;
        static_import: StaticImport => crate::domain::relations::collections::relations::static_import::item::StaticImportItem;
        string_literal: StringLiteral => crate::domain::relations::collections::relations::string_literal::item::StringLiteralItem;
        syntax_error: SyntaxError => crate::domain::relations::collections::relations::syntax_error::item::SyntaxErrorItem;
        source_location: SourceLocation => super::source_location::SourceLocationItem;
        visibility: Visibility => crate::domain::relations::collections::relations::visibility::item::VisibilityItem;
    }

    absent! {
        analyzed_source: AnalyzedSource => NotYetRead;
        binding_event: BindingEvent => NoSuchConstruct;
        binding_read: BindingRead => NoSuchConstruct;
        binding_resolution: BindingResolution => NoSuchConstruct;
        binding_shadow: BindingShadow => NoSuchConstruct;
        binding_use: BindingUse => NoSuchConstruct;
        brace_format_field: BraceFormatField => NoSuchConstruct;
        brace_format_parse: BraceFormatParse => NoSuchConstruct;
        bytes_literal: BytesLiteral => NoSuchConstruct;
        compiler_feature: CompilerFeature => NoSuchConstruct;
        constant_key: ConstantKey => NoSuchConstruct;
        directive: Directive => NoSuchConstruct;
        embedded_expression: EmbeddedExpression => NoSuchConstruct;
        entry: Entry => NotAKeyedDocument;
        expression_type: ExpressionType => NoSuchConstruct;
        file_package: FilePackage => NotYetRead;
        filesystem_executable: FilesystemExecutable => NotYetRead;
        filesystem_file: FilesystemFile => NotYetRead;
        filesystem_line_count: FilesystemLineCount => NotYetRead;
        filesystem_read_failure: FilesystemReadFailure => NotYetRead;
        flow_block: FlowBlock => NoSuchConstruct;
        flow_edge: FlowEdge => NoSuchConstruct;
        flow_member: FlowMember => NoSuchConstruct;
        flow_step: FlowStep => NoSuchConstruct;
        format_field: FormatField => NoSuchConstruct;
        format_parse: FormatParse => NoSuchConstruct;
        format_specification: FormatSpecification => NoSuchConstruct;
        function_terminal: FunctionTerminal => NoSuchConstruct;
        identifier: Identifier => NoSuchConstruct;
        implicit_symbol: ImplicitSymbol => NoSuchConstruct;
        indent_region: IndentRegion => NoSuchConstruct;
        interpolated_string_literal: InterpolatedStringLiteral => NoSuchConstruct;
        language_target: LanguageTarget => NoSuchConstruct;
        literal_expression_parse: LiteralExpressionParse => NoSuchConstruct;
        literal_properties: LiteralProperties => NoSuchConstruct;
        lookup_scope: LookupScope => NoSuchConstruct;
        manifest_diagnostic: ManifestDiagnostic => NotYetRead;
        mapping: Mapping => NotAKeyedDocument;
        module_identity: ModuleIdentity => NotYetRead;
        package_dependency: PackageDependency => NotYetRead;
        pattern_child: PatternChild => NoSuchConstruct;
        pattern_parse: PatternParse => NoSuchConstruct;
        pattern_syntax: PatternSyntax => NoSuchConstruct;
        project_namespace_membership: ProjectNamespaceMembership => NotYetRead;
        prose: Prose => NotADocument;
        qualified_name: QualifiedName => NoSuchConstruct;
        raw_format_field: RawFormatField => NoSuchConstruct;
        raw_format_segment: RawFormatSegment => NoSuchConstruct;
        sequence_element: SequenceElement => NotAKeyedDocument;
        source_dependency: SourceDependency => NotYetRead;
        standard_library_import: StandardLibraryImport => NoSuchConstruct;
        token_link: TokenLink => NoSuchConstruct;
        workspace_member: WorkspaceMember => NotYetRead;
    }

    /// Rust renders no node: the language answers every request with what it cannot prove.
    fn render(_file: &mut RustFile<'_>, _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::Unsupported(languages::UnknownReason(24))
    }

    /// Rust's names for the run: every kind at its code and every field at its code, as the walk writes them into the node table.
    fn vocabulary() -> WalkVocabulary {
        WalkVocabulary {
            kinds: WalkKinds(kinds().map(kind_name).collect()),
            fields: FieldNames(fields().map(field_name).collect()),
            fixed: FixedNames(Vec::<NameText<'static>>::new()),
        }
    }
}
