//! The fillers of the `walk` contract: for every relation of ryo's catalog the engine fills, how CSS fills it from a walked file
//! (its `fills!` item), or that it is not in the language, with the cause; and the names CSS states for the whole run: its kinds
//! and its fields. CSS's fillers write no fixed name.

use crate::domain::file_view::types::{CssFile, CssWalk};
use crate::domain::syntax::service::{field_name, fields, kind_name, kinds};
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
                file: &mut CssFile<'_>,
                words: &mut FileWords,
                wanted: Wanted,
                rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                fillers::filled::<$item>(file, words, wanted, rows)
            }
        )*
    };
}

/// The filler of each relation CSS does not fill, with the cause.
macro_rules! absent {
    ($( $module:ident: $relation:ident => $cause:ident; )*) => {
        $(
            fn $module(
                _file: &mut CssFile<'_>,
                _words: &mut FileWords,
                _wanted: Wanted,
                _rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                Fill::Absent(AbsenceCause::$cause)
            }
        )*
    };
}

impl<P: Send + Sync + 'static> Fillers for CssWalk<P> {
    type File<'f>
        = CssFile<'f>
    where
        Self: 'f;

    written! {
        comment: Comment => crate::domain::relations::collections::relations::comment::item::CommentItem;
        reference: Reference => crate::domain::relations::collections::relations::reference::item::ReferenceItem;
        source_location: SourceLocation => crate::domain::relations::collections::relations::source_location::item::SourceLocationItem;
        static_import: StaticImport => crate::domain::relations::collections::relations::static_import::item::StaticImportItem;
        string_literal: StringLiteral => crate::domain::relations::collections::relations::string_literal::item::StringLiteralItem;
        syntax_error: SyntaxError => crate::domain::relations::collections::relations::syntax_error::item::SyntaxErrorItem;
    }

    absent! {
        analyzed_source: AnalyzedSource => NotYetRead;
        annotates: Annotates => NoSuchConstruct;
        assigns: Assigns => NoSuchConstruct;
        attribute_path: AttributePath => NoSuchConstruct;
        binding_event: BindingEvent => NoSuchConstruct;
        binding_read: BindingRead => NoSuchConstruct;
        binding_resolution: BindingResolution => NoSuchConstruct;
        binding_shadow: BindingShadow => NoSuchConstruct;
        binding_use: BindingUse => NoSuchConstruct;
        brace_format_field: BraceFormatField => NoSuchConstruct;
        brace_format_parse: BraceFormatParse => NoSuchConstruct;
        bytes_literal: BytesLiteral => NoSuchConstruct;
        call_argument: CallArgument => NoSuchConstruct;
        compiler_feature: CompilerFeature => NoSuchConstruct;
        constant_key: ConstantKey => NoSuchConstruct;
        decorates: Decorates => NoSuchConstruct;
        defines: Defines => NoSuchConstruct;
        directive: Directive => NoSuchConstruct;
        embedded_expression: EmbeddedExpression => NoSuchConstruct;
        entry: Entry => NotAKeyedDocument;
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
        function: Function => NoSuchConstruct;
        function_terminal: FunctionTerminal => NoSuchConstruct;
        identifier: Identifier => NoSuchConstruct;
        implicit_symbol: ImplicitSymbol => NoSuchConstruct;
        indent_region: IndentRegion => NoSuchConstruct;
        inherits: Inherits => NoSuchConstruct;
        interpolated_string_literal: InterpolatedStringLiteral => NoSuchConstruct;
        language_target: LanguageTarget => NoSuchConstruct;
        literal_expression_parse: LiteralExpressionParse => NoSuchConstruct;
        literal_properties: LiteralProperties => NoSuchConstruct;
        lookup_scope: LookupScope => NoSuchConstruct;
        manifest_diagnostic: ManifestDiagnostic => NotYetRead;
        mapping: Mapping => NotAKeyedDocument;
        mention: Mention => NoSuchConstruct;
        module_identity: ModuleIdentity => NotYetRead;
        numeric_literal: NumericLiteral => NoSuchConstruct;
        package_dependency: PackageDependency => NotYetRead;
        parameter: Parameter => NoSuchConstruct;
        pattern_child: PatternChild => NotYetRead;
        pattern_parse: PatternParse => NotYetRead;
        pattern_syntax: PatternSyntax => NotYetRead;
        project_namespace_membership: ProjectNamespaceMembership => NotYetRead;
        prose: Prose => NotADocument;
        qualified_name: QualifiedName => NoSuchConstruct;
        raw_format_field: RawFormatField => NoSuchConstruct;
        raw_format_segment: RawFormatSegment => NoSuchConstruct;
        resolves: Resolves => NoSuchConstruct;
        scope: Scope => NoSuchConstruct;
        sequence_element: SequenceElement => NotAKeyedDocument;
        source_dependency: SourceDependency => NotYetRead;
        standard_library_import: StandardLibraryImport => NoSuchConstruct;
        static_call: StaticCall => NoSuchConstruct;
        token_link: TokenLink => NoSuchConstruct;
        visibility: Visibility => NoSuchConstruct;
        workspace_member: WorkspaceMember => NotYetRead;
    }

    /// CSS renders no expression: it has none.
    fn render(_file: &mut CssFile<'_>, _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }

    /// CSS's names for the run: every kind at its code and every field at its code, as the walk writes them into the node table.
    fn vocabulary() -> WalkVocabulary {
        WalkVocabulary {
            kinds: WalkKinds(kinds().map(kind_name).collect()),
            fields: FieldNames(fields().map(field_name).collect()),
            fixed: FixedNames(Vec::new()),
        }
    }
}
