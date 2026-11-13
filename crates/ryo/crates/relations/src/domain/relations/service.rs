//! The `relations` service: free functions named by verb over the catalog, with no IO and no
//! ports. The lookups that name every relation are generated from the catalog's one list
//! (`each_relation`, in the `catalog` concept), so a relation added to it is answered by every lookup or does not compile.

use super::constants::{DIAGNOSTIC_BASE, KIND};
use crate::domain::labels::types::{Folder, Holds, NameRef};
use crate::domain::relations::types::{
    Bits, CellKind, Class, Column, Count, FileId, Flag, Integer, NodeId, Offset, Position,
    Provenance, Relation, RelationDeclaring, Span, Value,
};
use crate::domain::terms::types::{ChildOrdinal, Symbol};

use crate::domain::relations::collections::relations::bytes_literal::item::BytesLiteralColumn;
use crate::domain::relations::collections::relations::call_argument::item::CallArgumentColumn;
use crate::domain::relations::collections::relations::comment::item::CommentColumn;
use crate::domain::relations::collections::relations::defines::item::DefinesColumn;
use crate::domain::relations::collections::relations::entry::item::EntryColumn;
use crate::domain::relations::collections::relations::function::item::FunctionColumn;
use crate::domain::relations::collections::relations::mapping::item::MappingColumn;
use crate::domain::relations::collections::relations::mention::item::MentionColumn;
use crate::domain::relations::collections::relations::package_dependency::item::PackageDependencyColumn;
use crate::domain::relations::collections::relations::parameter::item::ParameterColumn;
use crate::domain::relations::collections::relations::prose::item::ProseColumn;
use crate::domain::relations::collections::relations::reference::item::ReferenceColumn;
use crate::domain::relations::collections::relations::resolves::item::ResolvesColumn;
use crate::domain::relations::collections::relations::sequence_element::item::SequenceElementColumn;
use crate::domain::relations::collections::relations::source_location::item::SourceLocationColumn;
use crate::domain::relations::collections::relations::static_call::item::StaticCallColumn;
use crate::domain::relations::collections::relations::string_literal::item::StringLiteralColumn;
use crate::domain::relations::collections::relations::syntax_child::item::SyntaxChildColumn;
use crate::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
use crate::domain::relations::types::ColumnKeyness;
use crate::domain::terms::types::{Vocabulary, WordBlock};

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
        Class::Code | Class::Literal | Class::Lexical | Class::Document
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
pub fn location(span: Span) -> crate::domain::relations::types::Locatedness {
    if span == super::constants::NO_SPAN {
        crate::domain::relations::types::Locatedness::Absent
    } else {
        crate::domain::relations::types::Locatedness::Located
    }
}

/// The explicit optional promise of a relation; all other relations remain mandatory.
#[must_use]
pub fn optional_capability(
    relation: Relation,
) -> Option<crate::domain::capabilities::types::OptionalCapability> {
    super::constants::OPTIONAL_CAPABILITIES
        .iter()
        .copied()
        .find(|capability| capability_relation(*capability) == relation)
}

/// The catalog relation promised by an optional capability.
#[must_use]
pub const fn capability_relation(
    capability: crate::domain::capabilities::types::OptionalCapability,
) -> Relation {
    use crate::domain::capabilities::types::OptionalCapability;
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
        OptionalCapability::NumericLiteral => Relation::NumericLiteral,
        OptionalCapability::TokenLink => Relation::TokenLink,
        OptionalCapability::InterpolatedStringLiteral => Relation::InterpolatedStringLiteral,
        OptionalCapability::BytesLiteral => Relation::BytesLiteral,
        OptionalCapability::FormatSpecification => Relation::FormatSpecification,
        OptionalCapability::LanguageTarget => Relation::LanguageTarget,
        OptionalCapability::StandardLibraryImport => Relation::StandardLibraryImport,
        OptionalCapability::CompilerFeature => Relation::CompilerFeature,
        OptionalCapability::ConstantKey => Relation::ConstantKey,
        OptionalCapability::FormatParse => Relation::FormatParse,
        OptionalCapability::FormatField => Relation::FormatField,
        OptionalCapability::BraceFormatParse => Relation::BraceFormatParse,
        OptionalCapability::BraceFormatField => Relation::BraceFormatField,
        OptionalCapability::FunctionTerminal => Relation::FunctionTerminal,
        OptionalCapability::ExpressionType => Relation::ExpressionType,
        OptionalCapability::RawFormatSegment => Relation::RawFormatSegment,
        OptionalCapability::RawFormatField => Relation::RawFormatField,
        OptionalCapability::ModuleIdentity => Relation::ModuleIdentity,
        OptionalCapability::ProjectNamespaceMembership => Relation::ProjectNamespaceMembership,
        OptionalCapability::LiteralExpressionParse => Relation::LiteralExpressionParse,
    }
}

/// The words a column holds: one of a closed set for the columns that hold a kind the catalog shares across languages, any name or
/// number for the rest.
#[must_use]
pub fn vocabulary_of(column: Column) -> Vocabulary {
    if matches!(column, Column::Defines(DefinesColumn::Kind)) {
        Vocabulary::Closed(WordBlock::Declaration)
    } else if matches!(column, Column::Parameter(ParameterColumn::Kind)) {
        Vocabulary::Closed(WordBlock::Parameter)
    } else if matches!(
        column,
        Column::Entry(EntryColumn::Kind) | Column::SequenceElement(SequenceElementColumn::Kind)
    ) {
        Vocabulary::Closed(WordBlock::Value)
    } else if matches!(column, Column::Mapping(MappingColumn::Kind)) {
        Vocabulary::Closed(WordBlock::Mapping)
    } else if matches!(
        column,
        Column::PackageDependency(PackageDependencyColumn::Kind)
    ) {
        Vocabulary::Closed(WordBlock::Dependency)
    } else if matches!(
        column,
        Column::PackageDependency(PackageDependencyColumn::Origin)
    ) {
        Vocabulary::Closed(WordBlock::Origin)
    } else {
        Vocabulary::Open
    }
}

/// Whether a column says which row a row is: no two rows of the relation hold the same value in it,
/// so a projection that keeps it has no repeated rows to drop.
#[must_use]
pub fn is_key(column: Column) -> ColumnKeyness {
    if matches!(
        column,
        Column::SyntaxTree(SyntaxTreeColumn::Id)
            | Column::SourceLocation(SourceLocationColumn::Id)
            | Column::SyntaxChild(SyntaxChildColumn::Child)
            | Column::Function(FunctionColumn::Id)
            | Column::Comment(CommentColumn::Id)
            | Column::Prose(ProseColumn::Id)
            | Column::Reference(ReferenceColumn::Occurrence)
            | Column::Mention(MentionColumn::Occurrence)
            | Column::StaticCall(StaticCallColumn::Occurrence)
            | Column::CallArgument(CallArgumentColumn::Expression)
            | Column::Resolves(ResolvesColumn::Occurrence)
            | Column::StringLiteral(StringLiteralColumn::Occurrence)
            | Column::BytesLiteral(BytesLiteralColumn::Occurrence)
    ) {
        ColumnKeyness::Key
    } else {
        ColumnKeyness::Plain
    }
}
