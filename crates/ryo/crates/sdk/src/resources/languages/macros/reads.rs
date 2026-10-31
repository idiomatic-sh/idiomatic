// ignore: idiomatic.arch/idiomatic-rust/no-string-literals - `env!` and the `reason` of an `allow` take a literal, which a constant cannot stand in for
/// Declares how a language reads a relation, as data. It is the only way an item implements `ryo::languages::ReadItem` or
/// `ryo::languages::ManifestItem`: both are sealed against an accidental hand-written impl: an impl that forgets the marker in `languages::declared` does not compile. A deliberate marker impl compiles, and the rule `readers-in-items` flags it.
///
/// A reader of the trees of a language names the relation, the names its code uses for the file's tree, the element and the
/// file's names, and what it looks at:
///
/// ```text
/// ryo::reads! {
///     Item => languages::Relation |tree, at, symbols| {
///         kinds: [NodeKind::Fn, NodeKind::Impl] as kind_bits,   // or: kinds: every,
///         <statements>
///     }
/// }
/// ```
///
/// A reader that would otherwise search the file again for each element it is given (the header an entry belongs to, the entries of a
/// table) reads the model the language worked out of the file once, when it parsed it (`Parse::FileModel`, made by `file_model`):
/// `Item => Relation, model(FileModel) |tree, model, at, symbols| { .. }` gives it `model` as a reference to that value. A reader without
/// `model(..)` reads the tree alone, and is for a language whose `FileModel` is `()`.
///
/// A reader of the manifests of a project, or of the links between its files, names the type the language derives when it reads the
/// manifests (`()` when it derives nothing), then the model it reads its rows from, which a function answers (nothing when the
/// project has none), and writes its rows from the model alone:
///
/// ```text
/// ryo::reads! {
///     Item => languages::Relation, manifests(Derived) |input, symbols| model packages: Packages = (packages_in(input.resolved)) {
///         <statements>
///     }
/// }
/// ```
///
/// | statement | what it says |
/// |---|---|
/// | `require <condition>;` | the element is skipped unless the condition holds |
/// | `given <name> = <value>;` | a value the rows need; one that ends in `?` and is `None` skips the element |
/// | `row { <column>: <form>, .. }` | writes a row of the relation; the row is the relation's own, so a column it does not have, or one left out, does not compile |
/// | `partial <cause> => <missing>;` | the relation is read only in part for this element |
/// | `when <function>(<arguments>) { <pattern> => { <statements> } .. }` | one branch for each shape the function answers |
/// | `each (<list>) as <item> { <statements> }` | the statements for each item of the list; `as (<ordinal>, <item>)` also names its place in the list from zero |
///
/// A column is given by one of:
///
/// | form | the value |
/// |---|---|
/// | `node(<position>)` | the id of the element at the position |
/// | `file` | the file's number |
/// | `span(<position>)` | where the element at the position starts and ends |
/// | `text(<position>)` | the text of the token at the position, as the name it was interned as |
/// | `value(<expression>)` | any value; the expression sees the names the declaration gave and the `given` values before it |
///
/// The functions a statement calls are the language's model: they answer values and never write a row.
#[macro_export]
macro_rules! reads {
    (
        $item:ident => $relation:ty, model ($model_type:ty)
        |$tree:ident, $model:ident, $at:ident, $symbols:ident|
        { kinds: every, expanded, $($body:tt)* }
    ) => {
        $crate::reads!(@syntax $item $relation [$tree $at $symbols] [$model : $model_type] $crate::languages::Expansions::Included,
            { $crate::languages::Reads::Every } $($body)*);
    };
    (
        $item:ident => $relation:ty
        |$tree:ident, $at:ident, $symbols:ident|
        { kinds: [ $( $kind:expr ),+ $(,)? ] as $bits:path, expanded, $($body:tt)* }
    ) => {
        $crate::reads!(@syntax $item $relation [$tree $at $symbols] [_model : ()] $crate::languages::Expansions::Included,
            { $crate::languages::Reads::Kinds(&[ $( $bits($kind) ),+ ]) } $($body)*);
    };
    (
        $item:ident => $relation:ty
        |$tree:ident, $at:ident, $symbols:ident|
        { kinds: [ $( $kind:expr ),+ $(,)? ] as $bits:path, $($body:tt)* }
    ) => {
        $crate::reads!(@syntax $item $relation [$tree $at $symbols] [_model : ()] $crate::languages::Expansions::Ignored,
            { $crate::languages::Reads::Kinds(&[ $( $bits($kind) ),+ ]) } $($body)*);
    };
    (
        $item:ident => $relation:ty
        |$tree:ident, $at:ident, $symbols:ident|
        { kinds: every, $($body:tt)* }
    ) => {
        $crate::reads!(@syntax $item $relation [$tree $at $symbols] [_model : ()] $crate::languages::Expansions::Ignored,
            { $crate::languages::Reads::Every } $($body)*);
    };
    (
        $item:ident => $relation:ty, model ($model_type:ty)
        |$tree:ident, $model:ident, $at:ident, $symbols:ident|
        { kinds: [ $( $kind:expr ),+ $(,)? ] as $bits:path, expanded, $($body:tt)* }
    ) => {
        $crate::reads!(@syntax $item $relation [$tree $at $symbols] [$model : $model_type] $crate::languages::Expansions::Included,
            { $crate::languages::Reads::Kinds(&[ $( $bits($kind) ),+ ]) } $($body)*);
    };
    (
        $item:ident => $relation:ty, model ($model_type:ty)
        |$tree:ident, $model:ident, $at:ident, $symbols:ident|
        { kinds: [ $( $kind:expr ),+ $(,)? ] as $bits:path, $($body:tt)* }
    ) => {
        $crate::reads!(@syntax $item $relation [$tree $at $symbols] [$model : $model_type] $crate::languages::Expansions::Ignored,
            { $crate::languages::Reads::Kinds(&[ $( $bits($kind) ),+ ]) } $($body)*);
    };
    (
        $item:ident => $relation:ty, model ($model_type:ty)
        |$tree:ident, $model:ident, $at:ident, $symbols:ident|
        { kinds: every, $($body:tt)* }
    ) => {
        $crate::reads!(@syntax $item $relation [$tree $at $symbols] [$model : $model_type] $crate::languages::Expansions::Ignored,
            { $crate::languages::Reads::Every } $($body)*);
    };
    (@syntax $item:ident $relation:ty [$tree:ident $at:ident $symbols:ident] [$model:ident : $model_type:ty] $expansions:expr, { $reads:expr } $($body:tt)*) => {
        impl $item {
            #[allow(clippy::redundant_closure_call, reason = "the closure is the scope a failed `require` or `?` leaves")]
            fn rows_at(
                $tree: &$crate::languages::Tree<'_>,
                $model: &$model_type,
                $symbols: &mut $crate::languages::Symbols,
                $at: $crate::languages::Position,
                out: &mut dyn $crate::languages::Rows<<$relation as $crate::languages::CatalogRelation>::Row>,
            ) {
                let _ = $model;
                let _ = (|| -> Option<()> {
                    $crate::__reads_statements!([$tree $symbols out $relation] $($body)*);
                    Some(())
                })();
            }
        }

        impl $crate::languages::declared::Syntax for $item {}

        impl $crate::languages::Visiting<$relation, $model_type> for $item {
            fn visit(
                &self,
                tree: &$crate::languages::Tree<'_>,
                model: &$model_type,
                symbols: &mut $crate::languages::Symbols,
                position: $crate::languages::Position,
                out: &mut dyn $crate::languages::Rows<<$relation as $crate::languages::CatalogRelation>::Row>,
            ) {
                Self::rows_at(tree, model, symbols, position, out);
            }
        }

        impl $crate::languages::ReadItem for $item {
            type Relation = $relation;
            type Model = $model_type;

            const READER: $crate::languages::Reader<$relation, $model_type> = $crate::languages::Reader {
                reads: $reads,
                visit: &$item,
                expansions: $expansions,
                relation: ::std::marker::PhantomData,
            };
        }
    };
    (
        $item:ident => $relation:ty, manifests ($derived:ty)
        |$input:ident, $symbols:ident|
        model $model:ident : $model_type:ty = ($source:expr)
        { $($body:tt)* }
    ) => {
        impl $item {
            fn read(
                $input: &$crate::languages::ManifestInput<'_, $derived>,
                symbols: &mut $crate::languages::Symbols,
                out: &mut dyn $crate::languages::Rows<<$relation as $crate::languages::CatalogRelation>::Row>,
            ) {
                if let Some(model) = $source {
                    Self::rows(&model, symbols, out);
                }
            }

            #[allow(clippy::redundant_closure_call, reason = "the closure is the scope a failed `require` or `?` leaves")]
            pub(crate) fn rows(
                $model: &$model_type,
                $symbols: &mut $crate::languages::Symbols,
                out: &mut dyn $crate::languages::Rows<<$relation as $crate::languages::CatalogRelation>::Row>,
            ) {
                let _ = (|| -> Option<()> {
                    $crate::__reads_statements!([$model $symbols out $relation] $($body)*);
                    Some(())
                })();
            }
        }

        impl $crate::languages::declared::Manifest for $item {}

        impl $crate::languages::ManifestItem for $item {
            type Relation = $relation;
            type Derived = $derived;

            const READER: $crate::languages::ManifestReader<$relation, $derived> =
                $crate::languages::ManifestReader {
                    read: Self::read,
                    relation: ::std::marker::PhantomData,
                };
        }
    };
}
