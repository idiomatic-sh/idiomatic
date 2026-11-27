//! The types of the `catalog` concept: what the declaration of a relation says of it beyond its columns: its scope, whether it needs
//! the languages' notes, and the optional capability a language promises it by; and `relation_row`, the macro that makes a
//! relation's row, its columns and what it declares from its item.

/// Which files a relation's rows are about: each file's own, or the project's as a whole.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::catalog::types::RowScope;
/// use ryo_relations::domain::catalog::service::scope_of;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_eq!(scope_of(Relation::Function), RowScope::File);
/// assert_eq!(scope_of(Relation::PackageDependency), RowScope::Project);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowScope {
    /// Its rows are read for one file at a time.
    File,
    /// Its rows are read once, for the project.
    Project,
}

/// Whether a relation's rows are made from the notes the languages write of the files, for the project to read.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::catalog::types::RelationNotes;
/// use ryo_relations::domain::catalog::service::needs_notes;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_eq!(needs_notes(Relation::SourceDependency), RelationNotes::Needed);
/// assert_eq!(needs_notes(Relation::Function), RelationNotes::Unneeded);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationNotes {
    /// The languages write their notes when a plan reads it.
    Needed,
    /// It needs no notes.
    Unneeded,
}

/// The capability enum, generated from the relations the catalog declares `optional`.
macro_rules! capability_enum {
    ( $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )* ) => {
        /// Opt-in semantic promises beyond the required language relation baseline: one for each relation the catalog declares
        /// `optional`, named as the relation is.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum OptionalCapability {
            $( $(
                /// The promise to answer the relation of the same name.
                $variant,
            )* )*
        }
    };
}

crate::each_relation!(@declared crate::optional_relations, capability_enum);

/// Declares one relation of the catalog from its columns, which its module names once, in the macro it calls (the fixtures' typed rows
/// are made from the same columns): its row, its columns and the numbers a row is stored as, with its class, its scope (`File` or
/// `Project`), whether it needs the languages' notes (`Needed` or `Unneeded`) and how the engine reaches its rows (`form`).
/// - `form NodeTable { extent Own; Column => NodeColumn, .. }`: read in place from the node table, with which ids it has rows for
///   (`extent`: `Own` for the file's own tree, `Embedded` for the embedded segments too) and the node-table column of each of its
///   columns.
/// - `form Filled(key), size S, keys [Column, ..],`: filled by the language, with the key the engine indexes filled rows by, how
///   its rows grow with the file (`size`: `PerFile`, `PerElement` or `PerPair`) and the columns every language that fills it
///   promises a fill by key for (`keys`, which may be empty).
///
/// `extent`, `size` and `keys` are required, as `form` is: an item without them matches no arm and does not compile. A column is
/// its name, the name of its variant, the type a row holds, what the store holds for it (a `CellKind` and the `Value` of the same
/// name) and, when the type is not that already, the function that turns it into that.
#[macro_export]
#[doc(hidden)]
macro_rules! relation_row {
    (
        item $item:ident, row $row:tt, column { $( #[$column_doc:meta] )* $column:ident }, expected $expected:tt,
        class $class:ident, scope $scope:ident, notes $notes:ident,
        form NodeTable { extent $extent:ident ; $( $node_column:ident => $node_source:ident ),* $(,)? },
        columns $columns:tt
    ) => {
        $crate::relation_row! {
            @declared
            item $item, row $row, column { $( #[$column_doc] )* $column }, class $class, scope $scope, notes $notes,
            form { $crate::domain::relations::types::FactForm::NodeTable },
            nodes [ $( ($column::$node_column, $crate::domain::relations::types::NodeColumn::$node_source) ),* ],
            extent { Some($crate::domain::relations::types::NodeExtent::$extent) },
            size { None },
            keys [],
            columns $columns
        }
    };
    (
        item $item:ident, row $row:tt, column { $( #[$column_doc:meta] )* $column:ident }, expected $expected:tt,
        class $class:ident, scope $scope:ident, notes $notes:ident,
        form Filled ( $( $key:tt )* ), size $size:ident, keys [ $( $key_column:ident ),* $(,)? ],
        columns $columns:tt
    ) => {
        $crate::relation_row! {
            @declared
            item $item, row $row, column { $( #[$column_doc] )* $column }, class $class, scope $scope, notes $notes,
            form { $crate::relation_row!(@form $item $column Filled ( $( $key )* )) },
            nodes [],
            extent { None },
            size { Some($crate::domain::relations::types::RelationSize::$size) },
            keys [ $( $crate::domain::relations::types::Column::$item($column::$key_column) ),* ],
            columns $columns
        }
    };
    (
        @declared
        item $item:ident, row { $( #[$row_doc:meta] )* $row:ident }, column { $( #[$column_doc:meta] )* $column:ident },
        class $class:ident, scope $scope:ident, notes $notes:ident,
        form { $( $form:tt )* }, nodes [ $( $nodes:tt )* ], extent { $( $extent:tt )* }, size { $( $size:tt )* },
        keys [ $( $keys:tt )* ],
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
            const FORM: $crate::domain::relations::types::FactForm = $( $form )*;
            const NODE_COLUMNS: &'static [($column, $crate::domain::relations::types::NodeColumn)] = &[ $( $nodes )* ];
            const EXTENT: Option<$crate::domain::relations::types::NodeExtent> = $( $extent )*;
            const SIZE: Option<$crate::domain::relations::types::RelationSize> = $( $size )*;
            const KEYS: &'static [$crate::domain::relations::types::Column] = &[ $( $keys )* ];

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
    (@form $item:ident $column:ident Filled (Whole)) => {
        $crate::domain::relations::types::FactForm::Filled($crate::domain::relations::types::FilledKey::Whole)
    };
    (@form $item:ident $column:ident Filled ($shape:ident ($key:ident))) => {
        $crate::domain::relations::types::FactForm::Filled($crate::domain::relations::types::FilledKey::$shape(
            $crate::domain::relations::types::Column::$item($column::$key),
        ))
    };
    (@word ($value:expr) $kind:ident) => {
        $crate::domain::relations::service::to_bits($crate::domain::relations::types::Value::$kind($value))
    };
    (@word ($value:expr) $kind:ident $convert:path) => {
        $crate::domain::relations::service::to_bits($crate::domain::relations::types::Value::$kind($convert($value)))
    };
}
