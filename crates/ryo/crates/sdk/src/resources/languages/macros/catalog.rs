//! The names of the catalog in the SDK: `catalog_names` writes them where the `languages` module calls it, over the catalog's one list.

/// The names of the catalog in the SDK, generated from the catalog's one list: the relation, its row and its columns.
macro_rules! catalog_names {
    (
        $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )*
    ) => {
        $( $(
            pub use ::ryo_language::domain::relations::collections::relations::$module::item::{$variant, $row, $column};
        )* )*
    };
}

pub(crate) use catalog_names;
