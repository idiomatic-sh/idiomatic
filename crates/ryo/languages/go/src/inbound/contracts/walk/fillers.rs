//! The fillers of the `walk` contract: for every relation of ryo's catalog the engine fills, how Go fills it from a walked file
//! (its `fills!` item), or that it is not in the language, with the cause; and the names Go states for the whole run: its kinds
//! and its fields. Go's fillers write no fixed name.

use crate::domain::file_view::types::{GoFile, GoWalk};
use crate::domain::roles::service::{field_name, fields};
use crate::domain::syntax::service::{kind_name, kinds};
use ryo::languages::{
    self, AbsenceCause, CatalogRelation, FieldNames, FileWords, Fill, Fillers, FixedNames,
    RenderOutcome, RenderRequest, Rows, WalkKinds, WalkVocabulary, Wanted, fillers,
};

/// The filler of each relation whose `fills!` item writes it whole alone: the item's run. Asked for a key, it writes the whole
/// relation and says so; `fillers::filled` is the one place that answer is written.
macro_rules! written {
    ($( $module:ident: $relation:ident => $item:path; )*) => {
        $(
            fn $module(
                file: &mut GoFile<'_>,
                words: &mut FileWords,
                wanted: Wanted,
                rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                fillers::filled::<$item>(file, words, wanted, rows)
            }
        )*
    };
}

/// The filler of each relation Go does not fill, with the cause.
macro_rules! absent {
    ($( $module:ident: $relation:ident => $cause:ident; )*) => {
        $(
            fn $module(
                _file: &mut GoFile<'_>,
                _words: &mut FileWords,
                _wanted: Wanted,
                _rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                Fill::Absent(AbsenceCause::$cause)
            }
        )*
    };
}

impl<P: Send + Sync + 'static> Fillers for GoWalk<P> {
    type File<'f>
        = GoFile<'f>
    where
        Self: 'f;

    written! {
        source_location: SourceLocation => crate::domain::relations::collections::relations::source_location::item::SourceLocationItem;
        defines: Defines => crate::domain::relations::collections::relations::defines::item::DefinesItem;
        reference: Reference => crate::domain::relations::collections::relations::reference::item::ReferenceItem;
        resolves: Resolves => crate::domain::relations::collections::relations::resolves::item::ResolvesItem;
        parameter: Parameter => crate::domain::relations::collections::relations::parameter::item::ParameterItem;
        inherits: Inherits => crate::domain::relations::collections::relations::inherits::item::InheritsItem;
        annotates: Annotates => crate::domain::relations::collections::relations::annotates::item::AnnotatesItem;
        assigns: Assigns => crate::domain::relations::collections::relations::assigns::item::AssignsItem;
        static_call: StaticCall => crate::domain::relations::collections::relations::static_call::item::StaticCallItem;
        call_argument: CallArgument => crate::domain::relations::collections::relations::call_argument::item::CallArgumentItem;
        entry: Entry => crate::domain::relations::collections::relations::entry::item::EntryItem;
        function: Function => crate::domain::relations::collections::relations::function::item::FunctionItem;
        mapping: Mapping => crate::domain::relations::collections::relations::mapping::item::MappingItem;
        scope: Scope => crate::domain::relations::collections::relations::scope::item::ScopeItem;
        comment: Comment => crate::domain::relations::collections::relations::comment::item::CommentItem;
        directive: Directive => crate::domain::relations::collections::relations::directive::item::DirectiveItem;
        static_import: StaticImport => crate::domain::relations::collections::relations::static_import::item::StaticImportItem;
        string_literal: StringLiteral => crate::domain::relations::collections::relations::string_literal::item::StringLiteralItem;
        syntax_error: SyntaxError => crate::domain::relations::collections::relations::syntax_error::item::SyntaxErrorItem;
    }

    absent! {
        analyzed_source: AnalyzedSource => NotYetRead;
        attribute_path: AttributePath => NoSuchConstruct;
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
        decorates: Decorates => NoSuchConstruct;
        embedded_expression: EmbeddedExpression => NoSuchConstruct;
        expression_shape: ExpressionShape => NoSuchConstruct;
        expression_type: ExpressionType => NoSuchConstruct;
        file_package: FilePackage => NotYetRead;
        filesystem_executable: FilesystemExecutable => NotYetRead;
        filesystem_file: FilesystemFile => NotYetRead;
        filesystem_line_count: FilesystemLineCount => NotYetRead;
        filesystem_read_failure: FilesystemReadFailure => NotYetRead;
        flow_block: FlowBlock => NotYetRead;
        flow_edge: FlowEdge => NotYetRead;
        flow_member: FlowMember => NotYetRead;
        flow_step: FlowStep => NotYetRead;
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
        mention: Mention => NoSuchConstruct;
        module_identity: ModuleIdentity => NotYetRead;
        numeric_literal: NumericLiteral => NoSuchConstruct;
        package_dependency: PackageDependency => NotYetRead;
        pattern_child: PatternChild => NotYetRead;
        pattern_parse: PatternParse => NotYetRead;
        pattern_syntax: PatternSyntax => NotYetRead;
        project_namespace_membership: ProjectNamespaceMembership => NotYetRead;
        prose: Prose => NotADocument;
        qualified_name: QualifiedName => NoSuchConstruct;
        raw_format_field: RawFormatField => NoSuchConstruct;
        raw_format_segment: RawFormatSegment => NoSuchConstruct;
        sequence_element: SequenceElement => NoSuchConstruct;
        source_dependency: SourceDependency => NotYetRead;
        standard_library_import: StandardLibraryImport => NoSuchConstruct;
        token_link: TokenLink => NoSuchConstruct;
        visibility: Visibility => NoSuchConstruct;
        workspace_member: WorkspaceMember => NotYetRead;
    }

    /// Go renders no expression: it has none.
    fn render(_file: &mut GoFile<'_>, _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }

    /// Go's names for the run: every kind at its code and every field at its code, as the walk writes them into the node table.
    fn vocabulary() -> WalkVocabulary {
        WalkVocabulary {
            kinds: WalkKinds(kinds().map(kind_name).collect()),
            fields: FieldNames(fields().map(field_name).collect()),
            fixed: FixedNames(Vec::new()),
        }
    }
}
