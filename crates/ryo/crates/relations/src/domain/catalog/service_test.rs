//! The `catalog` service's cases: the one list names every relation of the catalog once, each relation's form agrees with its
//! node columns and its keys, and each declares what its form requires: a node-table relation its extent and nothing else, a
//! filled relation its size and its key columns, which are columns of its own, each once.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::catalog::service::{
    fact_form, key_columns, needs_notes, node_column, node_extent, relation_size, scope_of,
};
use crate::domain::catalog::types::{RelationNotes, RowScope};
use crate::domain::relations::collections::relations::interface::RelationsItem;
use crate::domain::relations::service::{all, columns, is_key, relation_of};
use crate::domain::relations::types::{Column, ColumnKeyness};
use crate::domain::relations::types::{FactForm, FilledKey, NodeColumn, NodeExtent, RelationSize};

/// How many relations the one list of the catalog names, counted by the macro that every crate of the engine calls.
macro_rules! listed_relations {
    (
        syntax { $( $syntax:ident $syntax_module:ident $syntax_row:ident $syntax_column:ident; )* }
        manifest { $( $manifest:ident $manifest_module:ident $manifest_row:ident $manifest_column:ident; )* }
        engine { $( $engine:ident $engine_module:ident $engine_row:ident $engine_column:ident; )* }
    ) => {
        fn the_list_names_as_many_relations_as_the_catalog_has() -> usize {
            usize::from(
                [$( stringify!($syntax), )* $( stringify!($manifest), )* $( stringify!($engine), )*].len()
                    == all().len(),
            )
        }
    };
}

crate::each_relation!(listed_relations);

/// The relations whose form disagrees with their node columns: a `NodeTable` relation that leaves a column unmapped or maps one
/// twice, a filled relation that maps any; and the columns `is_key` calls a key that the form does not index as one.
macro_rules! form_faults {
    (
        $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )*
    ) => {
        fn node_table_relations_map_every_column_once() -> usize {
            let mut faults = 0;
            $( $(
                {
                    type Item = crate::domain::relations::collections::relations::$module::item::$variant;
                    let mapped = <Item as RelationsItem>::NODE_COLUMNS;
                    let fits = match <Item as RelationsItem>::FORM {
                        FactForm::NodeTable => <Item as RelationsItem>::columns().iter().all(|column| {
                            mapped.iter().filter(|(declared, _)| declared == column).count() == 1
                        }) && mapped.len() == <Item as RelationsItem>::columns().len(),
                        FactForm::Filled(_) => mapped.is_empty(),
                    };
                    faults += usize::from(!fits);
                }
            )* )*
            faults
        }

        fn every_key_column_is_a_form_key() -> usize {
            let mut faults = 0;
            $( $(
                for own in <crate::domain::relations::collections::relations::$module::item::$variant as RelationsItem>::columns() {
                    let column = Column::$variant(own);
                    let keyed = match fact_form(crate::domain::relations::types::Relation::$variant) {
                        FactForm::Filled(FilledKey::Unique(key)) => key == column,
                        FactForm::NodeTable => node_column(column) == Some(NodeColumn::Id),
                        FactForm::Filled(FilledKey::Many(_) | FilledKey::Whole) => false,
                    };
                    faults += usize::from(is_key(column) == ColumnKeyness::Key && !keyed);
                }
            )* )*
            faults
        }
    };
}

crate::each_relation!(form_faults);

/// How many relations of the catalog are read in place from the node table.
fn node_table_relations() -> usize {
    all()
        .iter()
        .filter(|relation| fact_form(**relation) == FactForm::NodeTable)
        .count()
}

/// How many relations of the catalog declare that their rows are the project's.
fn every_catalog_relation_declares_its_scope() -> usize {
    all()
        .iter()
        .filter(|relation| scope_of(**relation) == RowScope::Project)
        .count()
}

/// How many relations of the catalog declare that they are made from the languages' notes.
fn every_catalog_relation_declares_whether_it_needs_notes() -> usize {
    all()
        .iter()
        .filter(|relation| needs_notes(**relation) == RelationNotes::Needed)
        .count()
}

/// The relations whose declarations disagree with their form: a node-table relation with no extent, with a size or with a key
/// column; a filled relation with an extent or with no size.
fn every_relation_declares_what_its_form_requires() -> usize {
    all()
        .iter()
        .filter(|relation| {
            let (extent, size, keys) = (
                node_extent(**relation),
                relation_size(**relation),
                key_columns(**relation),
            );
            match fact_form(**relation) {
                FactForm::NodeTable => extent.is_none() || size.is_some() || !keys.is_empty(),
                FactForm::Filled(_) => extent.is_some() || size.is_none(),
            }
        })
        .count()
}

/// How many relations of the catalog declare a size of this class.
fn sized(size: RelationSize) -> usize {
    all()
        .iter()
        .filter(|relation| relation_size(**relation) == Some(size))
        .count()
}

/// How many relations declare at most one row for a file.
fn relations_of_one_row_for_a_file() -> usize {
    sized(RelationSize::PerFile)
}

/// How many relations declare a bounded number of rows for each element.
fn relations_of_rows_for_each_element() -> usize {
    sized(RelationSize::PerElement)
}

/// How many relations declare a row for a pair of elements.
fn relations_of_a_row_for_a_pair() -> usize {
    sized(RelationSize::PerPair)
}

/// How many relations promise a fill by key for at least one column.
fn relations_that_promise_a_fill_by_key() -> usize {
    all()
        .iter()
        .filter(|relation| !key_columns(**relation).is_empty())
        .count()
}

/// How many key columns the catalog declares in all.
fn declared_key_columns() -> usize {
    all()
        .iter()
        .map(|relation| key_columns(*relation).len())
        .sum()
}

/// The declared key columns that are not a column of their own relation, or are declared twice.
fn key_columns_that_are_not_their_relations_own_once() -> usize {
    all()
        .iter()
        .flat_map(|relation| {
            let keys = key_columns(*relation);
            keys.iter().filter(move |key| {
                relation_of(**key) != *relation
                    || !columns(*relation).contains(key)
                    || keys.iter().filter(|other| other == key).count() != 1
            })
        })
        .count()
}

/// How many node-table relations have rows for the file's own tree alone.
fn node_table_relations_of_the_own_tree() -> usize {
    all()
        .iter()
        .filter(|relation| node_extent(**relation) == Some(NodeExtent::Own))
        .count()
}

/// How many node-table relations have rows for the embedded segments too.
fn node_table_relations_of_the_embedded_segments_too() -> usize {
    all()
        .iter()
        .filter(|relation| node_extent(**relation) == Some(NodeExtent::Embedded))
        .count()
}

/// A scenario: what a row asks of the catalog's list.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_list_names_as_many_relations_as_the_catalog_has"),
        word: Word::Succeeds,
        input: || the_list_names_as_many_relations_as_the_catalog_has,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("every_catalog_relation_declares_its_scope"),
        word: Word::Succeeds,
        input: || every_catalog_relation_declares_its_scope,
        expect: Expect::Value(8),
    },
    Case {
        name: CaseName("every_catalog_relation_declares_whether_it_needs_notes"),
        word: Word::Succeeds,
        input: || every_catalog_relation_declares_whether_it_needs_notes,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("node_table_relations"),
        word: Word::Succeeds,
        input: || node_table_relations,
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("node_table_relations_map_every_column_once"),
        word: Word::Succeeds,
        input: || node_table_relations_map_every_column_once,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("every_key_column_is_a_form_key"),
        word: Word::Succeeds,
        input: || every_key_column_is_a_form_key,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("every_relation_declares_what_its_form_requires"),
        word: Word::Succeeds,
        input: || every_relation_declares_what_its_form_requires,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("relations_of_one_row_for_a_file"),
        word: Word::Succeeds,
        input: || relations_of_one_row_for_a_file,
        expect: Expect::Value(8),
    },
    Case {
        name: CaseName("relations_of_rows_for_each_element"),
        word: Word::Succeeds,
        input: || relations_of_rows_for_each_element,
        expect: Expect::Value(66),
    },
    Case {
        name: CaseName("relations_of_a_row_for_a_pair"),
        word: Word::Succeeds,
        input: || relations_of_a_row_for_a_pair,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("relations_that_promise_a_fill_by_key"),
        word: Word::Succeeds,
        input: || relations_that_promise_a_fill_by_key,
        expect: Expect::Value(12),
    },
    Case {
        name: CaseName("declared_key_columns"),
        word: Word::Succeeds,
        input: || declared_key_columns,
        expect: Expect::Value(13),
    },
    Case {
        name: CaseName("key_columns_that_are_not_their_relations_own_once"),
        word: Word::Succeeds,
        input: || key_columns_that_are_not_their_relations_own_once,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("node_table_relations_of_the_own_tree"),
        word: Word::Succeeds,
        input: || node_table_relations_of_the_own_tree,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("node_table_relations_of_the_embedded_segments_too"),
        word: Word::Succeeds,
        input: || node_table_relations_of_the_embedded_segments_too,
        expect: Expect::Value(2),
    },
];

/// Runs one scenario; the author states it.
fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
