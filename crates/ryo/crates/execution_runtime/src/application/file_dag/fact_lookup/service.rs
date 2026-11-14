//! Everything the run reads of one file's rows, and the only way it reads them: each source reads its part of what the file wants
//! once, its rows are admitted, and the file's partiality is final before any rule runs. The whole file is read when it opens.

use super::types::{FileWants, RowsRead};
use crate::application::file_dag::program::file_session::types::FileStop;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::FactsWorker;
use crate::outbound::facts::types::fact_sources::value::FactSources;
use ryo_execution_domain::domain::coverage::service::partial_of;
use ryo_execution_domain::domain::coverage::types::Partial;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{ColumnsView, Relation};

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
        }
    }

    /// Makes `relations` readable for the open file. The first call reads every wanted relation, each once by the one source that
    /// serves it, and admits the active ones; a later call has nothing left to read.
    ///
    /// # Errors
    ///
    /// `Refused` when a source refuses; the storage's own failure otherwise.
    pub(crate) fn ensure(
        &mut self,
        symbols: &mut Symbols,
        _relations: &[Relation],
    ) -> Result<(), FactsError> {
        if self.read == RowsRead::Read {
            return Ok(());
        }
        let mut said = Vec::new();
        for source in self.sources.each() {
            let asked: Vec<Relation> = self
                .wants
                .wanted
                .iter()
                .filter(|relation| source.serves().contains(relation))
                .copied()
                .collect();
            let mut columns = self.facts.columns_mut()?;
            let answered = source.read(symbols, &asked, &mut columns)?;
            // What the readers said through the writer comes first, then what the source answered.
            said.append(&mut columns.partials);
            said.extend(answered);
        }
        for relation in self.wants.active {
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

    /// An ensured relation's rows, borrowed in place. A relation that was not ensured is refused, never empty.
    ///
    /// # Errors
    ///
    /// `Refused` before anything is ensured; the storage's own failure otherwise.
    pub(crate) fn columns(&self, relation: Relation) -> Result<ColumnsView<'_>, FactsError> {
        match self.read {
            RowsRead::Read => self.facts.rows(relation),
            RowsRead::Closed => Err(FactsError::Refused),
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
        self.facts.release()
    }

    /// The file's tree: the reader's, or core's empty table for a file no language read.
    pub(crate) fn tree(&self) -> &Elements {
        self.sources.tree()
    }

    /// The worker's facts, for what the run keeps of the file.
    pub(crate) fn worker(&self) -> &dyn FactsWorker {
        &*self.facts
    }
}

/// Opens one file: every wanted relation read and admitted. A source's refusal releases the rows it wrote and stops the file.
///
/// # Errors
///
/// `Reader` when a source refused, its rows released; `Facts` with the storage's own failure otherwise.
pub(crate) fn opened<'a, 's>(
    sources: FactSources<'s>,
    facts: &'a mut dyn FactsWorker,
    wants: FileWants<'a>,
    symbols: &mut Symbols,
) -> Result<FactLookup<'a, 's>, FileStop> {
    let mut lookup = FactLookup::new(sources, facts, wants);
    match lookup.ensure(symbols, wants.active) {
        Ok(()) => Ok(lookup),
        Err(FactsError::Refused) => Err(lookup
            .release()
            .err()
            .map_or(FileStop::Reader, FileStop::Facts)),
        Err(other) => Err(FileStop::Facts(other)),
    }
}
