//! The cases of discovery failures, primary manifest failures and project entry acquisition.
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::corpus::types::CorpusInventory;
use crate::outbound::corpus::types::PathStat;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::language::interface::LanguagePort;
use crate::outbound::language::types::ProjectToRead;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::project_context::types::EntryObservation;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::files::types::FileBytes;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

pub(crate) struct CountingCorpus {
    pub(crate) inner: CorpusFake,
    pub(crate) observations: Arc<AtomicUsize>,
}

impl CorpusPort for CountingCorpus {
    fn inventory(&self) -> Result<CorpusInventory, CorpusError> {
        self.inner.inventory()
    }
    fn read(&self, entry: &CorpusEntry, into: &mut FileBytes) -> Result<(), CorpusError> {
        self.inner.read(entry, into)
    }
    fn stat(&self, path: &FilePath) -> Result<PathStat, CorpusError> {
        self.inner.stat(path)
    }
    fn observe_entry(&self, path: &FilePath) -> EntryObservation {
        self.observations.fetch_add(1, Ordering::Relaxed);
        self.inner.observe_entry(path)
    }
}

/// A language that answers the project without the entries it was given to observe.
pub(crate) struct Forgetful(pub(crate) LanguageFake);

impl LanguagePort for Forgetful {
    fn relations(
        &self,
    ) -> Result<
        crate::outbound::language::types::RelationList,
        crate::outbound::language::errors::LanguageError,
    > {
        self.0.relations()
    }
    fn linked(&self) -> crate::outbound::language::types::LinkedNames {
        self.0.linked()
    }
    fn kind_vocabularies(&self) -> ryo_values::domain::descriptions::types::KindVocabularies {
        self.0.kind_vocabularies()
    }
    fn unknown_reasons(&self) -> crate::outbound::language::types::UnknownReasons {
        self.0.unknown_reasons()
    }
    fn supports(&self, relation: Relation) -> crate::outbound::language::types::Answers {
        self.0.supports(relation)
    }
    fn reads_manifest(&self, path: &FilePath) -> crate::outbound::language::types::ManifestReading {
        self.0.reads_manifest(path)
    }
    fn parse(
        &self,
        file: crate::outbound::language::types::FileToRead<'_>,
        table: &mut ryo_language::domain::elements::types::Elements,
        symbols: &mut Symbols,
        out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
    ) -> crate::outbound::language::types::FileReading {
        self.0.parse(file, table, symbols, out)
    }
    fn project_entries(
        &self,
        input: &ProjectToRead<'_>,
    ) -> ryo_language::domain::files::types::ProjectEntries {
        self.0.project_entries(input)
    }
    fn project(
        &self,
        input: &ProjectToRead<'_>,
        symbols: &mut Symbols,
        out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
    ) -> Result<
        crate::outbound::language::types::ProjectAnswer,
        crate::outbound::language::errors::LanguageError,
    > {
        let mut answer = self.0.project(input, symbols, out)?;
        answer.observed_entries.clear();
        Ok(answer)
    }
}
