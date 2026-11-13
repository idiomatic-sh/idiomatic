//! The `catalog` service: the list of every relation of the catalog and the macro that declares one relation from its columns.
//! Both are macros every crate that handles each relation calls, so a relation added to the list is answered everywhere or does not compile.
//! The list is the one declaration of the catalog: its relations, their modules and whether a language promises each one as an optional
//! capability (`optional`) or always answers it (`baseline`). The capability enum, its map and the module list are generated from it.

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

/// Declares one relation of the catalog from its columns, which its module names once, in the macro it calls (the fixtures' typed rows
/// are made from the same columns): its row, its columns and the numbers a row is stored as, with its class, its scope (`File` or
/// `Project`) and whether it needs the languages' notes (`Needed` or `Unneeded`). A column is its name, the name of its
/// variant, the type a row holds, what the store holds for it (a `CellKind` and the `Value` of the same name) and, when the type is
/// not that already, the function that turns it into that.
#[macro_export]
#[doc(hidden)]
macro_rules! relation_row {
    (
        item $item:ident, row { $( #[$row_doc:meta] )* $row:ident }, column { $( #[$column_doc:meta] )* $column:ident },
        expected { $( #[$expected_doc:meta] )* $expected:ident }, class $class:ident, scope $scope:ident, notes $notes:ident,
        columns { $( $( #[$doc:meta] )* $field:ident $variant:ident : $ty:ty => $kind:ident $( ( $convert:path ) )? ; )* }
    ) => {
        $( #[$row_doc] )*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $row {
            $( $( #[$doc] )* pub $field: $ty, )*
        }

        $( #[$column_doc] )*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $column {
            $(
                $( #[$doc] )*
                $variant,
            )*
        }

        impl RelationsItem for $item {
            type Row = $row;
            type Column = $column;
            type Words = [$crate::domain::relations::types::Bits; [$( $crate::domain::relations::types::CellKind::$kind ),*].len()];
            const CLASS: $crate::domain::relations::types::Class = $crate::domain::relations::types::Class::$class;
            const SCOPE: $crate::domain::catalog::types::RowScope = $crate::domain::catalog::types::RowScope::$scope;
            const NOTES: $crate::domain::catalog::types::RelationNotes = $crate::domain::catalog::types::RelationNotes::$notes;

            fn relation() -> $crate::domain::relations::types::Relation {
                $crate::domain::relations::types::Relation::$item
            }

            fn kinds() -> &'static [$crate::domain::relations::types::CellKind] {
                &[ $( $crate::domain::relations::types::CellKind::$kind ),* ]
            }

            fn columns() -> Vec<$column> {
                vec![ $( $column::$variant ),* ]
            }

            fn kind(column: $column) -> $crate::domain::relations::types::CellKind {
                match column {
                    $( $column::$variant => $crate::domain::relations::types::CellKind::$kind, )*
                }
            }

            fn words(row: $row) -> [$crate::domain::relations::types::Bits; [$( $crate::domain::relations::types::CellKind::$kind ),*].len()] {
                [ $( $crate::relation_row!(@word (row.$field) $kind $( $convert )?) ),* ]
            }
        }
    };
    (@word ($value:expr) $kind:ident) => {
        $crate::domain::relations::service::to_bits($crate::domain::relations::types::Value::$kind($value))
    };
    (@word ($value:expr) $kind:ident $convert:path) => {
        $crate::domain::relations::service::to_bits($crate::domain::relations::types::Value::$kind($convert($value)))
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
