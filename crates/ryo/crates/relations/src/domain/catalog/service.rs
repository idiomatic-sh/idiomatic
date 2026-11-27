//! The `catalog` service: the list of every relation of the catalog, a macro every crate that handles each relation calls, so a
//! relation added to the list is answered everywhere or does not compile; and the lookups of what each relation's item declares.
//! The list is the one declaration of the catalog: its relations, their modules and whether a language promises each one as an optional
//! capability (`optional`) or always answers it (`baseline`). The capability enum, its map and the module list are generated from it.
//! The macro that declares one relation from its columns (`relation_row`) is beside the catalog's types.

/// Hands the catalog to `$callback`, which is a macro that takes the three groups, each relation as its variant, module, row and column.
/// `@declared` hands the same list with each relation's capability word after its column, to `$callback`, followed by `$forward` and a
/// `;` when one is given.
#[macro_export]
macro_rules! each_relation {
    (@declared $callback:path $( , $forward:path )?) => {
        $callback! {
            $( $forward ; )?
            syntax {
                Function function FunctionRow FunctionColumn baseline;
                Annotates annotates AnnotatesRow AnnotatesColumn baseline;
                Assigns assigns AssignsRow AssignsColumn baseline;
                Comment comment CommentRow CommentColumn baseline;
                Directive directive DirectiveRow DirectiveColumn baseline;
                Decorates decorates DecoratesRow DecoratesColumn baseline;
                Defines defines DefinesRow DefinesColumn baseline;
                Visibility visibility VisibilityRow VisibilityColumn baseline;
                Inherits inherits InheritsRow InheritsColumn baseline;
                Parameter parameter ParameterRow ParameterColumn baseline;
                Reference reference ReferenceRow ReferenceColumn baseline;
                Mention mention MentionRow MentionColumn baseline;
                Scope scope ScopeRow ScopeColumn baseline;
                StaticCall static_call StaticCallRow StaticCallColumn baseline;
                CallArgument call_argument CallArgumentRow CallArgumentColumn baseline;
                Resolves resolves ResolvesRow ResolvesColumn baseline;
                BindingEvent binding_event BindingEventRow BindingEventColumn optional;
                BindingRead binding_read BindingReadRow BindingReadColumn optional;
                BindingUse binding_use BindingUseRow BindingUseColumn optional;
                BindingResolution binding_resolution BindingResolutionRow BindingResolutionColumn optional;
                LookupScope lookup_scope LookupScopeRow LookupScopeColumn optional;
                BindingShadow binding_shadow BindingShadowRow BindingShadowColumn optional;
                ImplicitSymbol implicit_symbol ImplicitSymbolRow ImplicitSymbolColumn optional;
                QualifiedName qualified_name QualifiedNameRow QualifiedNameColumn optional;
                Identifier identifier IdentifierRow IdentifierColumn optional;
                ExpressionShape expression_shape ExpressionShapeRow ExpressionShapeColumn optional;
                LiteralProperties literal_properties LiteralPropertiesRow LiteralPropertiesColumn optional;
                FunctionTerminal function_terminal FunctionTerminalRow FunctionTerminalColumn optional;
                ExpressionType expression_type ExpressionTypeRow ExpressionTypeColumn optional;
                InterpolatedStringLiteral interpolated_string_literal InterpolatedStringLiteralRow InterpolatedStringLiteralColumn optional;
                LanguageTarget language_target LanguageTargetRow LanguageTargetColumn optional;
                CompilerFeature compiler_feature CompilerFeatureRow CompilerFeatureColumn optional;
                AnnotationEvaluation annotation_evaluation AnnotationEvaluationRow AnnotationEvaluationColumn optional;
                ConstantKey constant_key ConstantKeyRow ConstantKeyColumn optional;
                FormatParse format_parse FormatParseRow FormatParseColumn optional;
                FormatField format_field FormatFieldRow FormatFieldColumn optional;
                BraceFormatParse brace_format_parse BraceFormatParseRow BraceFormatParseColumn optional;
                BraceFormatField brace_format_field BraceFormatFieldRow BraceFormatFieldColumn optional;
                RawFormatSegment raw_format_segment RawFormatSegmentRow RawFormatSegmentColumn optional;
                RawFormatField raw_format_field RawFormatFieldRow RawFormatFieldColumn optional;
                LiteralExpressionParse literal_expression_parse LiteralExpressionParseRow LiteralExpressionParseColumn optional;
                StaticImport static_import StaticImportRow StaticImportColumn baseline;
                StandardLibraryImport standard_library_import StandardLibraryImportRow StandardLibraryImportColumn optional;
                StringLiteral string_literal StringLiteralRow StringLiteralColumn baseline;
                BytesLiteral bytes_literal BytesLiteralRow BytesLiteralColumn optional;
                FormatSpecification format_specification FormatSpecificationRow FormatSpecificationColumn optional;
                SyntaxChild syntax_child SyntaxChildRow SyntaxChildColumn baseline;
                SyntaxError syntax_error SyntaxErrorRow SyntaxErrorColumn baseline;
                SyntaxTree syntax_tree SyntaxTreeRow SyntaxTreeColumn baseline;
                SemanticSyntax semantic_syntax SemanticSyntaxRow SemanticSyntaxColumn baseline;
                SemanticChild semantic_child SemanticChildRow SemanticChildColumn baseline;
                EmbeddedExpression embedded_expression EmbeddedExpressionRow EmbeddedExpressionColumn optional;
                SourceLocation source_location SourceLocationRow SourceLocationColumn baseline;
                IndentRegion indent_region IndentRegionRow IndentRegionColumn baseline;
                Mapping mapping MappingRow MappingColumn baseline;
                Entry entry EntryRow EntryColumn baseline;
                SequenceElement sequence_element SequenceElementRow SequenceElementColumn baseline;
                Prose prose ProseRow ProseColumn baseline;
                NumericLiteral numeric_literal NumericLiteralRow NumericLiteralColumn optional;
                TokenLink token_link TokenLinkRow TokenLinkColumn optional;
                PatternParse pattern_parse PatternParseRow PatternParseColumn optional;
                PatternSyntax pattern_syntax PatternSyntaxRow PatternSyntaxColumn optional;
                PatternChild pattern_child PatternChildRow PatternChildColumn optional;
                FlowEdge flow_edge FlowEdgeRow FlowEdgeColumn optional;
                FlowBlock flow_block FlowBlockRow FlowBlockColumn optional;
                FlowMember flow_member FlowMemberRow FlowMemberColumn optional;
                FlowStep flow_step FlowStepRow FlowStepColumn optional;
                AttributePath attribute_path AttributePathRow AttributePathColumn optional;
            }
            manifest {
                PackageDependency package_dependency PackageDependencyRow PackageDependencyColumn baseline;
                WorkspaceMember workspace_member WorkspaceMemberRow WorkspaceMemberColumn baseline;
                SourceDependency source_dependency SourceDependencyRow SourceDependencyColumn baseline;
                AnalyzedSource analyzed_source AnalyzedSourceRow AnalyzedSourceColumn baseline;
                FilePackage file_package FilePackageRow FilePackageColumn baseline;
                ModuleIdentity module_identity ModuleIdentityRow ModuleIdentityColumn optional;
                ProjectNamespaceMembership project_namespace_membership ProjectNamespaceMembershipRow ProjectNamespaceMembershipColumn optional;
                ManifestDiagnostic manifest_diagnostic ManifestDiagnosticRow ManifestDiagnosticColumn baseline;
            }
            engine {
                FilesystemFile filesystem_file FilesystemFileRow FilesystemFileColumn baseline;
                FilesystemLineCount filesystem_line_count FilesystemLineCountRow FilesystemLineCountColumn baseline;
                FilesystemExecutable filesystem_executable FilesystemExecutableRow FilesystemExecutableColumn baseline;
                FilesystemReadFailure filesystem_read_failure FilesystemReadFailureRow FilesystemReadFailureColumn baseline;
            }
        }
    };
    ($callback:path) => {
        $crate::each_relation! { @declared $crate::relation_names, $callback }
    };
}

/// Hands the catalog to `$callback` without the capability words: the form `each_relation` gives.
#[macro_export]
#[doc(hidden)]
macro_rules! relation_names {
    (
        $callback:path ;
        $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident $capability:ident ; )* } )*
    ) => {
        $callback! {
            $( $group { $( $variant $module $row $column ; )* } )*
        }
    };
}

/// Hands the catalog to `$callback` in the form `each_relation` gives, keeping only the relations it declares `optional`: the ones a
/// language answers through its `Semantics`, or the project, and the capability enum is named after.
#[macro_export]
#[doc(hidden)]
macro_rules! optional_relations {
    ($callback:path ; $( $groups:tt )*) => {
        $crate::kept_relations! { optional $callback [] $( $groups )* }
    };
}

/// Hands the catalog to `$callback` in the form `each_relation` gives, keeping only the relations it declares `baseline`: the ones every
/// language answers.
#[macro_export]
#[doc(hidden)]
macro_rules! baseline_relations {
    ($callback:path ; $( $groups:tt )*) => {
        $crate::kept_relations! { baseline $callback [] $( $groups )* }
    };
}

/// Keeps, group by group and in the catalog's order, the relations whose capability word is the first word (`optional` or `baseline`),
/// and hands the groups to `$callback` without the words.
#[macro_export]
#[doc(hidden)]
macro_rules! kept_relations {
    (@rows optional $callback:path [ $( $done:tt )* ] $group:ident [ $( $kept:tt )* ]
        [ $variant:ident $module:ident $row:ident $column:ident optional ; $( $rest:tt )* ] $( $groups:tt )*) => {
        $crate::kept_relations! { @rows optional $callback [ $( $done )* ] $group [ $( $kept )* $variant $module $row $column ; ] [ $( $rest )* ] $( $groups )* }
    };
    (@rows baseline $callback:path [ $( $done:tt )* ] $group:ident [ $( $kept:tt )* ]
        [ $variant:ident $module:ident $row:ident $column:ident baseline ; $( $rest:tt )* ] $( $groups:tt )*) => {
        $crate::kept_relations! { @rows baseline $callback [ $( $done )* ] $group [ $( $kept )* $variant $module $row $column ; ] [ $( $rest )* ] $( $groups )* }
    };
    (@rows $word:ident $callback:path [ $( $done:tt )* ] $group:ident [ $( $kept:tt )* ]
        [ $variant:ident $module:ident $row:ident $column:ident $other:ident ; $( $rest:tt )* ] $( $groups:tt )*) => {
        $crate::kept_relations! { @rows $word $callback [ $( $done )* ] $group [ $( $kept )* ] [ $( $rest )* ] $( $groups )* }
    };
    (@rows $word:ident $callback:path [ $( $done:tt )* ] $group:ident [ $( $kept:tt )* ] [] $( $groups:tt )*) => {
        $crate::kept_relations! { $word $callback [ $( $done )* $group { $( $kept )* } ] $( $groups )* }
    };
    ($word:ident $callback:path [ $( $done:tt )* ] $group:ident { $( $rows:tt )* } $( $groups:tt )*) => {
        $crate::kept_relations! { @rows $word $callback [ $( $done )* ] $group [] [ $( $rows )* ] $( $groups )* }
    };
    ($word:ident $callback:path [ $( $done:tt )* ]) => {
        $callback! { $( $done )* }
    };
}

/// The lookups of what each relation's item declares beyond its columns, generated from the catalog's list.
macro_rules! declared_lookups {
    ( $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )* ) => {
        /// Which files a relation's rows are about, as its item declares it.
        ///
        /// # Examples
        ///
        /// ```
        /// use ryo_relations::domain::catalog::service::scope_of;
        /// use ryo_relations::domain::catalog::types::RowScope;
        /// use ryo_relations::domain::relations::types::Relation;
        ///
        /// assert_eq!(scope_of(Relation::ModuleIdentity), RowScope::Project);
        /// ```
        #[must_use]
        pub fn scope_of(
            relation: crate::domain::relations::types::Relation,
        ) -> crate::domain::catalog::types::RowScope {
            match relation {
                $( $( crate::domain::relations::types::Relation::$variant => <crate::domain::relations::collections::relations::$module::item::$variant as crate::domain::relations::collections::relations::interface::RelationsItem>::SCOPE, )* )*
            }
        }

        /// How the engine reaches a relation's rows, as its item declares it: read in place from the node table, or filled by the
        /// language once per file and indexed by its key.
        ///
        /// # Examples
        ///
        /// ```
        /// use ryo_relations::domain::catalog::service::fact_form;
        /// use ryo_relations::domain::relations::types::{FactForm, FilledKey};
        /// use ryo_relations::domain::relations::collections::relations::function::item::FunctionColumn;
        /// use ryo_relations::domain::relations::types::{Column, Relation};
        ///
        /// assert_eq!(fact_form(Relation::SyntaxTree), FactForm::NodeTable);
        /// assert_eq!(fact_form(Relation::Function), FactForm::Filled(FilledKey::Unique(Column::Function(FunctionColumn::Id))));
        /// ```
        #[must_use]
        pub fn fact_form(
            relation: crate::domain::relations::types::Relation,
        ) -> crate::domain::relations::types::FactForm {
            match relation {
                $( $( crate::domain::relations::types::Relation::$variant => <crate::domain::relations::collections::relations::$module::item::$variant as crate::domain::relations::collections::relations::interface::RelationsItem>::FORM, )* )*
            }
        }


        /// Which ids of the node table a `NodeTable` relation has rows for, as its item declares it (`extent`): the file's own
        /// tree, or the embedded segments too. Nothing for a filled relation.
        ///
        /// # Examples
        ///
        /// ```
        /// use ryo_relations::domain::catalog::service::node_extent;
        /// use ryo_relations::domain::relations::types::{NodeExtent, Relation};
        ///
        /// assert_eq!(node_extent(Relation::SyntaxChild), Some(NodeExtent::Own));
        /// assert_eq!(node_extent(Relation::SemanticChild), Some(NodeExtent::Embedded));
        /// assert_eq!(node_extent(Relation::Function), None);
        /// ```
        #[must_use]
        pub fn node_extent(
            relation: crate::domain::relations::types::Relation,
        ) -> Option<crate::domain::relations::types::NodeExtent> {
            match relation {
                $( $( crate::domain::relations::types::Relation::$variant => <crate::domain::relations::collections::relations::$module::item::$variant as crate::domain::relations::collections::relations::interface::RelationsItem>::EXTENT, )* )*
            }
        }

        /// How a filled relation's rows grow with the file, as its item declares it (`size`). Nothing for a `NodeTable` relation,
        /// whose rows are the table's elements.
        ///
        /// # Examples
        ///
        /// ```
        /// use ryo_relations::domain::catalog::service::relation_size;
        /// use ryo_relations::domain::relations::types::{Relation, RelationSize};
        ///
        /// assert_eq!(relation_size(Relation::FilesystemFile), Some(RelationSize::PerFile));
        /// assert_eq!(relation_size(Relation::Function), Some(RelationSize::PerElement));
        /// assert_eq!(relation_size(Relation::SyntaxTree), None);
        /// ```
        #[must_use]
        pub fn relation_size(
            relation: crate::domain::relations::types::Relation,
        ) -> Option<crate::domain::relations::types::RelationSize> {
            match relation {
                $( $( crate::domain::relations::types::Relation::$variant => <crate::domain::relations::collections::relations::$module::item::$variant as crate::domain::relations::collections::relations::interface::RelationsItem>::SIZE, )* )*
            }
        }

        /// The columns of a relation that every language that fills it promises a fill by key for, as its item declares them
        /// (`keys`): a fill asked by one of them never writes the relation whole. None for a relation that declares none, and
        /// for a `NodeTable` relation. It is not the key the engine indexes a whole fill by (`fact_form`): each says what the
        /// other does not.
        ///
        /// # Examples
        ///
        /// ```
        /// use ryo_relations::domain::catalog::service::key_columns;
        /// use ryo_relations::domain::relations::collections::relations::token_link::item::TokenLinkColumn;
        /// use ryo_relations::domain::relations::types::{Column, Relation};
        ///
        /// let keys = key_columns(Relation::TokenLink);
        /// assert_eq!(keys, [Column::TokenLink(TokenLinkColumn::Source), Column::TokenLink(TokenLinkColumn::Target)]);
        /// assert!(key_columns(Relation::Comment).is_empty());
        /// ```
        #[must_use]
        pub fn key_columns(
            relation: crate::domain::relations::types::Relation,
        ) -> &'static [crate::domain::relations::types::Column] {
            match relation {
                $( $( crate::domain::relations::types::Relation::$variant => <crate::domain::relations::collections::relations::$module::item::$variant as crate::domain::relations::collections::relations::interface::RelationsItem>::KEYS, )* )*
            }
        }

        /// Whether a relation's rows are made from the notes the languages write of the files, as its item declares it.
        ///
        /// # Examples
        ///
        /// ```
        /// use ryo_relations::domain::catalog::service::needs_notes;
        /// use ryo_relations::domain::catalog::types::RelationNotes;
        /// use ryo_relations::domain::relations::types::Relation;
        ///
        /// assert_eq!(needs_notes(Relation::AnalyzedSource), RelationNotes::Needed);
        /// ```
        #[must_use]
        pub fn needs_notes(
            relation: crate::domain::relations::types::Relation,
        ) -> crate::domain::catalog::types::RelationNotes {
            match relation {
                $( $( crate::domain::relations::types::Relation::$variant => <crate::domain::relations::collections::relations::$module::item::$variant as crate::domain::relations::collections::relations::interface::RelationsItem>::NOTES, )* )*
            }
        }
    };
}

crate::each_relation!(declared_lookups);

/// The extent a read of a node relation has for a language: the catalog's extent says whether the relation MAY include what a
/// file embeds (`Embedded`) or is the file's own tree alone (`Own`), and the language's `includes_expansions` says whether it
/// DOES; the effective extent is the conjunction, so a relation the catalog states as `Embedded` is the file's own tree alone
/// for a language that does not cover its embedded content, and a relation the catalog states as `Own` stays `Own` whatever
/// the language says. Nothing for a relation that is no node relation. The one rule the engine's reads and the adapter's
/// projection both take.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::catalog::service::effective_extent;
/// use ryo_relations::domain::relations::types::{NodeExtent, Relation};
///
/// // Rust and Python cover what they embed in the semantic relations; TypeScript (its patterns) does not.
/// assert_eq!(effective_extent(Relation::SemanticSyntax, true), Some(NodeExtent::Embedded));
/// assert_eq!(effective_extent(Relation::SemanticChild, false), Some(NodeExtent::Own));
/// // The syntax relations are the own tree whatever the language says.
/// assert_eq!(effective_extent(Relation::SyntaxTree, true), Some(NodeExtent::Own));
/// assert_eq!(effective_extent(Relation::SyntaxChild, false), Some(NodeExtent::Own));
/// assert_eq!(effective_extent(Relation::Function, true), None);
/// ```
#[must_use]
pub fn effective_extent(
    relation: crate::domain::relations::types::Relation,
    includes_expansions: bool,
) -> Option<crate::domain::relations::types::NodeExtent> {
    use crate::domain::relations::types::NodeExtent;
    match node_extent(relation)? {
        NodeExtent::Embedded if includes_expansions => Some(NodeExtent::Embedded),
        NodeExtent::Embedded | NodeExtent::Own => Some(NodeExtent::Own),
    }
}

/// Whether a read of the node table through `relation` covers the file's own tree alone for a language that says
/// `includes_expansions`: the catalog's extent and the language's answer together (`effective_extent`), the one rule the
/// numbered adapter's projection takes too.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::catalog::service::covers_own_tree_for;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert!(covers_own_tree_for(Relation::SemanticSyntax, false));
/// assert!(!covers_own_tree_for(Relation::SemanticSyntax, true));
/// assert!(covers_own_tree_for(Relation::SyntaxTree, true));
/// ```
#[must_use]
pub fn covers_own_tree_for(
    relation: crate::domain::relations::types::Relation,
    includes_expansions: bool,
) -> bool {
    effective_extent(relation, includes_expansions)
        == Some(crate::domain::relations::types::NodeExtent::Own)
}
