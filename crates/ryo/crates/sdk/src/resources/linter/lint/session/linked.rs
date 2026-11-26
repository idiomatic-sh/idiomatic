//! A linked language as the executor's `Source`: the SDK's `Language` and the parse engine it hands out. This is the part of the
//! source adapter that names an SDK type, so it lives here, above the executor.
use crate::resources::languages as l;
use ryo_executor::check_session::source::{FileRelation, Source};
use ryo_executor::check_session::Result;
use ryo_language::domain::files::types::ProjectEntries;
use std::sync::Arc;

#[derive(Clone)]
pub struct Linked {
    language: l::Language,
    parser: Arc<dyn ryo_executor::languages::contract::ParseEngine>,
}
impl From<l::Language> for Linked {
    fn from(language: l::Language) -> Self {
        Self {
            parser: language.parse_engine(),
            language,
        }
    }
}
impl Source for Linked {
    fn source_words(&self) -> Option<l::SourceWords> { self.parser.source_words() }
    fn uses_project(&self) -> bool { self.parser.file_relations().is_some_and(|p| p.uses_project()) }
    fn scoped(&self) -> Option<&dyn l::NumberedVisit> { self.parser.file_relations().and_then(|reader| reader.numbered()) }
    fn declarations(&self) -> &'static [FileRelation] {
        self.parser.file_relations().map_or(&[], |supplier| supplier.declarations())
    }
    fn identity(&self) -> (&str, &str) { (self.language.build().0.as_ref(), self.language.configuration_identity().0) }
    fn name(&self) -> &str { &self.language.name().0 }
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
        self.language
            .project()
            .manifest_names()
            .iter()
            .map(|m| m.0)
            .collect()
    }
    fn words(&self) -> Vec<l::UnknownName> {
        self.parser.unknown_reasons().to_vec()
    }
    fn fields(&self) -> l::FieldNames {
        self.scoped().and_then(l::NumberedVisit::fields).cloned().unwrap_or_else(|| self.parser.stable_fields())
    }
    fn kinds(&self) -> Vec<&'static str> {
        self.parser
            .kind_names()
            .0
            .into_iter()
            .map(|n| n.0)
            .collect()
    }
    fn expands(&self, rel: l::Relation) -> bool {
        self.parser.includes_expansions(rel)
    }
    fn available(&self, rel: l::Relation) -> bool {
        self.parser.absence_of(rel) != Some(l::AbsenceCause::NotYetRead)
    }
    fn absent(&self, rel: l::Relation) -> bool {
        self.parser.absence_of(rel).is_some()
    }
    fn entries(&self, req: &l::ProjectRequest, out: &mut ProjectEntries) -> Result<()> {
        self.language.project().project_entries(req, out);
        Ok(())
    }
    fn project(
        &self,
        req: &l::ProjectRequest,
        symbols: &mut l::Symbols,
        rows: &mut l::contract::Collect,
    ) -> Result<l::Resolved> {
        let reply = self
            .language
            .project()
            .project(req, symbols, rows)
            .map_err(|e| e.to_string())?;
        if reply.observed_entries.as_slice() != req.observed_entries.0.as_ref() {
            return Err("project observation receipt differs from its input".into());
        }
        Ok(reply)
    }
}


/// A language that reads files by extension and either uses the project or does not, for the cases of the admission.
#[cfg(test)]
#[derive(Clone)]
pub(crate) struct Fake {
    pub(crate) name: &'static str,
    pub(crate) extension: &'static str,
    pub(crate) project: bool,
}

#[cfg(test)]
impl Source for Fake {
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
    fn project(&self, _: &l::ProjectRequest, _: &mut l::Symbols, _: &mut ryo_executor::languages::contract::Collect) -> Result<l::Resolved> { unreachable!() }
}
