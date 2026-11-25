//! The fillers of the `walk` contract: for every relation of ryo's catalog the engine fills, how Go fills it from a walked file
//! (its `fills!` item), or that it is not in the language, with the cause; and the names Go states for the whole run: its kinds
//! and its fields. Go's fillers write no fixed name.

use crate::domain::file_view::types::{GoFile, GoWalk};
use crate::domain::roles::service::{field_name, fields};
use crate::domain::extras::types::Extra;
use crate::domain::syntax::service::{kind_name, kind_of, kinds};
use crate::domain::syntax::types::Kind;
use ryo::languages::{
    self, AbsenceCause, CatalogRelation, FieldNames, FileCompleteness, FileWords, Fill, Fillers, FixedNames,
    RenderOutcome, RenderRequest, Rows, SyntaxCompleteness, WalkKinds, WalkVocabulary, Wanted, fillers, node_table,
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

/// The relations Go does not fill, with the cause: the one list, handed to the macro that makes the fillers of them and to the one that
/// makes the static answer.
macro_rules! each_absent {
    ($callback:ident) => {
        $callback! {
            analyzed_source: AnalyzedSource => NotYetRead;
            attribute_path: AttributePath => NotYetRead;
            binding_event: BindingEvent => NotYetRead;
            binding_read: BindingRead => NotYetRead;
            binding_resolution: BindingResolution => NotYetRead;
            binding_shadow: BindingShadow => NotYetRead;
            binding_use: BindingUse => NotYetRead;
            brace_format_field: BraceFormatField => NotYetRead;
            brace_format_parse: BraceFormatParse => NotYetRead;
            bytes_literal: BytesLiteral => NotYetRead;
            compiler_feature: CompilerFeature => NotYetRead;
            constant_key: ConstantKey => NotYetRead;
            decorates: Decorates => NoSuchConstruct;
            embedded_expression: EmbeddedExpression => NotYetRead;
            expression_shape: ExpressionShape => NotYetRead;
            expression_type: ExpressionType => NotYetRead;
            file_package: FilePackage => NotYetRead;
            filesystem_executable: FilesystemExecutable => NotYetRead;
            filesystem_file: FilesystemFile => NotYetRead;
            filesystem_line_count: FilesystemLineCount => NotYetRead;
            filesystem_read_failure: FilesystemReadFailure => NotYetRead;
            flow_block: FlowBlock => NotYetRead;
            flow_edge: FlowEdge => NotYetRead;
            flow_member: FlowMember => NotYetRead;
            flow_step: FlowStep => NotYetRead;
            format_field: FormatField => NotYetRead;
            format_parse: FormatParse => NotYetRead;
            format_specification: FormatSpecification => NotYetRead;
            function_terminal: FunctionTerminal => NotYetRead;
            identifier: Identifier => NotYetRead;
            implicit_symbol: ImplicitSymbol => NotYetRead;
            indent_region: IndentRegion => NoSuchConstruct;
            interpolated_string_literal: InterpolatedStringLiteral => NotYetRead;
            language_target: LanguageTarget => NotYetRead;
            literal_expression_parse: LiteralExpressionParse => NotYetRead;
            literal_properties: LiteralProperties => NotYetRead;
            lookup_scope: LookupScope => NotYetRead;
            manifest_diagnostic: ManifestDiagnostic => NotYetRead;
            mention: Mention => NoSuchConstruct;
            module_identity: ModuleIdentity => NotYetRead;
            numeric_literal: NumericLiteral => NotYetRead;
            package_dependency: PackageDependency => NotYetRead;
            pattern_child: PatternChild => NotYetRead;
            pattern_parse: PatternParse => NotYetRead;
            pattern_syntax: PatternSyntax => NotYetRead;
            project_namespace_membership: ProjectNamespaceMembership => NotYetRead;
            prose: Prose => NotADocument;
            qualified_name: QualifiedName => NotYetRead;
            raw_format_field: RawFormatField => NotYetRead;
            raw_format_segment: RawFormatSegment => NotYetRead;
            sequence_element: SequenceElement => NoSuchConstruct;
            source_dependency: SourceDependency => NotYetRead;
            standard_library_import: StandardLibraryImport => NotYetRead;
            token_link: TokenLink => NotYetRead;
            visibility: Visibility => NoSuchConstruct;
            workspace_member: WorkspaceMember => NotYetRead;
        }
    };
    ($callback:ident, $asked:ident) => {
        $callback! {
            $asked;
            analyzed_source: AnalyzedSource => NotYetRead;
            attribute_path: AttributePath => NotYetRead;
            binding_event: BindingEvent => NotYetRead;
            binding_read: BindingRead => NotYetRead;
            binding_resolution: BindingResolution => NotYetRead;
            binding_shadow: BindingShadow => NotYetRead;
            binding_use: BindingUse => NotYetRead;
            brace_format_field: BraceFormatField => NotYetRead;
            brace_format_parse: BraceFormatParse => NotYetRead;
            bytes_literal: BytesLiteral => NotYetRead;
            compiler_feature: CompilerFeature => NotYetRead;
            constant_key: ConstantKey => NotYetRead;
            decorates: Decorates => NoSuchConstruct;
            embedded_expression: EmbeddedExpression => NotYetRead;
            expression_shape: ExpressionShape => NotYetRead;
            expression_type: ExpressionType => NotYetRead;
            file_package: FilePackage => NotYetRead;
            filesystem_executable: FilesystemExecutable => NotYetRead;
            filesystem_file: FilesystemFile => NotYetRead;
            filesystem_line_count: FilesystemLineCount => NotYetRead;
            filesystem_read_failure: FilesystemReadFailure => NotYetRead;
            flow_block: FlowBlock => NotYetRead;
            flow_edge: FlowEdge => NotYetRead;
            flow_member: FlowMember => NotYetRead;
            flow_step: FlowStep => NotYetRead;
            format_field: FormatField => NotYetRead;
            format_parse: FormatParse => NotYetRead;
            format_specification: FormatSpecification => NotYetRead;
            function_terminal: FunctionTerminal => NotYetRead;
            identifier: Identifier => NotYetRead;
            implicit_symbol: ImplicitSymbol => NotYetRead;
            indent_region: IndentRegion => NoSuchConstruct;
            interpolated_string_literal: InterpolatedStringLiteral => NotYetRead;
            language_target: LanguageTarget => NotYetRead;
            literal_expression_parse: LiteralExpressionParse => NotYetRead;
            literal_properties: LiteralProperties => NotYetRead;
            lookup_scope: LookupScope => NotYetRead;
            manifest_diagnostic: ManifestDiagnostic => NotYetRead;
            mention: Mention => NoSuchConstruct;
            module_identity: ModuleIdentity => NotYetRead;
            numeric_literal: NumericLiteral => NotYetRead;
            package_dependency: PackageDependency => NotYetRead;
            pattern_child: PatternChild => NotYetRead;
            pattern_parse: PatternParse => NotYetRead;
            pattern_syntax: PatternSyntax => NotYetRead;
            project_namespace_membership: ProjectNamespaceMembership => NotYetRead;
            prose: Prose => NotADocument;
            qualified_name: QualifiedName => NotYetRead;
            raw_format_field: RawFormatField => NotYetRead;
            raw_format_segment: RawFormatSegment => NotYetRead;
            sequence_element: SequenceElement => NoSuchConstruct;
            source_dependency: SourceDependency => NotYetRead;
            standard_library_import: StandardLibraryImport => NotYetRead;
            token_link: TokenLink => NotYetRead;
            visibility: Visibility => NoSuchConstruct;
            workspace_member: WorkspaceMember => NotYetRead;
        }
    };
}

/// The static answer: the cause of each relation in the list, nothing for any other.
macro_rules! absent_arms {
    ($asked:ident; $( $module:ident: $relation:ident => $cause:ident; )*) => {
        match $asked {
            $( languages::Relation::$relation => Some(AbsenceCause::$cause), )*
            _ => None,
        }
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

    each_absent!(absent);


    fn completeness(file: &GoFile<'_>) -> FileCompleteness {
        let said = if node_table::syntax_error_count(file.syntax.table).0 == 0 {
            SyntaxCompleteness::Complete
        } else {
            SyntaxCompleteness::Incomplete
        };
        FileCompleteness { syntax: said, lexical: said }
    }

    /// The syntax was read whole: Go has no other part a relation of the file can answer in part.
    fn fills_are_complete(file: &mut GoFile<'_>) -> bool {
        <Self as Fillers>::completeness(file).syntax == SyntaxCompleteness::Complete
    }

    /// The packages a source imports; a module's `go.mod` imports none, and leaves no note.
    fn notes(file: &mut GoFile<'_>, _table: &languages::NodeTable, _source: languages::ByteSlice<'_>, out: &mut languages::Bytes) {
        use crate::outbound::notes::interface::NotesPort as _;
        if !file.module {
            crate::outbound::notes::factory::postcard().write(&crate::domain::walked::fact_notes::noted(&file.syntax), out);
        }
    }

    /// An element that is a leaf holds its text as its word.
    fn spells(kind: languages::KindBits) -> bool {
        matches!(
            kind_of(kind),
            Kind::Token(_)
                | Kind::Extra(
                    Extra::Whitespace | Extra::Error | Extra::ModWord | Extra::ModLParen | Extra::ModRParen | Extra::ModComment
                )
        )
    }

    fn absent(relation: languages::Relation) -> Option<AbsenceCause> {
        use languages::Relation as R;
        // The manifest and filesystem relations are the project contract's and the host's: the language does not say they are absent.
        if matches!(
            relation,
            R::AnalyzedSource | R::FilePackage | R::FilesystemExecutable | R::FilesystemFile | R::FilesystemLineCount
                | R::FilesystemReadFailure | R::ManifestDiagnostic | R::ModuleIdentity | R::PackageDependency
                | R::ProjectNamespaceMembership | R::SourceDependency | R::WorkspaceMember
        ) {
            return None;
        }
        each_absent!(absent_arms, relation)
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
