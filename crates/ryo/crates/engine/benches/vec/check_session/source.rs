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

/// Only a file of a language that uses the project takes the project environment; every other file keeps the
/// shared overlay. A request that names a reader gives the environment to that reader's files, as before, which also covers a
/// file read only because the request names it. The session routes a file by `reads` here and the scheduler by the same
/// recognizers, so the two are kept together: a change to how a language recognizes a path changes both.
pub(crate) fn takes_environment<L: Source>(languages: &[L], reader: Option<&str>, path: &str) -> bool {
    languages.iter().any(|language| language.uses_project() && reader.map_or_else(|| language.reads(path), |name| name == language.name()))
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

#[cfg(test)]
mod fake {
    use super::{FileId, Result, Run, Source, l, reader, ProjectEntries, Recovery};
    /// A language that reads files by extension and either uses the project or does not.
    #[derive(Clone)]
    pub(crate) struct Fake { pub(crate) name: &'static str, pub(crate) extension: &'static str, pub(crate) project: bool }

    impl Source for Fake {
        fn uses_project(&self) -> bool { self.project }
        fn name(&self) -> &str { self.name }
        fn identity(&self) -> (&str, &str) { (self.name, "") }
        type File = ();
        type Dictionary = ();
        fn reads(&self, path: &str) -> bool { path.ends_with(self.extension) }
        fn manifests(&self) -> Vec<&'static str> { Vec::new() }
        fn words(&self) -> Vec<l::UnknownName> { Vec::new() }
        fn kinds(&self) -> Vec<&'static str> { Vec::new() }
        fn expands(&self, _: l::Relation) -> bool { false }
        fn available(&self, _: l::Relation) -> bool { true }
        fn absent(&self, _: l::Relation) -> bool { false }
        fn build(&self, _: l::ParseRequest<'_>, _: &mut (), _: &mut (), _: &[l::Relation]) -> Result<()> { Ok(()) }
        fn notes(_: &()) -> l::Bytes { unreachable!() }
        fn recovery(_: &()) -> Recovery { Recovery::Whole }
        fn tree(_: &(), _: &(), _: &[u8], _: &mut Run, _: &mut l::NodeTable, _: &mut reader::ReaderFacts<'_>) {}
        fn expanded_summary(_: &(), _: &(), _: &[u8], _: &mut Run, _: &std::collections::HashMap<u32, u32>, _: usize,
            _: Option<&mut rustc_hash::FxHashSet<u32>>, _: Option<&mut rustc_hash::FxHashSet<u32>>, _: Option<&mut rustc_hash::FxHashSet<u32>>) {}
        fn fill(&self, _: &mut (), _: &mut (), _: &[u8], _: FileId, _: &[l::Relation], _: &mut l::contract::Collect) -> Result<()> { Ok(()) }
        fn symbols(_: &()) -> &l::Symbols { unreachable!() }
        fn entries(&self, _: &l::ProjectRequest, _: &mut ProjectEntries) -> Result<()> { Ok(()) }
        fn project(&self, _: &l::ProjectRequest, _: &mut l::Symbols, _: &mut l::contract::Collect) -> Result<l::Resolved> { unreachable!() }
    }

}
#[cfg(test)]
pub(crate) use fake::Fake;

#[cfg(test)]
mod environment_scope_tests {
    use super::{Fake, takes_environment};

    const FILES: [&str; 6] = ["a.pyx", "b.pyx", "c.pyx", "x.rs", "y.rs", "z.md"];

    fn taking(languages: &[Fake], reader: Option<&str>) -> Vec<&'static str> {
        FILES.into_iter().filter(|path| takes_environment(languages, reader, path)).collect()
    }

    #[test]
    fn a_batch_with_one_project_language_gives_the_environment_to_its_files_only() {
        let languages = [
            Fake { name: "project", extension: ".pyx", project: true },
            Fake { name: "plain", extension: ".rs", project: false },
        ];
        let files = taking(&languages, None);
        assert_eq!(files.len(), 3, "the environment is built for the three files of the project language");
        assert_eq!(files, ["a.pyx", "b.pyx", "c.pyx"]);
    }

    #[test]
    fn no_language_that_uses_the_project_means_no_environment() {
        let languages = [Fake { name: "plain", extension: ".rs", project: false }];
        assert!(taking(&languages, None).is_empty());
    }

    #[test]
    fn a_request_that_names_a_reader_keeps_the_environment_for_every_file() {
        let languages = [
            Fake { name: "project", extension: ".pyx", project: true },
            Fake { name: "plain", extension: ".rs", project: false },
        ];
        assert_eq!(taking(&languages, Some("project")).len(), FILES.len());
        assert!(taking(&languages, Some("plain")).is_empty());
    }
}
