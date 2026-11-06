//! The writer that keeps the rows a language wrote, for the contract and the tests to read.

use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::types::{Bits, FileId, Missing, Relation, Rows};

/// A writer that keeps the rows it is given, each as the relation it is of and its numbers, one
/// row after another.
#[derive(Default, Debug)]
pub struct Collect {
    /// The relation of each row, in the order they were written.
    pub relations: Vec<Relation>,
    /// The numbers of every row, one row after another.
    pub words: Vec<u64>,
    /// The relations said to be partly read, each with its cause.
    pub partials: Vec<(Relation, String, Missing)>,
    /// The relations said to be partly read in one file of the run, each with the file and its cause.
    pub partials_at: Vec<(Relation, FileId, String)>,
    /// Reusable cause buffers after clear, so the collector adds no warmed allocations.
    pub cause_buffers: Vec<String>,
}

impl Collect {
    /// Forgets the rows, keeping the room.
    pub fn clear(&mut self) {
        self.relations.clear();
        self.words.clear();
        self.cause_buffers
            .extend(self.partials.drain(..).map(|(_, cause, _)| cause));
        self.cause_buffers
            .extend(self.partials_at.drain(..).map(|(_, _, cause)| cause));
    }

    /// Reuses a cause buffer retained from the previous file.
    pub(super) fn cause(&mut self, text: &str) -> String {
        let mut buffer = self.cause_buffers.pop().unwrap_or_default();
        buffer.clear();
        buffer.push_str(text);
        buffer
    }

    /// How many rows were written of `relation`.
    #[must_use]
    pub fn count(&self, relation: Relation) -> usize {
        self.relations
            .iter()
            .filter(|each| **each == relation)
            .count()
    }

    /// The rows, each a relation and its numbers.
    #[must_use]
    pub fn rows(&self) -> Vec<(Relation, Vec<u64>)> {
        let mut rest = self.words.as_slice();
        self.relations
            .iter()
            .map(|relation| {
                let (row, after) = rest.split_at(columns(*relation).len());
                rest = after;
                (*relation, row.to_vec())
            })
            .collect()
    }

    /// Keeps one row: its relation, and its words after those of the rows kept before.
    pub(super) fn keep(&mut self, relation: Relation, words: &[Bits]) {
        self.relations.push(relation);
        self.words.extend(words.iter().map(|word| word.0));
    }
}

/// What `Collect` writes rows with, one `Rows` for each row type of the catalog, generated from its one list.
macro_rules! collectors {
    (
        $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )*
    ) => {
        $( $(
            impl Rows<ryo_relations::domain::relations::collections::relations::$module::item::$row> for Collect {
                fn write(&mut self, row: ryo_relations::domain::relations::collections::relations::$module::item::$row) {
                    let words = <ryo_relations::domain::relations::collections::relations::$module::item::$variant as RelationsItem>::words(row);
                    self.keep(Relation::$variant, words.as_ref());
                }

                fn partial(&mut self, cause: ryo_relations::domain::relations::types::ReportText<'_>, missing: Missing) {
                    if !self.partials.iter().any(|(relation, each, how)| *relation == Relation::$variant && each == cause.0 && *how == missing) {
                        let cause = self.cause(cause.0);
                        self.partials.push((Relation::$variant, cause, missing));
                    }
                }

                fn partial_at(&mut self, file: ryo_relations::domain::relations::types::FileId, cause: ryo_relations::domain::relations::types::ReportText<'_>, _missing: Missing) {
                    if !self.partials_at.iter().any(|(relation, at, each)| *relation == Relation::$variant && *at == file && each == cause.0) {
                        let cause = self.cause(cause.0);
                        self.partials_at.push((Relation::$variant, file, cause));
                    }
                }
            }
        )* )*
    };
}

ryo_relations::each_relation!(collectors);
