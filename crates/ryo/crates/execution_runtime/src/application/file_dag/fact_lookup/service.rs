//! Everything the run reads of one file's rows, and the only way it reads them: each source reads its part of the relations the
//! run demands once, all of them in one read, its rows are admitted, and the file's partiality is final before any rule runs. A
//! relation outside what was demanded is refused, never read late.

use super::types::{FileWants, RowsRead};
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::FactsWorker;
use crate::outbound::facts::types::fact_sources::value::FactSources;
use crate::outbound::facts::types::WordsToKeep;
use ryo_execution_domain::domain::coverage::service::partial_of;
use ryo_execution_domain::domain::coverage::types::Partial;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{ColumnsView, Relation};
use ryo_values::domain::stored_facts::types::Bucket;

/// The fact lookup of one file: its sources, the worker's facts its rows are written to and read from, and its partiality.
pub(crate) struct FactLookup<'a, 's> {
    /// Where the file's rows come from.
    sources: FactSources<'s>,
    /// Where they are written and read.
    facts: &'a mut dyn FactsWorker,
    /// What the file is read for.
    wants: FileWants<'a>,
    /// The file's physical partiality, once read.
    partials: Vec<Partial>,
    /// Whether the rows were read.
    read: RowsRead,
    /// The relations the first `ensure` demanded: the only ones readable until the file is released.
    ensured: Vec<Relation>,
}

impl<'a, 's> FactLookup<'a, 's> {
    /// A lookup that reads nothing until it is first asked to ensure.
    pub(crate) fn new(
        sources: FactSources<'s>,
        facts: &'a mut dyn FactsWorker,
        wants: FileWants<'a>,
    ) -> Self {
        Self {
            sources,
            facts,
            wants,
            partials: Vec::new(),
            read: RowsRead::Closed,
            ensured: Vec::new(),
        }
    }

    /// Makes `relations` readable for the open file. The first call reads exactly `relations`, each once by the one source that
    /// serves it and all of a source's part in one read, and admits the active ones among them; a relation no source serves is
    /// read as the worker holds it. A later call asking only for what was ensured has nothing left to read. Any failure
    /// releases what was written, so a failed file leaves no row.
    ///
    /// # Errors
    ///
    /// `Refused` when a source refuses; `NotEnsured` for a later call asking for a relation the first did not; the storage's own
    /// failure otherwise.
    pub(crate) fn ensure(
        &mut self,
        symbols: &mut Symbols,
        relations: &[Relation],
    ) -> Result<(), FactsError> {
        if self.read == RowsRead::Read {
            return match relations
                .iter()
                .find(|relation| !self.ensured.contains(relation))
            {
                Some(relation) => Err(FactsError::NotEnsured {
                    relation: *relation,
                }),
                None => Ok(()),
            };
        }
        match self.read_demanded(symbols, relations) {
            Ok(()) => Ok(()),
            Err(error) => {
                let released = self.release();
                match (error, released) {
                    (FactsError::Refused, Err(release)) => Err(release),
                    (error, _) => Err(error),
                }
            }
        }
    }

    /// The first `ensure`'s read: each source asked once for its part of `relations`, the active ones admitted, the partiality
    /// fixed.
    fn read_demanded(
        &mut self,
        symbols: &mut Symbols,
        relations: &[Relation],
    ) -> Result<(), FactsError> {
        self.ensured.extend_from_slice(relations);
        let mut said = Vec::new();
        let mut asked = Vec::with_capacity(relations.len());
        for source in self.sources.each() {
            asked.clear();
            asked.extend(
                relations
                    .iter()
                    .filter(|relation| source.serves().contains(relation))
                    .copied(),
            );
            let mut columns = self.facts.columns_mut()?;
            let answered = source.read(symbols, &asked, &mut columns)?;
            // What the readers said through the writer comes first, then what the source answered.
            said.append(&mut columns.partials);
            said.extend(answered);
        }
        for relation in self
            .wants
            .active
            .iter()
            .filter(|relation| relations.contains(relation))
        {
            for source in self.sources.each() {
                if source.serves().contains(relation) {
                    source.admit(*relation, &self.facts.rows(*relation)?)?;
                }
            }
        }
        said.extend_from_slice(self.wants.said);
        self.partials = partial_of(&said, self.wants.recovery, self.wants.active);
        self.read = RowsRead::Read;
        Ok(())
    }

    /// An ensured relation's rows, borrowed in place: what its source wrote, or what the worker holds of one no source serves.
    /// A relation that was not ensured is refused, never empty.
    ///
    /// # Errors
    ///
    /// `NotEnsured` before anything is ensured, for a relation outside what was, and after release; the storage's own failure
    /// otherwise.
    pub(crate) fn columns(&self, relation: Relation) -> Result<ColumnsView<'_>, FactsError> {
        if self.read == RowsRead::Read && self.ensured.contains(&relation) {
            self.facts.rows(relation)
        } else {
            Err(FactsError::NotEnsured { relation })
        }
    }

    /// The file's physical partiality so far: `coverage::partial_of(said, recovery, active)`.
    pub(crate) fn partials(&self) -> &[Partial] {
        &self.partials
    }

    /// Ends the file: the rows go, and nothing is readable until the next `ensure`.
    ///
    /// # Errors
    ///
    /// The storage's own failure.
    pub(crate) fn release(&mut self) -> Result<(), FactsError> {
        self.read = RowsRead::Closed;
        self.ensured.clear();
        self.facts.release()
    }

    /// The file's tree: the reader's, or core's empty table for a file no language read.
    pub(crate) fn tree(&self) -> &Elements {
        self.sources.tree()
    }

    /// Keeps a rule's captured words in the worker's facts: the one write the rule loop makes, so it never holds the worker.
    ///
    /// # Errors
    ///
    /// The storage's own failure.
    pub(crate) fn keep(&self, bucket: Bucket, words: WordsToKeep<'_>) -> Result<(), FactsError> {
        self.facts.keep(bucket, words)
    }
}
