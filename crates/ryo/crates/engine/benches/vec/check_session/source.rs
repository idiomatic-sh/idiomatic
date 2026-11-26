//! The source adapter keeps the linked provider's native tree and symbol storage.
//! Both walks feed the same numbered node table, catalog rows and executor.
use super::{Result, l};
use ryo_language::domain::files::types::ProjectEntries;
use std::sync::Arc;

pub(crate) use ryo_language::inbound::use_cases::parse::named::FileRelation;
pub(crate) type Columns = Vec<ryo_relations::domain::relations::types::FactColumn>;

pub(crate) trait Source: Clone + Sync {
    fn source_words(&self) -> Option<l::SourceWords> { None }
    fn uses_project(&self) -> bool { false }
    fn declarations(&self) -> &'static [FileRelation] { &[] }

    fn scoped(&self) -> Option<&dyn l::NumberedVisit> { None }
    fn numbered_projections(&self) -> &'static [l::Relation] {
        self.scoped().map_or(&[], |visit| visit.projections())
    }


    fn name(&self) -> &str;
    fn identity(&self) -> (&str, &str);
    fn reads(&self, path: &str) -> bool;
    fn manifests(&self) -> Vec<&'static str>;
    fn words(&self) -> Vec<l::UnknownName>;
    fn kinds(&self) -> Vec<&'static str>;
    fn fields(&self) -> l::FieldNames { l::FieldNames(Vec::new()) }
    fn expands(&self, rel: l::Relation) -> bool;
    fn available(&self, rel: l::Relation) -> bool;
    /// Whether the language never answers `rel`, for whatever cause: static, so known before any step runs.
    fn absent(&self, rel: l::Relation) -> bool;
    fn entries(&self, req: &l::ProjectRequest, out: &mut ProjectEntries) -> Result<()>;
    fn project(
        &self,
        req: &l::ProjectRequest,
        symbols: &mut l::Symbols,
        rows: &mut l::contract::Collect,
    ) -> Result<l::Resolved>;
}

#[derive(Clone)]
pub(crate) struct Linked {
    language: l::Language,
    parser: Arc<dyn l::contract::ParseEngine>,
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
