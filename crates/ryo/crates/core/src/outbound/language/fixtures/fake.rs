//! The fake of the `language` port: what the core's tests stand in for it with.

use crate::outbound::language::types::{Answers, LinkedNames, RelationList, UnknownReasons};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::terms::types::NameText;
use ryo_values::domain::descriptions::types::{KindVocabularies, KindVocabulary};
use std::sync::atomic::{AtomicUsize, Ordering};

use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::relations::collections::relations::function::item::FunctionRow;
use ryo_language::domain::relations::service::all;
use ryo_language::domain::relations::types::RowWriter;
use ryo_language::domain::relations::types::{KindBits, Offset, Position, Relation, Span};
use ryo_language::domain::symbols::types::Symbols;

use super::super::errors::LanguageError;
use super::super::interface::LanguagePort;
use super::super::types::{
    Absence, FileReading, FileToRead, LinkedName, ManifestReading, ProjectAnswer, ProjectInput,
    RelationAnswer,
};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::descriptions::types::LanguageName;
use ryo_values::domain::stored_facts::types::{LanguageNumber, ReadFailure};

/// What the core's tests stand in for `language` with: it owns the catalog, reads every file
/// whose path ends in `.rs` as one function named `main`, reads any other file as unsupported,
/// and fails on a file whose path contains `broken`.
#[derive(Debug, Default)]
pub struct LanguageFake {
    /// Which build of the language the fake stands for.
    pub build: String,
    /// What the catalog answers: every relation, none, or a failure.
    pub catalog: FakeCatalog,
    /// Whether the node of the first row is a place past the file's table: a language that writes what its file cannot hold.
    pub out_of_range: bool,
    /// How many rows beyond the first each file read gives: files with many rows.
    pub extra_rows: u32,
    /// Whether the one language the fake stands for reads the relations, and when it does not, why.
    pub absent: Absence,
    /// Whether every function is named `main`, whatever the file: the names do not grow with the files.
    pub constant_name: bool,
    /// Whether a function is named by its file's text up to the first `(`, so an edit of the text can rename it.
    pub named_by_text: bool,
    /// How many files it was asked to read.
    pub parses: AtomicUsize,
    /// How many times it was asked to read the manifests.
    pub projects: AtomicUsize,
    /// The files it says it read beyond the manifests whenever it reads them, by path from the root.
    pub consulted: Vec<FilePath>,
}

/// What the fake's catalog answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FakeCatalog {
    /// It lists every relation.
    #[default]
    Lists,
    /// It lists no relation.
    Empty,
    /// It fails.
    Fails,
}

/// The ending of the paths the fake reads.
pub const READ: &str = ".rs";

/// The ending of the paths the fake reads as manifests.
pub const MANIFEST: &str = "manifest.fake";

/// The text in the paths the fake fails on.
pub const BROKEN: &str = "broken";

/// The name of the one kind of the fake's tables.
const NODE_KIND: &str = "node";

/// Refuses the manifests when the path of one of them contains the text the fake fails on.
fn refuse_broken(input: &ProjectInput<'_>) -> Result<(), LanguageError> {
    if input
        .manifests
        .iter()
        .any(|manifest| manifest.path.0.contains(BROKEN))
    {
        return Err(LanguageError::Manifest {
            reason: "the fake was told to fail".to_owned(),
        });
    }
    Ok(())
}

impl LanguagePort for LanguageFake {
    fn relations(&self) -> Result<RelationList, LanguageError> {
        match self.catalog {
            FakeCatalog::Fails => Err(LanguageError::Catalog {
                reason: "the fake was told to fail".to_owned(),
            }),
            FakeCatalog::Empty => Ok(RelationList(Vec::new())),
            FakeCatalog::Lists => Ok(RelationList(all().to_vec())),
        }
    }

    fn linked(&self) -> LinkedNames {
        LinkedNames(vec![LinkedName(format!("fake {}", self.build).into())])
    }

    fn unknown_reasons(&self) -> UnknownReasons {
        UnknownReasons(Vec::new())
    }

    fn kind_vocabularies(&self) -> KindVocabularies {
        KindVocabularies(vec![KindVocabulary {
            language: LanguageName("fake".into()),
            kinds: Vec::new(),
        }])
    }

    fn supports(&self, _relation: Relation) -> Answers {
        Answers(vec![RelationAnswer {
            language: LanguageName("fake".into()),
            absence: self.absent,
            unknowns: &[],
        }])
    }

    fn reads_manifest(&self, path: &FilePath) -> ManifestReading {
        if path.0.ends_with(MANIFEST) {
            ManifestReading::Read
        } else {
            ManifestReading::NotRead
        }
    }

    fn parse(
        &self,
        file: FileToRead<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
        out: &mut dyn RowWriter,
    ) -> FileReading {
        let _parses_before = self.parses.fetch_add(1, Ordering::Relaxed);
        if file.path.0.contains(BROKEN) {
            return FileReading::Failed(ReadFailure("".into()));
        }
        if file.path.0.ends_with(MANIFEST) {
            return FileReading::Manifest;
        }
        if !file.path.0.ends_with(READ) {
            return FileReading::Unsupported;
        }
        let end = u32::try_from(file.bytes.0.len()).unwrap_or(u32::MAX);
        // The tree has an element for each row, so that every node the rows name is a place of it.
        let mut builder = table_builder(table, symbols, ByteSlice(&file.bytes.0));
        for _ in 0..=self.extra_rows {
            enter_node(&mut builder, KindBits(0), Offset(0));
            leave_node(&mut builder, Offset(end));
        }
        // A language names every kind its table holds, as the engine reads a kind by its name.
        name_kind(
            &mut completion(table, symbols, ByteSlice(&file.bytes.0), file.file),
            KindBits(0),
            NameText(NODE_KIND),
        );
        let text = String::from_utf8_lossy(&file.bytes.0);
        let name = ryo_language::domain::symbols::service::intern_derived(
            symbols,
            ByteSlice(&file.bytes.0),
            NameText(if self.constant_name {
                "main"
            } else if self.named_by_text {
                text.split('(').next().unwrap_or_default()
            } else {
                &file.path.0
            }),
        );
        let elements = self.extra_rows + 1;
        for step in 0..=self.extra_rows {
            let place = if self.out_of_range && step == 0 {
                elements
            } else {
                step
            };
            out.write(FunctionRow {
                id: ryo_language::domain::relations::service::node_id(file.file, Position(place)),
                name,
                file: file.file,
                span: Span {
                    start: Offset(step.min(end)),
                    end: Offset(end),
                },
            });
        }
        FileReading::Read(LanguageNumber(0))
    }

    fn project(
        &self,
        input: &ProjectInput<'_>,
        _symbols: &mut Symbols,
        _out: &mut dyn RowWriter,
    ) -> Result<ProjectAnswer, LanguageError> {
        let _projects_before = self.projects.fetch_add(1, Ordering::Relaxed);
        refuse_broken(input)?;
        Ok(ProjectAnswer {
            consulted: self.consulted.clone(),
            ..ProjectAnswer::default()
        })
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
