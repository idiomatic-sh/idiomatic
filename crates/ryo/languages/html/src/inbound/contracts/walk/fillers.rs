//! The fillers of the `walk` contract: for every relation of ryo's catalog the engine fills, how html fills it from a walked file.
//! The relations are answered one group at a time as their fillers are written; each one not yet written says it is not read yet, and
//! the names the language states for the whole run are its kinds and its fields.

use crate::domain::file_view::types::{HtmlFile, HtmlWalk};
use crate::domain::syntax::service::{field_name, fields, kind_name, kinds};
use ryo::languages::{
    self, AbsenceCause, CatalogRelation, FieldNames, FileWords, Fill, Fillers, FixedNames, NameText, RenderOutcome, RenderRequest,
    Rows, WalkKinds, WalkVocabulary, Wanted, fillers,
};

/// The filler of each relation whose `fills!` item writes it whole alone: the item's run.
macro_rules! written {
    ($( $module:ident: $relation:ident => $item:path; )*) => {
        $(
            fn $module(
                file: &mut HtmlFile<'_>,
                words: &mut FileWords,
                wanted: Wanted,
                rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                fillers::filled::<$item>(file, words, wanted, rows)
            }
        )*
    };
}

/// The filler of each relation the language does not fill, with the cause.
macro_rules! absent {
    ($( $module:ident: $relation:ident => $cause:ident; )*) => {
        $(
            fn $module(
                _file: &mut HtmlFile<'_>,
                _words: &mut FileWords,
                _wanted: Wanted,
                _rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                Fill::Absent(AbsenceCause::$cause)
            }
        )*
    };
}

impl<P: Send + Sync + 'static> Fillers for HtmlWalk<P> {
    type File<'f>
        = HtmlFile<'f>
    where
        Self: 'f;

    written! {
        comment: Comment => crate::domain::relations::collections::relations::comment::item::CommentItem;
        defines: Defines => crate::domain::relations::collections::relations::defines::item::DefinesItem;
        prose: Prose => crate::domain::relations::collections::relations::prose::item::ProseItem;
        reference: Reference => crate::domain::relations::collections::relations::reference::item::ReferenceItem;
        scope: Scope => crate::domain::relations::collections::relations::scope::item::ScopeItem;
        source_location: SourceLocation => crate::domain::relations::collections::relations::source_location::item::SourceLocationItem;
        string_literal: StringLiteral => crate::domain::relations::collections::relations::string_literal::item::StringLiteralItem;
        syntax_error: SyntaxError => crate::domain::relations::collections::relations::syntax_error::item::SyntaxErrorItem;
    }

    absent! {
        binding_event: BindingEvent => NotYetRead;
        binding_resolution: BindingResolution => NotYetRead;
        binding_use: BindingUse => NotYetRead;
        brace_format_field: BraceFormatField => NotYetRead;
        brace_format_parse: BraceFormatParse => NotYetRead;
        bytes_literal: BytesLiteral => NotYetRead;
        call_argument: CallArgument => NotYetRead;
        compiler_feature: CompilerFeature => NotYetRead;
        decorates: Decorates => NotYetRead;
        embedded_expression: EmbeddedExpression => NotYetRead;
        format_field: FormatField => NotYetRead;
        format_parse: FormatParse => NotYetRead;
        format_specification: FormatSpecification => NotYetRead;
        function_terminal: FunctionTerminal => NotYetRead;
        identifier: Identifier => NotYetRead;
        implicit_symbol: ImplicitSymbol => NotYetRead;
        indent_region: IndentRegion => NotYetRead;
        interpolated_string_literal: InterpolatedStringLiteral => NotYetRead;
        language_target: LanguageTarget => NotYetRead;
        literal_expression_parse: LiteralExpressionParse => NotYetRead;
        literal_properties: LiteralProperties => NotYetRead;
        numeric_literal: NumericLiteral => NotYetRead;
        parameter: Parameter => NotYetRead;
        qualified_name: QualifiedName => NotYetRead;
        raw_format_field: RawFormatField => NotYetRead;
        standard_library_import: StandardLibraryImport => NotYetRead;
        static_import: StaticImport => NotYetRead;
        binding_read: BindingRead => NotYetRead;
        binding_shadow: BindingShadow => NotYetRead;
        constant_key: ConstantKey => NotYetRead;
        expression_shape: ExpressionShape => NotYetRead;
        expression_type: ExpressionType => NotYetRead;
        function: Function => NotYetRead;
        inherits: Inherits => NotYetRead;
        lookup_scope: LookupScope => NotYetRead;
        raw_format_segment: RawFormatSegment => NotYetRead;
        resolves: Resolves => NotYetRead;
        token_link: TokenLink => NotYetRead;
        analyzed_source: AnalyzedSource => NotYetRead;
        annotates: Annotates => NotYetRead;
        assigns: Assigns => NotYetRead;
        attribute_path: AttributePath => NotYetRead;
        directive: Directive => NotYetRead;
        entry: Entry => NotYetRead;
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
        mapping: Mapping => NotYetRead;
        mention: Mention => NotYetRead;
        module_identity: ModuleIdentity => NotYetRead;
        package_dependency: PackageDependency => NotYetRead;
        pattern_child: PatternChild => NotYetRead;
        pattern_parse: PatternParse => NotYetRead;
        pattern_syntax: PatternSyntax => NotYetRead;
        project_namespace_membership: ProjectNamespaceMembership => NotYetRead;
        sequence_element: SequenceElement => NotYetRead;
        source_dependency: SourceDependency => NotYetRead;
        static_call: StaticCall => NotYetRead;
        visibility: Visibility => NotYetRead;
        workspace_member: WorkspaceMember => NotYetRead;
    }

    fn render(_file: &mut HtmlFile<'_>, _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }

    fn vocabulary() -> WalkVocabulary {
        WalkVocabulary {
            kinds: WalkKinds(kinds().map(kind_name).collect()),
            fields: FieldNames(fields().map(field_name).collect()),
            fixed: FixedNames(Vec::<NameText<'static>>::new()),
        }
    }
}

#[cfg(test)]
#[path = "fillers_test.rs"]
mod tests;
