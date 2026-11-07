//! The `catalog` service: the list of every relation of the catalog and the macro that declares one relation from its columns.
//! Both are macros every crate that handles each relation calls, so a relation added to the list is answered everywhere or does not compile.

/// Hands the catalog to `$callback`, which is a macro that takes the three groups.
#[macro_export]
macro_rules! each_relation {
    ($callback:path) => {
        $callback! {
            syntax {
                Function function FunctionRow FunctionColumn;
                Annotates annotates AnnotatesRow AnnotatesColumn;
                Assigns assigns AssignsRow AssignsColumn;
                Comment comment CommentRow CommentColumn;
                Directive directive DirectiveRow DirectiveColumn;
                Decorates decorates DecoratesRow DecoratesColumn;
                Defines defines DefinesRow DefinesColumn;
                Visibility visibility VisibilityRow VisibilityColumn;
                Inherits inherits InheritsRow InheritsColumn;
                Parameter parameter ParameterRow ParameterColumn;
                Reference reference ReferenceRow ReferenceColumn;
                Mention mention MentionRow MentionColumn;
                Scope scope ScopeRow ScopeColumn;
                StaticCall static_call StaticCallRow StaticCallColumn;
                CallArgument call_argument CallArgumentRow CallArgumentColumn;
                Resolves resolves ResolvesRow ResolvesColumn;
                BindingEvent binding_event BindingEventRow BindingEventColumn;
                BindingRead binding_read BindingReadRow BindingReadColumn;
                BindingUse binding_use BindingUseRow BindingUseColumn;
                BindingShadow binding_shadow BindingShadowRow BindingShadowColumn;
                ImplicitSymbol implicit_symbol ImplicitSymbolRow ImplicitSymbolColumn;
                QualifiedName qualified_name QualifiedNameRow QualifiedNameColumn;
                Identifier identifier IdentifierRow IdentifierColumn;
                ExpressionShape expression_shape ExpressionShapeRow ExpressionShapeColumn;
                LiteralProperties literal_properties LiteralPropertiesRow LiteralPropertiesColumn;
                FunctionTerminal function_terminal FunctionTerminalRow FunctionTerminalColumn;
                ExpressionType expression_type ExpressionTypeRow ExpressionTypeColumn;
                InterpolatedStringLiteral interpolated_string_literal InterpolatedStringLiteralRow InterpolatedStringLiteralColumn;
                LanguageTarget language_target LanguageTargetRow LanguageTargetColumn;
                CompilerFeature compiler_feature CompilerFeatureRow CompilerFeatureColumn;
                ConstantKey constant_key ConstantKeyRow ConstantKeyColumn;
                FormatParse format_parse FormatParseRow FormatParseColumn;
                FormatField format_field FormatFieldRow FormatFieldColumn;
                BraceFormatParse brace_format_parse BraceFormatParseRow BraceFormatParseColumn;
                BraceFormatField brace_format_field BraceFormatFieldRow BraceFormatFieldColumn;
                RawFormatSegment raw_format_segment RawFormatSegmentRow RawFormatSegmentColumn;
                RawFormatField raw_format_field RawFormatFieldRow RawFormatFieldColumn;
                LiteralExpressionParse literal_expression_parse LiteralExpressionParseRow LiteralExpressionParseColumn;
                StaticImport static_import StaticImportRow StaticImportColumn;
                StandardLibraryImport standard_library_import StandardLibraryImportRow StandardLibraryImportColumn;
                StringLiteral string_literal StringLiteralRow StringLiteralColumn;
                BytesLiteral bytes_literal BytesLiteralRow BytesLiteralColumn;
                FormatSpecification format_specification FormatSpecificationRow FormatSpecificationColumn;
                SyntaxChild syntax_child SyntaxChildRow SyntaxChildColumn;
                SyntaxError syntax_error SyntaxErrorRow SyntaxErrorColumn;
                SyntaxTree syntax_tree SyntaxTreeRow SyntaxTreeColumn;
                SemanticSyntax semantic_syntax SemanticSyntaxRow SemanticSyntaxColumn;
                SemanticChild semantic_child SemanticChildRow SemanticChildColumn;
                EmbeddedExpression embedded_expression EmbeddedExpressionRow EmbeddedExpressionColumn;
                SourceLocation source_location SourceLocationRow SourceLocationColumn;
                IndentRegion indent_region IndentRegionRow IndentRegionColumn;
                Mapping mapping MappingRow MappingColumn;
                Entry entry EntryRow EntryColumn;
                SequenceElement sequence_element SequenceElementRow SequenceElementColumn;
                Prose prose ProseRow ProseColumn;
                NumericLiteral numeric_literal NumericLiteralRow NumericLiteralColumn;
                TokenLink token_link TokenLinkRow TokenLinkColumn;
            }
            manifest {
                PackageDependency package_dependency PackageDependencyRow PackageDependencyColumn;
                WorkspaceMember workspace_member WorkspaceMemberRow WorkspaceMemberColumn;
                SourceDependency source_dependency SourceDependencyRow SourceDependencyColumn;
                AnalyzedSource analyzed_source AnalyzedSourceRow AnalyzedSourceColumn;
                FilePackage file_package FilePackageRow FilePackageColumn;
                ModuleIdentity module_identity ModuleIdentityRow ModuleIdentityColumn;
                ProjectNamespaceMembership project_namespace_membership ProjectNamespaceMembershipRow ProjectNamespaceMembershipColumn;
                ManifestDiagnostic manifest_diagnostic ManifestDiagnosticRow ManifestDiagnosticColumn;
            }
            engine {
                FilesystemFile filesystem_file FilesystemFileRow FilesystemFileColumn;
                FilesystemLineCount filesystem_line_count FilesystemLineCountRow FilesystemLineCountColumn;
                FilesystemExecutable filesystem_executable FilesystemExecutableRow FilesystemExecutableColumn;
                FilesystemReadFailure filesystem_read_failure FilesystemReadFailureRow FilesystemReadFailureColumn;
            }
        }
    };
}

/// Declares one relation of the catalog from its columns, which its module names once, in the macro it calls (the fixtures' typed rows
/// are made from the same columns): its row, its columns and the numbers a row is stored as. A column is its name, the name of its
/// variant, the type a row holds, what the store holds for it (a `CellKind` and the `Value` of the same name) and, when the type is
/// not that already, the function that turns it into that.
#[macro_export]
#[doc(hidden)]
macro_rules! relation_row {
    (
        item $item:ident, row { $( #[$row_doc:meta] )* $row:ident }, column { $( #[$column_doc:meta] )* $column:ident },
        expected { $( #[$expected_doc:meta] )* $expected:ident }, class $class:ident,
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
