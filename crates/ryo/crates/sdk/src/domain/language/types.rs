//! The types of the `language` concept: the opaque handle a linked language is, and the languages a program linked.

use std::collections::BTreeMap;
use std::sync::Arc;

use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::parse::types::{
    FieldRequest, NoteRequest, ParseConfigurationIdentity, ParseEngine, ParseRequest, Recognition,
    SkippedDirectory,
};
use ryo_language::inbound::use_cases::project::types::ProjectEngine;
use ryo_relations::domain::relations::types::FileId;

/// What a language is called: the name of its directory.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageName(pub Box<str>);

/// Which build of a language it is: its crate's version.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageBuild(pub Box<str>);

/// A language as a program links it: its name, its build and the two use cases it implements. Made only by [`link!`](crate::link).
#[derive(Clone)]
pub struct Language {
    /// What the language is called, which a run reports its findings under.
    name: LanguageName,
    /// Which build of the language this is, which what a run kept is tied to.
    build: LanguageBuild,
    /// The use case that reads one file of the language.
    engine: Arc<dyn ParseEngine>,
    /// The use case that reads the repository's manifests for the language.
    project: Arc<dyn ProjectEngine>,
}

impl std::fmt::Debug for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(std::any::type_name::<Self>())
            .field(&self.name)
            .field(&self.build)
            .finish_non_exhaustive()
    }
}

impl Language {
    /// A language of these parts: what [`link`](crate::languages::link) builds for [`link!`](crate::link), and nothing else.
    pub(crate) const fn from_parts(
        name: LanguageName,
        build: LanguageBuild,
        engine: Arc<dyn ParseEngine>,
        project: Arc<dyn ProjectEngine>,
    ) -> Self {
        Self {
            name,
            build,
            engine,
            project,
        }
    }

    /// What the language is called: the name of its directory.
    #[must_use]
    pub fn name(&self) -> &LanguageName {
        &self.name
    }

    /// The provider's pure source-versus-project ownership decision for a primary path.
    #[must_use]
    pub fn primary_role(
        &self,
        path: SourcePath<'_>,
    ) -> ryo_language::domain::files::types::PrimaryFileRole {
        self.project.primary_role(path)
    }

    /// Which build of the language this is: what a run kept of its files is used only for the same build.
    #[must_use]
    pub fn build(&self) -> &LanguageBuild {
        &self.build
    }

    /// The canonical parser settings that affect this language's trees and diagnostics.
    #[must_use]
    pub fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        self.engine.configuration_identity()
    }

    /// The names of the directories the language never reads, at any depth: the ecosystem's dependency, environment and build
    /// directories (`node_modules`, `target`): what an edge hands the walks.
    #[must_use]
    pub fn skips(&self) -> &'static [SkippedDirectory] {
        self.engine.skips()
    }

    /// The use case that reads one file: what the SDK's edges hand the engine, and nothing outside the SDK reaches.
    #[must_use]
    pub(crate) fn parse_use_case(&self) -> Arc<dyn ParseEngine> {
        Arc::clone(&self.engine)
    }

    /// The use case that reads the repository's manifests: what the SDK's edges hand the engine.
    #[must_use]
    pub(crate) fn project_use_case(&self) -> Arc<dyn ProjectEngine> {
        Arc::clone(&self.project)
    }

    /// Reads one file through the interface the engine calls: what a language's own tests and benches drive its parse through. Only
    /// with the `fixtures` feature, which only tests switch on: an edge hands a language to the engine through the SDK's edges.
    #[cfg(feature = "fixtures")]
    #[must_use]
    pub fn parse_engine(&self) -> Arc<dyn ParseEngine> {
        self.parse_use_case()
    }

    /// Reads the repository's manifests: what a language's own tests drive its project through. Only with the `fixtures` feature.
    #[cfg(feature = "fixtures")]
    #[must_use]
    pub fn project(&self) -> Arc<dyn ProjectEngine> {
        self.project_use_case()
    }
}

/// The languages a program linked, which read the repository's files; with none, every file is reported unread.
#[derive(Clone, Debug, Default)]
pub struct Languages(pub Vec<Language>);

impl Languages {
    /// The directory names the linked languages never read, each with the languages that declare it, in name order: what a walk
    /// of the repository skips at any depth, beside what `.gitignore` excludes: what an edge hands the walks.
    #[doc(hidden)]
    #[must_use]
    pub fn skipped(&self) -> BTreeMap<SkippedDirectory, Vec<&LanguageName>> {
        let mut skipped: BTreeMap<SkippedDirectory, Vec<&LanguageName>> = BTreeMap::new();
        for language in &self.0 {
            for directory in language.skips() {
                skipped.entry(*directory).or_default().push(language.name());
            }
        }
        skipped
    }

    /// Whether a linked language reads a file at `path`, by its path alone: `Reads` when one does, `Declines` when none does. What
    /// an edge asks before it names an extension for files it will write, such as the scenario files of a definition it creates.
    #[doc(hidden)]
    #[must_use]
    pub fn reads(&self, path: &crate::domain::file::types::RelativePath) -> Recognition {
        let request = ParseRequest {
            path: SourcePath(&path.0),
            file: FileId(0),
            bytes: ByteSlice(&[]),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        };
        if self
            .0
            .iter()
            .any(|language| language.engine.recognition(&request) == Recognition::Reads)
        {
            Recognition::Reads
        } else {
            Recognition::Declines
        }
    }
}
