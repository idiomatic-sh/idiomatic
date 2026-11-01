//! The `relations` service: free functions named by verb over the catalog, with no IO and no
//! ports. The lookups that name every relation are generated from the catalog's one list
//! (`each_relation`, below), so a relation added to it is answered by every lookup or does not compile.

use super::constants::{DIAGNOSTIC_BASE, KIND};
use super::types::{
    Bits, CellKind, Class, Column, Count, FileId, Flag, Folder, Holds, Integer, NameRef, NodeId,
    Offset, Position, Provenance, Relation, RelationDeclaring, Span, Value,
};
use crate::domain::terms::types::{ChildOrdinal, Symbol};

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
                InterpolatedStringLiteral interpolated_string_literal InterpolatedStringLiteralRow InterpolatedStringLiteralColumn;
                ConstantKey constant_key ConstantKeyRow ConstantKeyColumn;
                StaticImport static_import StaticImportRow StaticImportColumn;
                StringLiteral string_literal StringLiteralRow StringLiteralColumn;
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
            }
            manifest {
                PackageDependency package_dependency PackageDependencyRow PackageDependencyColumn;
                WorkspaceMember workspace_member WorkspaceMemberRow WorkspaceMemberColumn;
                SourceDependency source_dependency SourceDependencyRow SourceDependencyColumn;
                AnalyzedSource analyzed_source AnalyzedSourceRow AnalyzedSourceColumn;
                FilePackage file_package FilePackageRow FilePackageColumn;
            }
            engine {
                FilesystemFile filesystem_file FilesystemFileRow FilesystemFileColumn;
                FilesystemLineCount filesystem_line_count FilesystemLineCountRow FilesystemLineCountColumn;
            }
        }
    };
}

/// What the lookups over the catalog are made of, generated from the list in `each_relation`.
macro_rules! catalog_lookups {
    (
        syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        manifest { $( $manifest_variant:ident $manifest_module:ident $manifest_row:ident $manifest_column:ident ; )* }
        engine { $( $engine_variant:ident $engine_module:ident $engine_row:ident $engine_column:ident ; )* }
    ) => {
        /// Every relation, in the order the catalog lists them: the ones a language reads from a tree,
        /// the ones it reads from the manifests, and the ones the engine gives.
        pub const ALL: &[Relation] = &[
            $( Relation::$variant, )*
            $( Relation::$manifest_variant, )*
            $( Relation::$engine_variant, )*
        ];

        /// A relation's name in RyoQL and in listings: text that leaves the engine.
        #[must_use]
        pub fn name(relation: Relation) -> NameRef<'static> {
            match relation {
                $( Relation::$variant => NameRef($crate::domain::relations::collections::relations::$module::constants::name()), )*
                $( Relation::$manifest_variant => NameRef($crate::domain::relations::collections::relations::$manifest_module::constants::name()), )*
                $( Relation::$engine_variant => NameRef($crate::domain::relations::collections::relations::$engine_module::constants::name()), )*
            }
        }

        /// What a relation holds, in a sentence.
        #[must_use]
        pub fn summary(relation: Relation) -> Holds {
            match relation {
                $( Relation::$variant => Holds($crate::domain::relations::collections::relations::$module::constants::summary()), )*
                $( Relation::$manifest_variant => Holds($crate::domain::relations::collections::relations::$manifest_module::constants::summary()), )*
                $( Relation::$engine_variant => Holds($crate::domain::relations::collections::relations::$engine_module::constants::summary()), )*
            }
        }

        /// A relation's columns, in order.
        #[must_use]
        pub fn columns(relation: Relation) -> Vec<Column> {
            match relation {
                $( Relation::$variant => <$crate::domain::relations::collections::relations::$module::item::$variant as $crate::domain::relations::collections::relations::interface::RelationsItem>::columns().into_iter().map(Column::$variant).collect(), )*
                $( Relation::$manifest_variant => <$crate::domain::relations::collections::relations::$manifest_module::item::$manifest_variant as $crate::domain::relations::collections::relations::interface::RelationsItem>::columns().into_iter().map(Column::$manifest_variant).collect(), )*
                $( Relation::$engine_variant => <$crate::domain::relations::collections::relations::$engine_module::item::$engine_variant as $crate::domain::relations::collections::relations::interface::RelationsItem>::columns().into_iter().map(Column::$engine_variant).collect(), )*
            }
        }

        /// A column's name in RyoQL and in listings: text that leaves the engine.
        #[must_use]
        pub fn column_name(column: Column) -> NameRef<'static> {
            match column {
                $( Column::$variant(column) => NameRef($crate::domain::relations::collections::relations::$module::constants::column_name(column)), )*
                $( Column::$manifest_variant(column) => NameRef($crate::domain::relations::collections::relations::$manifest_module::constants::column_name(column)), )*
                $( Column::$engine_variant(column) => NameRef($crate::domain::relations::collections::relations::$engine_module::constants::column_name(column)), )*
            }
        }

        /// What a column holds.
        #[must_use]
        pub fn kind(column: Column) -> CellKind {
            match column {
                $( Column::$variant(column) => <$crate::domain::relations::collections::relations::$module::item::$variant as $crate::domain::relations::collections::relations::interface::RelationsItem>::kind(column), )*
                $( Column::$manifest_variant(column) => <$crate::domain::relations::collections::relations::$manifest_module::item::$manifest_variant as $crate::domain::relations::collections::relations::interface::RelationsItem>::kind(column), )*
                $( Column::$engine_variant(column) => <$crate::domain::relations::collections::relations::$engine_module::item::$engine_variant as $crate::domain::relations::collections::relations::interface::RelationsItem>::kind(column), )*
            }
        }

        /// What each column of `relation` holds, in column order.
        #[must_use]
        pub fn kinds(relation: Relation) -> &'static [CellKind] {
            match relation {
                $( Relation::$variant => <$crate::domain::relations::collections::relations::$module::item::$variant as $crate::domain::relations::collections::relations::interface::RelationsItem>::kinds(), )*
                $( Relation::$manifest_variant => <$crate::domain::relations::collections::relations::$manifest_module::item::$manifest_variant as $crate::domain::relations::collections::relations::interface::RelationsItem>::kinds(), )*
                $( Relation::$engine_variant => <$crate::domain::relations::collections::relations::$engine_module::item::$engine_variant as $crate::domain::relations::collections::relations::interface::RelationsItem>::kinds(), )*
            }
        }

        /// The name of a relation as a file system names it: the folder of its item, of its fixtures.
        #[must_use]
        pub fn folder_of(relation: Relation) -> Folder {
            match relation {
                $( Relation::$variant => Folder(stringify!($module)), )*
                $( Relation::$manifest_variant => Folder(stringify!($manifest_module)), )*
                $( Relation::$engine_variant => Folder(stringify!($engine_module)), )*
            }
        }

        /// What a relation is about, as its item says it.
        #[must_use]
        pub fn class_of(relation: Relation) -> Class {
            match relation {
                $( Relation::$variant => <$crate::domain::relations::collections::relations::$module::item::$variant as $crate::domain::relations::collections::relations::interface::RelationsItem>::CLASS, )*
                $( Relation::$manifest_variant => <$crate::domain::relations::collections::relations::$manifest_module::item::$manifest_variant as $crate::domain::relations::collections::relations::interface::RelationsItem>::CLASS, )*
                $( Relation::$engine_variant => <$crate::domain::relations::collections::relations::$engine_module::item::$engine_variant as $crate::domain::relations::collections::relations::interface::RelationsItem>::CLASS, )*
            }
        }

        /// The relation a column belongs to.
        #[must_use]
        pub fn relation_of(column: Column) -> Relation {
            match column {
                $( Column::$variant(_) => Relation::$variant, )*
                $( Column::$manifest_variant(_) => Relation::$manifest_variant, )*
                $( Column::$engine_variant(_) => Relation::$engine_variant, )*
            }
        }

        /// Where a relation's rows come from: the files' trees, the manifests of the project and the links between its files, or the engine.
        #[must_use]
        pub const fn provenance_of(relation: Relation) -> Provenance {
            match relation {
                $( Relation::$variant => Provenance::Trees, )*
                $( Relation::$manifest_variant => Provenance::Manifests, )*
                $( Relation::$engine_variant => Provenance::Engine, )*
            }
        }
    };
}

crate::each_relation!(catalog_lookups);

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

/// Every relation, in the order the catalog lists them.
#[must_use]
pub fn all() -> &'static [Relation] {
    ALL
}

/// Whether the relation says something of what a file declares, which an empty file has nothing of: code, and the text written in a file.
#[must_use]
pub fn is_declaring(relation: Relation) -> RelationDeclaring {
    if matches!(
        class_of(relation),
        Class::Code | Class::Literal | Class::Document
    ) {
        RelationDeclaring::Declares
    } else {
        RelationDeclaring::Silent
    }
}

/// The relation RyoQL text calls `text`: text that enters the engine and becomes a variant once.
#[must_use]
pub fn relation_named(text: NameRef<'_>) -> Option<Relation> {
    all()
        .iter()
        .copied()
        .find(|relation| name(*relation) == text)
}

/// The column of a relation that says what kind of thing a row is, when it has one.
#[must_use]
pub fn kind_column(relation: Relation) -> Option<Column> {
    column_named(relation, NameRef(KIND))
}

/// The column of `relation` that RyoQL text calls `text`.
#[must_use]
pub fn column_named(relation: Relation, text: NameRef<'_>) -> Option<Column> {
    columns(relation)
        .into_iter()
        .find(|column| column_name(*column) == text)
}

/// The node at `position` of the file `file`: the file in the high half, the place in the file's
/// table in the low half. Every relation names a node this way, so a join of two relations on a node is a join on the same element.
#[must_use]
pub fn node_id(file: FileId, position: Position) -> NodeId {
    NodeId((u64::from(file.0) << 32) | u64::from(position.0))
}

/// The file a node is in: the high half of its id.
#[must_use]
pub fn node_file(node: NodeId) -> FileId {
    FileId(u32::try_from(node.0 >> 32).unwrap_or(u32::MAX))
}

/// Where a node is in its file's table: the low half of its id.
#[must_use]
pub fn node_position(node: NodeId) -> Position {
    Position(u32::try_from(node.0 & 0xffff_ffff).unwrap_or(u32::MAX))
}

/// The id of the report numbered `number` that the parser made about the file `file`: in a space of its own, so it is never the id of
/// a node.
#[must_use]
pub fn diagnostic_id(file: FileId, number: ChildOrdinal) -> NodeId {
    node_id(file, Position(DIAGNOSTIC_BASE + number.0))
}

/// How many kinds of node and token a walk can tell apart, whatever the language.
pub const KIND_LIMIT: usize = 1024;

/// The number of a relation among the catalog's, which is its place in a set of relations.
#[expect(
    clippy::as_conversions,
    reason = "a relation is a field-less enum: its number is the cast"
)]
#[must_use]
pub const fn number_of(relation: Relation) -> Count {
    Count(relation as usize)
}

/// The number a value is stored as.
#[must_use]
pub fn to_bits(value: Value) -> Bits {
    Bits(match value {
        Value::Node(node) => node.0,
        Value::Symbol(symbol) => u64::from(symbol.0),
        Value::Name(name) => name.hash.0,
        Value::File(file) => u64::from(file.0),
        Value::Span(span) => (u64::from(span.start.0) << 32) | u64::from(span.end.0),
        Value::Number(number) => number.0.cast_unsigned(),
        Value::Flag(flag) => u64::from(flag == Flag::Raised),
    })
}

/// The value a number stands for in a column of `kind`.
#[must_use]
pub fn from_bits(kind: CellKind, bits: Bits) -> Value {
    let low = u32::try_from(bits.0 & 0xffff_ffff).unwrap_or(u32::MAX);
    let whole = u32::try_from(bits.0).unwrap_or(u32::MAX);
    match kind {
        CellKind::Node => Value::Node(NodeId(bits.0)),
        CellKind::Symbol => Value::Symbol(Symbol(whole)),
        CellKind::File => Value::File(FileId(whole)),
        CellKind::Span => Value::Span(Span {
            start: Offset(u32::try_from(bits.0 >> 32).unwrap_or(u32::MAX)),
            end: Offset(low),
        }),
        CellKind::Number => Value::Number(Integer(bits.0.cast_signed())),
        CellKind::Flag => Value::Flag(if bits.0 == 0 {
            Flag::Lowered
        } else {
            Flag::Raised
        }),
    }
}

/// Whether a location denotes source bytes, rather than explicit absence.
#[must_use]
pub fn location(span: Span) -> super::types::Locatedness {
    if span == super::constants::NO_SPAN {
        super::types::Locatedness::Absent
    } else {
        super::types::Locatedness::Located
    }
}

/// The explicit optional promise of a relation; all other relations remain mandatory.
#[must_use]
pub fn optional_capability(relation: Relation) -> Option<super::types::OptionalCapability> {
    super::constants::OPTIONAL_CAPABILITIES
        .iter()
        .copied()
        .find(|capability| capability_relation(*capability) == relation)
}

/// The catalog relation promised by an optional capability.
#[must_use]
pub const fn capability_relation(capability: super::types::OptionalCapability) -> Relation {
    use super::types::OptionalCapability;
    match capability {
        OptionalCapability::Identifier => Relation::Identifier,
        OptionalCapability::QualifiedName => Relation::QualifiedName,
        OptionalCapability::EmbeddedExpression => Relation::EmbeddedExpression,
        OptionalCapability::ExpressionShape => Relation::ExpressionShape,
        OptionalCapability::BindingEvent => Relation::BindingEvent,
        OptionalCapability::BindingRead => Relation::BindingRead,
        OptionalCapability::BindingUse => Relation::BindingUse,
        OptionalCapability::BindingShadow => Relation::BindingShadow,
        OptionalCapability::ImplicitSymbol => Relation::ImplicitSymbol,
        OptionalCapability::LiteralProperties => Relation::LiteralProperties,
        OptionalCapability::InterpolatedStringLiteral => Relation::InterpolatedStringLiteral,
        OptionalCapability::ConstantKey => Relation::ConstantKey,
    }
}
