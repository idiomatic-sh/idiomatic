//! The fillers of the `walk` contract: for every relation of ryo's catalog the engine fills, how Markdown fills it from a walked file
//! (its `fills!` item), or that it is not in the language, with the cause; and the names Markdown states for the whole run: its kinds
//! and its fields. Markdown's fillers write no fixed name.

use crate::domain::file_view::types::{MarkdownFile, MarkdownWalk};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{field_name, fields, kind_bits, kind_name, kind_of, kinds};
use crate::domain::syntax::types::Kind;
use ryo::languages::{
    self, AbsenceCause, CatalogRelation, FieldNames, FileWords, Fill, Fillers, FixedNames,
    FileCompleteness, RenderOutcome, RenderRequest, Rows, WalkKinds, WalkVocabulary, Wanted, fillers, node_table,
};

/// The filler of each relation whose `fills!` item writes it whole alone: the item's run. Asked for a key, it writes the whole
/// relation and says so; `fillers::filled` is the one place that answer is written.
macro_rules! written {
    ($( $module:ident: $relation:ident => $item:path; )*) => {
        $(
            fn $module(
                file: &mut MarkdownFile<'_>,
                words: &mut FileWords,
                wanted: Wanted,
                rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                fillers::filled::<$item>(file, words, wanted, rows)
            }
        )*
    };
}

/// The filler of each relation Markdown does not fill, with the cause.
macro_rules! absent {
    ($( $module:ident: $relation:ident => $cause:ident; )*) => {
        $(
            fn $module(
                _file: &mut MarkdownFile<'_>,
                _words: &mut FileWords,
                _wanted: Wanted,
                _rows: &mut dyn Rows<<languages::$relation as CatalogRelation>::Row>,
            ) -> Fill {
                Fill::Absent(AbsenceCause::$cause)
            }
        )*
    };
}

/// The relations the language does not fill, with the cause: the one list, handed to the macro that makes the fillers of them and to
/// the one that makes the static answer.
macro_rules! each_absent {
    ($callback:ident $(, $asked:ident)?) => {
        $callback! {
            $( $asked; )?
            sequence_element: SequenceElement => NoSuchConstruct;
            mapping: Mapping => NoSuchConstruct;
            entry: Entry => NoSuchConstruct;
            analyzed_source: AnalyzedSource => NotYetRead;
            annotates: Annotates => NoSuchConstruct;
            assigns: Assigns => NoSuchConstruct;
            attribute_path: AttributePath => NoSuchConstruct;
            binding_event: BindingEvent => NotYetRead;
            binding_read: BindingRead => NotYetRead;
            binding_resolution: BindingResolution => NotYetRead;
            binding_shadow: BindingShadow => NotYetRead;
            binding_use: BindingUse => NotYetRead;
            brace_format_field: BraceFormatField => NotYetRead;
            brace_format_parse: BraceFormatParse => NotYetRead;
            bytes_literal: BytesLiteral => NotYetRead;
            call_argument: CallArgument => NoSuchConstruct;
            compiler_feature: CompilerFeature => NotYetRead;
            constant_key: ConstantKey => NotYetRead;
            decorates: Decorates => NoSuchConstruct;
            directive: Directive => NoSuchConstruct;
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
            function: Function => NoSuchConstruct;
            function_terminal: FunctionTerminal => NotYetRead;
            identifier: Identifier => NotYetRead;
            implicit_symbol: ImplicitSymbol => NotYetRead;
            indent_region: IndentRegion => NoSuchConstruct;
            inherits: Inherits => NoSuchConstruct;
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
            parameter: Parameter => NoSuchConstruct;
            pattern_child: PatternChild => NotYetRead;
            pattern_parse: PatternParse => NotYetRead;
            pattern_syntax: PatternSyntax => NotYetRead;
            project_namespace_membership: ProjectNamespaceMembership => NotYetRead;
            qualified_name: QualifiedName => NoSuchConstruct;
            raw_format_field: RawFormatField => NotYetRead;
            raw_format_segment: RawFormatSegment => NotYetRead;
            resolves: Resolves => NoSuchConstruct;
            source_dependency: SourceDependency => NotYetRead;
            standard_library_import: StandardLibraryImport => NotYetRead;
            static_call: StaticCall => NoSuchConstruct;
            static_import: StaticImport => NoSuchConstruct;
            token_link: TokenLink => NotYetRead;
            visibility: Visibility => NoSuchConstruct;
            workspace_member: WorkspaceMember => NotYetRead;
        }
    };
}

/// The static answer of the absent relations: the cause, when `relation` is one of them.
macro_rules! absent_arms {
    ($asked:ident; $( $module:ident: $relation:ident => $cause:ident; )*) => {
        match $asked {
            $( languages::Relation::$relation => Some(AbsenceCause::$cause), )*
            _ => None,
        }
    };
}

impl<P: Send + Sync + 'static> Fillers for MarkdownWalk<P> {
    type File<'f>
        = MarkdownFile<'f>
    where
        Self: 'f;

    written! {
        defines: Defines => crate::domain::relations::collections::relations::defines::item::DefinesItem;
        prose: Prose => crate::domain::relations::collections::relations::prose::item::ProseItem;
        reference: Reference => crate::domain::relations::collections::relations::reference::item::ReferenceItem;
        scope: Scope => crate::domain::relations::collections::relations::scope::item::ScopeItem;
        comment: Comment => crate::domain::relations::collections::relations::comment::item::CommentItem;
        source_location: SourceLocation => crate::domain::relations::collections::relations::source_location::item::SourceLocationItem;
        string_literal: StringLiteral => crate::domain::relations::collections::relations::string_literal::item::StringLiteralItem;
        syntax_error: SyntaxError => crate::domain::relations::collections::relations::syntax_error::item::SyntaxErrorItem;
    }

    each_absent!(absent);

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

    /// Whether the parser read the file whole: its syntax when it accepted the file, its lexical stream when no text is left it could
    /// not read.
    fn completeness(file: &MarkdownFile<'_>) -> FileCompleteness {
        let table = file.syntax.table;
        let whole = |held: bool| {
            if held {
                languages::SyntaxCompleteness::Complete
            } else {
                languages::SyntaxCompleteness::Incomplete
            }
        };
        FileCompleteness {
            syntax: whole(node_table::acceptance_of(table) == languages::SyntaxAcceptance::Accepted),
            lexical: whole(
                node_table::ids_of_kind(table, kind_bits(Kind::Extra(Extra::Error))).is_empty(),
            ),
        }
    }

    /// No relation of a file the parser accepted answers in part: a partial answer comes only from something the parser or the decoder
    /// reported, and every report rejects the file (held by the routes row's corpus test).
    fn fills_are_complete(file: &mut MarkdownFile<'_>) -> bool {
        Self::completeness(file).syntax == languages::SyntaxCompleteness::Complete
    }

    /// The language reads no project notes from a file.
    fn notes(
        _file: &mut MarkdownFile<'_>,
        _table: &languages::NodeTable,
        _source: languages::ByteSlice<'_>,
        _out: &mut languages::Bytes,
    ) {
    }

    fn spells(kind: languages::KindBits) -> bool {
        matches!(
            kind_of(kind),
            Kind::Token(_)
                | Kind::Extra(_)
                | Kind::Node(Node::Text | Node::InlineCode | Node::Definition | Node::Break | Node::ThematicBreak | Node::FootnoteReference)
        )
    }

    /// Markdown renders no expression: it has none.
    fn render(_file: &mut MarkdownFile<'_>, _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }

    /// Markdown's names for the run: every kind at its code and every field at its code, as the walk writes them into the node table.
    fn vocabulary() -> WalkVocabulary {
        WalkVocabulary {
            kinds: WalkKinds(kinds().map(kind_name).collect()),
            fields: FieldNames(fields().map(field_name).collect()),
            fixed: FixedNames(Vec::new()),
        }
    }
}
