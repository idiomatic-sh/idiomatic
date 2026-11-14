//! The types of the `facts` port: plain data, public fields and derives, with no constructor and
//! no inherent impl.

pub mod fact_sources;

use ryo_relations::domain::relations::types::{FileId, Relation, Rows};

use ryo_relations::domain::relations::types::FactColumn;

use ryo_values::domain::stored_facts::types::{KeptWords, PackedWords};
use ryo_values::domain::stored_facts::types::{PartialAt, PartialCause, PartialScope, ReadInPart};

/// Whether resident fact rows remain unchanged during one file's read/keep phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResidentStability {
    /// Values, kinds, row order and population survive `rows` of any relation and `keep`,
    /// until the next `columns_mut` or `release` call.
    Stable,
    /// No immutable resident snapshot is promised; callers must build invocation-local indexes.
    Unstable,
}

/// Whether a source of the open file serves a relation: one no source serves is core's own, written to the worker by core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceServes {
    /// A source of the file makes its rows.
    Served,
    /// No source of the file makes them.
    Unserved,
}

/// One physical file's packed frontier, with the identity and coverage issued
/// by the owner that captured it. The proof is required even for empty rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordsToKeep<'a> {
    pub words: &'a PackedWords,
    pub file: FileId,
    pub proof: &'a ryo_values::domain::records::types::FrontierProof,
}

/// Where a source range begins inside the concatenated bucket words: the bare offset inside is named here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeptWordsStart(pub usize);

/// How many bucket words a source range spans: the bare count inside is named here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeptWordsLength(pub usize);

/// One exact source range inside the concatenated bucket bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BucketSource {
    pub file: FileId,
    pub start: KeptWordsStart,
    pub length: KeptWordsLength,
    pub proof: ryo_values::domain::records::types::FrontierProof,
}

/// The existing bucket payload and its mandatory per-source provenance. The words are shared, not copied: the finish hands out
/// the one buffer that holds them, and every reader of the bucket borrows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptBucket {
    /// The bucket's words, the one buffer every reader of the bucket borrows.
    pub words: std::sync::Arc<KeptWords>,
    /// The provenance of each source whose words the bucket holds.
    pub sources: Vec<BucketSource>,
}

/// What the columns the worker's file writes to are made of, generated from the catalog's one list: a field for
/// each relation, and the rows of each pushed onto its own columns.
macro_rules! columns_mut {
    (
        $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )*
    ) => {
        /// The columns of every relation that a worker's file writes to: the worker's own, borrowed, so a
        /// row is pushed onto them directly, with static dispatch and nothing that can fail. A caller asks
        /// the port for them once for a file, never for a row, and gives the engine's writer to the readers
        /// one relation at a time.
        #[derive(Debug, Default)]
        pub struct ColumnsMut<'a> {
            /// The relations a reader said it read only in part, in the file being written, each with its cause.
            pub partials: Vec<ReadInPart>,
            $( $(
                #[doc = stringify!($module)]
                pub $module: &'a mut [FactColumn],
            )* )*
        }

        impl<'a> FromIterator<(Relation, &'a mut [FactColumn])> for ColumnsMut<'a> {
            fn from_iter<Each>(each: Each) -> Self where Each: IntoIterator<Item = (Relation, &'a mut [FactColumn])> {
                let mut columns = Self::default();
                for (relation, each) in each {
                    match relation {
                        $( $( Relation::$variant => columns.$module = each, )* )*
                    }
                }
                columns
            }
        }

        $( $(
            impl Rows<::ryo_relations::domain::relations::collections::relations::$module::item::$row> for ColumnsMut<'_> {
                fn write(&mut self, row: ::ryo_relations::domain::relations::collections::relations::$module::item::$row) {
                    let words = <::ryo_relations::domain::relations::collections::relations::$module::item::$variant as ::ryo_relations::domain::relations::collections::relations::interface::RelationsItem>::words(row);
                    let row_numbers: &[::ryo_relations::domain::relations::types::Bits] = words.as_ref();
                    for (column, number) in self.$module.iter_mut().zip(row_numbers) {
                        column.0.push(number.0);
                    }
                }

                fn partial(&mut self, cause: ::ryo_relations::domain::relations::types::ReportText<'_>, missing: ::ryo_relations::domain::relations::types::Missing) {
                    let (cause, at) = (cause.0, PartialScope::Repository);
                    let relation = <::ryo_relations::domain::relations::collections::relations::$module::item::$variant as ::ryo_relations::domain::relations::collections::relations::interface::RelationsItem>::relation();
                    if !self.partials.iter().any(|each| each.relation == relation && &*each.cause.0 == cause && each.lacking == missing && each.at == at) {
                        self.partials.push(ReadInPart { relation, cause: PartialCause(cause.into()), lacking: missing, at });
                    }
                }

                fn partial_at(&mut self, file: FileId, cause: ::ryo_relations::domain::relations::types::ReportText<'_>, missing: ::ryo_relations::domain::relations::types::Missing) {
                    let (cause, at) = (cause.0, PartialScope::OneFile(PartialAt(file.0)));
                    let relation = <::ryo_relations::domain::relations::collections::relations::$module::item::$variant as ::ryo_relations::domain::relations::collections::relations::interface::RelationsItem>::relation();
                    if !self.partials.iter().any(|each| each.relation == relation && &*each.cause.0 == cause && each.lacking == missing && each.at == at) {
                        self.partials.push(ReadInPart { relation, cause: PartialCause(cause.into()), lacking: missing, at });
                    }
                }
            }
        )* )*
    };
}

::ryo_relations::each_relation!(columns_mut);
