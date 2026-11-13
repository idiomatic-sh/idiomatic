//! The language that read a file, as the run's source of its rows and its tree: it walks the file for what the run asks and
//! admits what it wrote against the file, keeping the refusal it met for core to raise unchanged.
use super::super::errors::FileAnalysisError;
use super::super::types::LanguageSource;
use crate::domain::admission::errors::AdmissionError;
use crate::domain::admission::service::{admit_columns, admit_elements, limits_of};
use crate::outbound::language::types::FileToRead;
use ryo_execution_runtime::outbound::facts::errors::FactsError;
use ryo_execution_runtime::outbound::facts::types::fact_sources::value::{FactSource, TreeSource};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{ColumnsView, Relation, RowWriter};
use ryo_values::domain::stored_facts::types::ReadInPart;

impl LanguageSource<'_> {
    /// Keeps the refusal of what the language wrote, with the language's name, and answers the port's refusal.
    fn refuse(&mut self, code: AdmissionError) -> FactsError {
        *self.refused = Some(FileAnalysisError::Provider {
            provider: self
                .language
                .linked()
                .0
                .get(self.number.0)
                .map(|name| name.0.to_string())
                .unwrap_or_default(),
            code,
        });
        FactsError::Refused
    }
}

impl FactSource for LanguageSource<'_> {
    fn serves(&self) -> &[Relation] {
        self.serves
    }

    fn read(
        &mut self,
        symbols: &mut Symbols,
        wanted: &[Relation],
        out: &mut dyn RowWriter,
    ) -> Result<Vec<ReadInPart>, FactsError> {
        self.language.walk(
            self.number,
            FileToRead {
                wanted,
                ..self.file
            },
            self.tree,
            symbols,
            out,
        );
        // The tree is admitted once, before any of the rows read of it.
        let limits = limits_of(self.tree, self.file.bytes, self.file.file);
        admit_elements(self.tree, &limits).map_err(|code| self.refuse(code))?;
        // What the readers said is read in part they said to `out`, which the run keeps: the language says nothing more.
        Ok(Vec::new())
    }

    fn admit(&mut self, _relation: Relation, rows: &ColumnsView<'_>) -> Result<(), FactsError> {
        let limits = limits_of(self.tree, self.file.bytes, self.file.file);
        admit_columns(rows, &limits).map_err(|code| self.refuse(code))
    }
}

impl TreeSource for LanguageSource<'_> {
    fn tree(&self) -> &Elements {
        self.tree
    }
}
