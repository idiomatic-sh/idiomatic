//! The fake of the `language` port: what the core's tests stand in for it with.

use crate::outbound::language::types::{Answers, LinkedNames, RelationList, UnknownReasons};
use ryo_language::domain::fillers::types::{Fillers as _, WalkVocabulary};
use ryo_values::domain::descriptions::types::{
    FieldWord, FixedName, KindVocabularies, KindVocabulary, KindWord};
use std::sync::atomic::{AtomicUsize, Ordering};

use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::all;
use ryo_relations::domain::relations::types::RowWriter;
use ryo_relations::domain::relations::types::{Relation};

use super::super::errors::LanguageError;
use super::super::interface::LanguagePort;
use super::super::types::{
    Absence, FileReading, FileToRead, LinkedName, ManifestReading, ProjectAnswer, ProjectToRead,
    RelationAnswer};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::descriptions::types::LanguageName;

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
    /// Candidate entry paths requested by the fake, including duplicates for engine acquisition controls.
    pub project_entry_paths: Vec<FilePath>,
    /// Whether the project answer gives back none of the entry observations it was given: a language whose receipt does not match the batch.
    pub forgets_entries: bool,
    /// Paths for which this fake reports rejected syntax; other parsed files are accepted when this or `undetermined_syntax` is
    /// nonempty.
    pub rejected_syntax: Vec<FilePath>,
    /// Paths for which this fake leaves syntax acceptance undetermined; other parsed files are accepted when this or
    /// `rejected_syntax` is nonempty.
    pub undetermined_syntax: Vec<FilePath>,
    /// Whether the one language the fake stands for is linked with a walk, which the new engine scans files through.
    pub walk: FakeWalk,
}

/// The walk the fake's one language is linked with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FakeWalk {
    /// None: the new engine can scan none of its files.
    #[default]
    NotLinked,
    /// The language crate's fixture walk of files of words (`*.walk`), with its kinds.
    Words,
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

/// Refuses the manifests when the path of one of them contains the text the fake fails on.
fn refuse_broken(input: &ProjectToRead<'_>) -> Result<(), LanguageError> {
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

    fn walks(
        &self,
    ) -> Vec<std::sync::Arc<dyn ryo_language::inbound::use_cases::walk::types::WalkEngine>> {
        match self.walk {
            // The fake's one language, linked without its walk.
            FakeWalk::NotLinked => vec![std::sync::Arc::new(
                ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl::new(),
            )],
            FakeWalk::Words => vec![std::sync::Arc::new(
                ryo_language::domain::fillers::fixtures::Walking,
            )],
        }
    }

    fn kind_vocabularies(&self) -> KindVocabularies {
        // The names of the fake's one language are its walk's, as the adapter over linked languages reads them.
        let names = match self.walk {
            FakeWalk::NotLinked => WalkVocabulary::default(),
            FakeWalk::Words => ryo_language::domain::fillers::fixtures::Walking::vocabulary(),
        };
        KindVocabularies(vec![KindVocabulary {
            language: LanguageName("fake".into()),
            kinds: names
                .kinds
                .0
                .iter()
                .map(|name| KindWord(name.0.into()))
                .collect(),
            fields: names
                .fields
                .0
                .iter()
                .map(|name| FieldWord(name.0.into()))
                .collect(),
            fixed: names
                .fixed
                .0
                .iter()
                .map(|name| FixedName(name.0.into()))
                .collect(),
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

    fn project_entries(
        &self,
        _input: &ProjectToRead<'_>,
    ) -> ryo_language::domain::files::types::ProjectEntries {
        self.project_entry_paths.clone().into()
    }

    fn project(
        &self,
        input: &ProjectToRead<'_>,
        _symbols: &mut Symbols,
        _out: &mut dyn RowWriter,
    ) -> Result<ProjectAnswer, LanguageError> {
        let _projects_before = self.projects.fetch_add(1, Ordering::Relaxed);
        refuse_broken(input)?;
        Ok(ProjectAnswer {
            consulted: self.consulted.clone(),
            observed_entries: if self.forgets_entries {
                Vec::new()
            } else {
                ryo_language::domain::project_context::service::receipt(input.observed_entries)
            },
            ..ProjectAnswer::default()
        })
    }
}

