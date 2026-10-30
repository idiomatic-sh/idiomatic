//! The statements a declaration of `reads!` is made of, one rule each. Every body runs in a scope that a failed `require` or a `given`
//! that ends in `?` leaves: for a reader of trees that is the visit of one element, inside an `each` the turn of one item.
//!
//! The context `[tree symbols out relation]` is passed down in the tokens the user and the macro wrote, so each name is the same name
//! in every rule: a user's expression sees the names the declaration gave.

#[doc(hidden)]
#[macro_export]
macro_rules! __reads_statements {
    ($context:tt) => {};
    // A condition the element must meet; a failed one writes nothing for it.
    ($context:tt require $condition:expr; $($rest:tt)*) => {
        if !($condition) {
            return None;
        }
        $crate::__reads_statements!($context $($rest)*);
    };
    // A value the rows need; one that ends in `?` and is `None` writes nothing for the element.
    ($context:tt given $name:ident = $value:expr; $($rest:tt)*) => {
        let $name = $value;
        $crate::__reads_statements!($context $($rest)*);
    };
    // A row of the relation: every column is named and given from one of the forms of `reads!`.
    (
        [$tree:ident $symbols:ident $out:ident $relation:ty]
        row { $( $column:ident : $form:tt $( ( $($argument:tt)* ) )? ),+ $(,)? }
        $($rest:tt)*
    ) => {
        {
            type Row = <$relation as $crate::languages::CatalogRelation>::Row;
            $out.write(Row {
                $( $column: $crate::__reads_value!($tree $form $( ( $($argument)* ) )?), )+
            });
        }
        $crate::__reads_statements!([$tree $symbols $out $relation] $($rest)*);
    };
    // The relation is read only in part for this element: the cause, and which rows may be missing.
    (
        [$tree:ident $symbols:ident $out:ident $relation:ty]
        partial $cause:expr => $missing:expr;
        $($rest:tt)*
    ) => {
        $out.partial($crate::languages::ReportText(&$cause), $missing);
        $crate::__reads_statements!([$tree $symbols $out $relation] $($rest)*);
    };
    // The relation is read only in part for the rows about one file of the run.
    (
        [$tree:ident $symbols:ident $out:ident $relation:ty]
        partial_at $file:expr, $cause:expr => $missing:expr;
        $($rest:tt)*
    ) => {
        $out.partial_at($file, $crate::languages::ReportText(&$cause), $missing);
        $crate::__reads_statements!([$tree $symbols $out $relation] $($rest)*);
    };
    // One branch for each shape of a value a model function answers.
    (
        $context:tt
        when $model:ident ( $($argument:tt)* ) { $( $pattern:pat => { $($branch:tt)* } )+ }
        $($rest:tt)*
    ) => {
        match $model($($argument)*) {
            $( $pattern => {
                $crate::__reads_statements!($context $($branch)*);
            } )+
        }
        $crate::__reads_statements!($context $($rest)*);
    };
    // The same statements for each item of a list, with the item's place in the list from zero.
    (
        $context:tt
        each ($list:expr) as ($ordinal:ident, $item:ident) { $($body:tt)* }
        $($rest:tt)*
    ) => {
        for ($ordinal, $item) in ::std::iter::IntoIterator::into_iter($list).enumerate() {
            let $ordinal = $crate::languages::ChildOrdinal(u32::try_from($ordinal).unwrap_or(u32::MAX));
            let _ = (|| -> Option<()> {
                $crate::__reads_statements!($context $($body)*);
                Some(())
            })();
        }
        $crate::__reads_statements!($context $($rest)*);
    };
    (
        $context:tt
        each ($list:expr) as $item:ident { $($body:tt)* }
        $($rest:tt)*
    ) => {
        for $item in ::std::iter::IntoIterator::into_iter($list) {
            let _ = (|| -> Option<()> {
                $crate::__reads_statements!($context $($body)*);
                Some(())
            })();
        }
        $crate::__reads_statements!($context $($rest)*);
    };
}

/// The value of one column of a row, from the form that names it.
#[doc(hidden)]
#[macro_export]
macro_rules! __reads_value {
    ($tree:ident node($at:expr)) => {
        $crate::languages::node_id($tree.file, $at)
    };
    ($tree:ident file) => {
        $tree.file
    };
    ($tree:ident span($at:expr)) => {
        $crate::languages::Span {
            start: $crate::languages::start_at($tree.elements, $at),
            end: $crate::languages::end_at($tree.elements, $at),
        }
    };
    ($tree:ident text($at:expr)) => {
        $crate::languages::text_at($tree.elements, $at)
    };
    ($tree:ident value($value:expr)) => {
        $value
    };
}
