//! The fillers of the `walk` contract: for every relation of ryo's catalog the engine fills, how Python fills it from a walked
//! file (its `fills!` item, or the semantic model's filler), or that it is not in the language, with the cause.

use crate::domain::file_view::types::{PythonFile, PythonWalk};
use ryo::languages::{
    self, AbsenceCause, CatalogRelation, FileWords, Fill, Fillers, RenderOutcome, RenderRequest,
    Rows, fillers,
};

/// The filler of each relation a `fills!` item writes: the item's run.
macro_rules! written {
    ($( $module:ident: $relation:ident => $item:path; )*) => {
        $(
            fn $module(
                file: &mut PythonFile<'_>,
                words: &mut FileWords,
                rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                fillers::filled::<$item>(file, words, rows)
            }
        )*
    };
}

/// The filler of each relation the semantic model answers: the model's own filler.
macro_rules! modelled {
    ($( $module:ident: $relation:ident => $filler:path; )*) => {
        $(
            fn $module(
                file: &mut PythonFile<'_>,
                words: &mut FileWords,
                rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                $filler(file, words, rows)
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
        brace_format_field: BraceFormatField => crate::inbound::contracts::parse::readers::brace_format::composition::BraceFormatFieldItem;
        brace_format_parse: BraceFormatParse => crate::inbound::contracts::parse::readers::brace_format::composition::BraceFormatParseItem;
        bytes_literal: BytesLiteral => crate::domain::relations::collections::relations::bytes_literal::item::BytesLiteralItem;
        call_argument: CallArgument => crate::domain::relations::collections::relations::call_argument::item::CallArgumentItem;
        comment: Comment => crate::domain::relations::collections::relations::comment::item::CommentItem;
        constant_key: ConstantKey => crate::domain::relations::collections::relations::constant_key::item::ConstantKeyItem;
        compiler_feature: CompilerFeature => crate::domain::relations::collections::relations::compiler_feature::item::CompilerFeatureItem;
        decorates: Decorates => crate::domain::relations::collections::relations::decorates::item::DecoratesItem;
        embedded_expression: EmbeddedExpression => crate::domain::relations::collections::relations::embedded_expression::item::EmbeddedExpressionItem;
        expression_shape: ExpressionShape => crate::domain::relations::collections::relations::expression_shape::item::ExpressionShapeItem;
        expression_type: ExpressionType => crate::domain::relations::collections::relations::expression_type::item::ExpressionTypeItem;
        format_field: FormatField => crate::inbound::contracts::parse::readers::format::composition::FormatFieldItem;
        format_parse: FormatParse => crate::inbound::contracts::parse::readers::format::composition::FormatParseItem;
        format_specification: FormatSpecification => crate::inbound::contracts::parse::readers::format_specification::composition::FormatSpecificationItem;
        function: Function => crate::domain::relations::collections::relations::function::item::FunctionItem;
        function_terminal: FunctionTerminal => crate::domain::relations::collections::relations::function_terminal::item::FunctionTerminalItem;
        identifier: Identifier => crate::domain::relations::collections::relations::identifier::item::IdentifierItem;
        indent_region: IndentRegion => crate::domain::relations::collections::relations::indent_region::item::IndentRegionItem;
        inherits: Inherits => crate::domain::relations::collections::relations::inherits::item::InheritsItem;
        interpolated_string_literal: InterpolatedStringLiteral => crate::domain::relations::collections::relations::interpolated_string_literal::item::InterpolatedStringLiteralItem;
        language_target: LanguageTarget => crate::domain::relations::collections::relations::language_target::item::LanguageTargetItem;
        literal_expression_parse: LiteralExpressionParse => crate::inbound::contracts::parse::readers::literal_expression::composition::LiteralExpressionParseItem;
        literal_properties: LiteralProperties => crate::domain::relations::collections::relations::literal_properties::item::LiteralPropertiesItem;
        numeric_literal: NumericLiteral => crate::domain::relations::collections::relations::numeric_literal::item::NumericLiteralItem;
        parameter: Parameter => crate::domain::relations::collections::relations::parameter::item::ParameterItem;
        raw_format_field: RawFormatField => crate::inbound::contracts::parse::readers::raw_format::composition::RawFormatFieldItem;
        raw_format_segment: RawFormatSegment => crate::inbound::contracts::parse::readers::raw_format::composition::RawFormatSegmentItem;
        source_location: SourceLocation => super::source_location::SourceLocationItem;
        standard_library_import: StandardLibraryImport => crate::domain::relations::collections::relations::standard_library_import::item::StandardLibraryImportItem;
        static_import: StaticImport => crate::domain::relations::collections::relations::static_import::item::StaticImportItem;
        string_literal: StringLiteral => crate::domain::relations::collections::relations::string_literal::item::StringLiteralItem;
        syntax_error: SyntaxError => crate::domain::relations::collections::relations::syntax_error::item::SyntaxErrorItem;
        token_link: TokenLink => crate::inbound::contracts::parse::readers::token_link::composition::TokenLinkItem;
    }

    modelled! {
        binding_event: BindingEvent => crate::domain::binding_facts::service::binding_event_rows;
        binding_read: BindingRead => crate::domain::binding_facts::service::binding_read_rows;
        binding_resolution: BindingResolution => crate::domain::binding_facts::service::binding_resolution_rows;
        binding_shadow: BindingShadow => crate::domain::binding_facts::service::binding_shadow_rows;
        binding_use: BindingUse => crate::domain::binding_facts::service::binding_use_rows;
        defines: Defines => crate::domain::lexical_facts::service::defines_rows;
        implicit_symbol: ImplicitSymbol => crate::domain::lexical_facts::service::implicit_symbol_rows;
        qualified_name: QualifiedName => crate::domain::lexical_facts::service::qualified_name_rows;
        reference: Reference => crate::domain::lexical_facts::service::reference_rows;
        resolves: Resolves => crate::domain::lexical_facts::service::resolves_rows;
        scope: Scope => crate::domain::lexical_facts::service::scope_rows;
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

    fn render(_file: &mut PythonFile<'_>, _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }
}
