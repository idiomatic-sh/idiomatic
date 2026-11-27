//! A linked language as the executor's `Source`: the engines a language hands out (its parse, its walk and its project) and its identity.
//! There is one of these for every caller: the SDK's linter and the Tastemaker bench package build it from the same getters of a linked
//! language, so no caller has a `Source` of its own.
use super::source::{FileRelation, Source};
use super::{Result, l};
use ryo_language::domain::files::types::ProjectEntries;
use ryo_language::inbound::use_cases::parse::types::ParseEngine;
use ryo_language::inbound::use_cases::project::types::ProjectEngine;
use ryo_language::inbound::use_cases::walk::types::WalkEngine;
use std::sync::Arc;

/// What the engine reads of one linked language.
#[derive(Clone)]
pub struct Linked {
    name: Arc<str>,
    build: Arc<str>,
    parser: Arc<dyn ParseEngine>,
    walker: Arc<dyn WalkEngine>,
    projector: Arc<dyn ProjectEngine>,
}

impl Linked {
    /// The language named `name`, of build `build`, reading files and manifests through the three engines it is linked with.
    #[must_use]
    pub fn new(
        name: &str,
        build: &str,
        parser: Arc<dyn ParseEngine>,
        walker: Arc<dyn WalkEngine>,
        projector: Arc<dyn ProjectEngine>,
    ) -> Self {
        Self { name: name.into(), build: build.into(), parser, walker, projector }
    }
}

impl Source for Linked {
    fn source_words(&self) -> Option<l::SourceWords> { self.parser.source_words() }
    fn uses_project(&self) -> bool { self.parser.file_relations().uses_project() }
    fn scoped(&self) -> &dyn l::NumberedVisit { self.parser.file_relations().numbered() }
    fn declarations(&self) -> &'static [FileRelation] {
        self.parser.file_relations().declarations()
    }
    fn identity(&self) -> (&str, &str) { (&self.build, self.parser.configuration_identity().0) }
    fn name(&self) -> &str { &self.name }
    fn reads(&self, path: &str) -> bool {
        self.parser.recognition(&l::ParseRequest {
            file: l::FileId(0),
            path: l::SourcePath(path),
            bytes: l::ByteSlice(&[]),
            notes: l::NoteRequest::Unwanted,
            fields: l::FieldRequest::Unwanted,
        }) == l::Recognition::Reads
    }
    fn manifests(&self) -> Vec<&'static str> {
        self.projector.manifest_names().iter().map(|m| m.0).collect()
    }
    fn words(&self) -> Vec<l::UnknownName> {
        self.parser.unknown_reasons().to_vec()
    }
    fn fields(&self) -> l::FieldNames {
        self.scoped().fields().cloned().unwrap_or_else(|| self.parser.stable_fields())
    }
    fn kinds(&self) -> Vec<&'static str> {
        self.parser.kind_names().0.into_iter().map(|n| n.0).collect()
    }
    fn expands(&self, rel: l::Relation) -> bool {
        self.walker.includes_expansions(rel)
    }
    fn available(&self, rel: l::Relation) -> bool {
        self.walker.absence_of(rel) != Some(l::AbsenceCause::NotYetRead)
    }
    fn absent(&self, rel: l::Relation) -> bool {
        self.walker.absence_of(rel).is_some()
    }
    fn entries(&self, req: &l::ProjectRequest, out: &mut ProjectEntries) -> Result<()> {
        self.projector.project_entries(req, out);
        Ok(())
    }
    fn file_entries(&self, req: &l::ProjectRequest, file: usize, notes: &l::Bytes, out: &mut ProjectEntries) -> Result<()> {
        self.projector.file_entries(req, file, notes, out);
        Ok(())
    }
    fn file_rows(
        &self,
        req: &l::ProjectRequest,
        file: usize,
        notes: &l::Bytes,
        observed: &l::ObservedEntries,
        symbols: &mut l::Symbols,
        rows: &mut crate::languages::contract::Collect,
    ) -> Result<Option<l::Resolved>> {
        match self.projector.file_rows(req, file, notes, observed, symbols, rows) {
            Ok(reply) if reply.observed_entries.as_slice() != observed.0.as_ref() => {
                Err("project observation receipt differs from its input".into())
            }
            Ok(reply) => Ok(Some(reply)),
            Err(ryo_language::inbound::use_cases::project::errors::ProjectError::Unsupported) => Ok(None),
            Err(error) => Err(error.to_string().into()),
        }
    }
    fn project(
        &self,
        req: &l::ProjectRequest,
        symbols: &mut l::Symbols,
        rows: &mut crate::languages::contract::Collect,
    ) -> Result<l::Resolved> {
        let reply = self.projector.project(req, symbols, rows).map_err(|e| e.to_string())?;
        if reply.observed_entries.as_slice() != req.observed_entries.0.as_ref() {
            return Err("project observation receipt differs from its input".into());
        }
        Ok(reply)
    }
}

/// The numbered visit of the language that reads nothing: it lends no file.
#[cfg(any(test, feature = "fixtures"))]
static NOTHING: ryo_language::inbound::use_cases::parse::use_case::ParseUseCaseImpl = ryo_language::inbound::use_cases::parse::use_case::ParseUseCaseImpl;

/// A language that reads files by extension and either uses the project or does not, for the cases of an admission.
#[cfg(any(test, feature = "fixtures"))]
#[derive(Clone)]
pub struct Fake {
    /// The language's name.
    pub name: &'static str,
    /// The extension of the files it reads.
    pub extension: &'static str,
    /// Whether it uses the project.
    pub project: bool,
}

#[cfg(any(test, feature = "fixtures"))]
impl Source for Fake {
    fn scoped(&self) -> &dyn l::NumberedVisit { &NOTHING }
    fn uses_project(&self) -> bool { self.project }
    fn name(&self) -> &str { self.name }
    fn identity(&self) -> (&str, &str) { (self.name, "") }
    fn reads(&self, path: &str) -> bool { path.ends_with(self.extension) }
    fn manifests(&self) -> Vec<&'static str> { Vec::new() }
    fn words(&self) -> Vec<l::UnknownName> { Vec::new() }
    fn kinds(&self) -> Vec<&'static str> { Vec::new() }
    fn expands(&self, _: l::Relation) -> bool { false }
    fn available(&self, _: l::Relation) -> bool { true }
    fn absent(&self, _: l::Relation) -> bool { false }
    fn entries(&self, _: &l::ProjectRequest, _: &mut ProjectEntries) -> Result<()> { Ok(()) }
    fn project(&self, _: &l::ProjectRequest, _: &mut l::Symbols, _: &mut crate::languages::contract::Collect) -> Result<l::Resolved> { unreachable!() }
}
