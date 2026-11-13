//! The typed rows of each relation as a fixture states them, one module for each relation of the catalog, made from the columns the
//! relation's item declares: one typed value for each column, so a column left out, one added, or one of another kind does not compile.

/// The type a fixture states a column of `kind` with.
macro_rules! stated {
    (Node) => {
        ryo_relations::domain::relations::types::Position
    };
    (Number) => {
        $crate::inbound::use_cases::parse::fixtures::expected::Count
    };
    (Symbol) => {
        $crate::inbound::use_cases::parse::fixtures::expected::Text
    };
    (File) => {
        $crate::inbound::use_cases::parse::fixtures::expected::FileName
    };
    (Span) => {
        $crate::inbound::use_cases::parse::fixtures::expected::Stretch
    };
    (Flag) => {
        bool
    };
}

/// The cell a value stated with `stated` is.
macro_rules! cell {
    (Node, $value:expr) => {
        $crate::inbound::use_cases::parse::fixtures::expected::Cell::Node($value.0)
    };
    (Number, $value:expr) => {
        $crate::inbound::use_cases::parse::fixtures::expected::Cell::Number(
            ryo_relations::domain::relations::types::Integer($value.0),
        )
    };
    (Symbol, $value:expr) => {
        $value.cell()
    };
    (File, $value:expr) => {
        $crate::inbound::use_cases::parse::fixtures::expected::Cell::File($value.0.to_owned())
    };
    (Span, $value:expr) => {
        $value.into()
    };
    (Flag, $value:expr) => {
        $crate::inbound::use_cases::parse::fixtures::expected::Cell::Flag($value)
    };
}

/// One row of a relation as a fixture states it, from the columns its item declares.
macro_rules! expected_row {
    (
        item $item:ident, row { $( #[$row_doc:meta] )* $row:ident }, column { $( #[$column_doc:meta] )* $column:ident },
        expected { $( #[$expected_doc:meta] )* $expected:ident }, class $class:ident, scope $scope:ident, notes $notes:ident,
        columns { $( $( #[$doc:meta] )* $field:ident $variant:ident : $ty:ty => $kind:ident $( ( $convert:path ) )? ; )* }
    ) => {
        $( #[$expected_doc] )*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $expected {
            $( $( #[$doc] )* pub $field: stated!($kind), )*
        }

        impl $crate::inbound::use_cases::parse::fixtures::expected::ExpectedRow for $expected {
            const RELATION: ryo_relations::domain::relations::types::Relation = ryo_relations::domain::relations::types::Relation::$item;

            fn cells(&self) -> Vec<$crate::inbound::use_cases::parse::fixtures::expected::Cell> {
                vec![ $( cell!($kind, self.$field) ),* ]
            }
        }
    };
}

/// A module for each relation of the catalog, in the order the catalog lists them.
macro_rules! expected_modules {
    (
        syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        manifest { $( $manifest_variant:ident $manifest_module:ident $manifest_row:ident $manifest_column:ident ; )* }
        engine { $( $engine_variant:ident $engine_module:ident $engine_row:ident $engine_column:ident ; )* }
    ) => {
        $(
            pub mod $module {
                //! The rows of one relation as a fixture states them.

                ryo_relations::$module!(expected_row);
            }
        )*
        $(
            pub mod $manifest_module {
                //! The rows of one relation as a fixture states them.

                ryo_relations::$manifest_module!(expected_row);
            }
        )*
        $(
            pub mod $engine_module {
                //! The rows of one relation as a fixture states them.

                ryo_relations::$engine_module!(expected_row);
            }
        )*
    };
}

ryo_relations::each_relation!(expected_modules);
