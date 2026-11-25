//! The statements a rust relation's reader is written in, as the `fills!` item of the walk runs them. They are the statements of
//! `ryo::reads!` (`require`, `given`, `row`, `partial`, `when`, `each`), and the item's body is the same text: each value written for
//! a row is made a cell of the file's words, because the fillers write the words of the run where a reader wrote a name of the
//! file. The reader visits the positions of the kinds it names, the file's own and, when it says `expanded`, the macro expansions'.

/// Declares the filler of a relation from the statements of its reader.
macro_rules! fills_rows {
    (
        $item:ident => $relation:ty, model ($model_type:ty)
        |$tree:ident, $model:ident, $at:ident, $symbols:ident|
        { kinds: [ $( $kind:expr ),+ $(,)? ] as $bits:path, expanded, $($body:tt)* }
    ) => {
        $crate::domain::file_view::macros::fills_rows!(@fill $item $relation [$tree $model $at $symbols] true
            [ $( $bits($kind) ),+ ] $($body)*);
    };
    (
        $item:ident => $relation:ty, model ($model_type:ty)
        |$tree:ident, $model:ident, $at:ident, $symbols:ident|
        { kinds: [ $( $kind:expr ),+ $(,)? ] as $bits:path, $($body:tt)* }
    ) => {
        $crate::domain::file_view::macros::fills_rows!(@fill $item $relation [$tree $model $at $symbols] false
            [ $( $bits($kind) ),+ ] $($body)*);
    };
    (@fill $item:ident $relation:ty [$tree:ident $model:ident $at:ident $symbols:ident] $expanded:tt [ $( $kind:expr ),+ ] $($body:tt)*) => {
        ryo::fills! {
            $item => $relation, walk($crate::domain::file_view::types::RustWalk)
            |file, words, rows| {
                let (owned, $symbols, $model, _table) = $crate::domain::file_view::service::split(file);
                let $tree = &owned;
                let $model: &_ = &*$model;
                let wanted = [ $( $kind ),+ ];
                for $at in $crate::domain::file_view::service::visited($tree.elements, $expanded) {
                    if !wanted.contains(&ryo::languages::kind_at($tree.elements, $at)) {
                        continue;
                    }
                    let mut scope = || -> Option<()> {
                        $crate::domain::file_view::macros::fills_statements!(
                            [$tree $symbols rows words $relation] $($body)*);
                        Some(())
                    };
                    let _ = scope();
                }
            }
        }
    };
}
pub(crate) use fills_rows;

/// One rule for each statement of a reader's body.
macro_rules! fills_statements {
    ($context:tt) => {};
    ($context:tt require $condition:expr; $($rest:tt)*) => {
        if !($condition) {
            return None;
        }
        $crate::domain::file_view::macros::fills_statements!($context $($rest)*);
    };
    ($context:tt given $name:ident = $value:expr; $($rest:tt)*) => {
        let $name = $value;
        $crate::domain::file_view::macros::fills_statements!($context $($rest)*);
    };
    (
        [$tree:ident $symbols:ident $out:ident $words:ident $relation:ty]
        row { $( $column:ident : $form:tt $( ( $($argument:tt)* ) )? ),+ $(,)? }
        $($rest:tt)*
    ) => {
        {
            type Row = <$relation as ryo::languages::CatalogRelation>::Row;
            $out.write(Row {
                $( $column: $crate::domain::file_view::macros::fills_value!($tree $symbols $words $form $( ( $($argument)* ) )?), )+
            });
        }
        $crate::domain::file_view::macros::fills_statements!([$tree $symbols $out $words $relation] $($rest)*);
    };
    (
        [$tree:ident $symbols:ident $out:ident $words:ident $relation:ty]
        partial $cause:expr => $missing:expr;
        $($rest:tt)*
    ) => {
        $out.partial(ryo::languages::ReportText(&$cause), $missing);
        $crate::domain::file_view::macros::fills_statements!([$tree $symbols $out $words $relation] $($rest)*);
    };
    (
        $context:tt
        when $model:ident ( $($argument:tt)* ) { $( $pattern:pat => { $($branch:tt)* } )+ }
        $($rest:tt)*
    ) => {
        match $model($($argument)*) {
            $( $pattern => {
                $crate::domain::file_view::macros::fills_statements!($context $($branch)*);
            } )+
        }
        $crate::domain::file_view::macros::fills_statements!($context $($rest)*);
    };
    (
        $context:tt
        each ($list:expr) as ($ordinal:ident, $item:ident) { $($body:tt)* }
        $($rest:tt)*
    ) => {
        for ($ordinal, $item) in ::std::iter::IntoIterator::into_iter($list).collect::<Vec<_>>().into_iter().enumerate() {
            let $ordinal = ryo::languages::ChildOrdinal(u32::try_from($ordinal).unwrap_or(u32::MAX));
            let _ = (|| -> Option<()> {
                $crate::domain::file_view::macros::fills_statements!($context $($body)*);
                Some(())
            })();
        }
        $crate::domain::file_view::macros::fills_statements!($context $($rest)*);
    };
    (
        $context:tt
        each ($list:expr) as $item:ident { $($body:tt)* }
        $($rest:tt)*
    ) => {
        for $item in ::std::iter::IntoIterator::into_iter($list).collect::<Vec<_>>() {
            let _ = (|| -> Option<()> {
                $crate::domain::file_view::macros::fills_statements!($context $($body)*);
                Some(())
            })();
        }
        $crate::domain::file_view::macros::fills_statements!($context $($rest)*);
    };
}
pub(crate) use fills_statements;

/// The cell of one column of a row, from the form that names it.
macro_rules! fills_value {
    ($tree:ident $symbols:ident $words:ident node($at:expr)) => {
        ryo::languages::node_id($tree.file, $at)
    };
    ($tree:ident $symbols:ident $words:ident file) => {
        $tree.file
    };
    ($tree:ident $symbols:ident $words:ident span($at:expr)) => {
        ryo::languages::Span {
            start: ryo::languages::start_at($tree.elements, $at),
            end: ryo::languages::end_at($tree.elements, $at),
        }
    };
    ($tree:ident $symbols:ident $words:ident text($at:expr)) => {
        $crate::domain::file_view::service::cell(
            ryo::languages::text_at($tree.elements, $at), &*$symbols, $tree.source, &mut *$words)
    };
    ($tree:ident $symbols:ident $words:ident value($value:expr)) => {
        {
            let made = $value;
            $crate::domain::file_view::service::cell(made, &*$symbols, $tree.source, &mut *$words)
        }
    };
}
pub(crate) use fills_value;
