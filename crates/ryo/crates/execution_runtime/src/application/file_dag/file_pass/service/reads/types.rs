//! The types of the open file's reads: each relation's one read, kept for the file.

/// What the lookup answered for one relation in the open file: its rows, or why it refused them. The file reads each relation
/// once: every reader of it in the file, and activation, answer this read.
#[derive(Debug)]
pub(crate) struct FactRead<'p> {
    /// The relation.
    pub(crate) relation: ryo_relations::domain::relations::types::Relation,
    /// Its rows, lent by the lookup, or the lookup's refusal.
    pub(crate) read: Result<
        ryo_relations::domain::relations::types::ColumnsView<'p>,
        crate::outbound::facts::errors::FactsError,
    >,
}

/// Every relation the open file read so far, each once, in the order first read.
#[derive(Debug, Default)]
pub(crate) struct FileReads<'p>(pub(crate) Vec<FactRead<'p>>);
