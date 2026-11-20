//! The `catalog` service's cases: the one list names every relation of the catalog once, and each relation's form agrees with its
//! node columns and its keys.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::catalog::service::{fact_form, needs_notes, node_column, scope_of};
use crate::domain::catalog::types::{RelationNotes, RowScope};
use crate::domain::relations::collections::relations::interface::RelationsItem;
use crate::domain::relations::service::{all, is_key};
use crate::domain::relations::types::{Column, ColumnKeyness};
use crate::domain::relations::types::{FactForm, FilledKey, NodeColumn};

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
];

/// Runs one scenario; the author states it.
fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
