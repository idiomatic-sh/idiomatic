//! The source adapter keeps the linked provider's native tree and symbol storage.
//! Both walks feed the same numbered node table, catalog rows and executor.
use super::{FileId, Result, Run, l, reader};
use rustc_hash::FxHashSet;
use ryo_execution_domain::domain::coverage::types::Recovery;
use ryo_language::domain::files::types::ProjectEntries;
use std::{collections::HashMap, sync::Arc};

pub(crate) use ryo_language::inbound::use_cases::parse::named::FileRelation;
pub(crate) type Columns = Vec<ryo_relations::domain::relations::types::FactColumn>;

pub(crate) trait Source: Clone + Sync {
    fn source_words(&self) -> Option<l::SourceWords> { None }
    fn uses_project(&self) -> bool { false }
    fn declarations(&self) -> &'static [FileRelation] { &[] }
    fn named_rows(&self, relations: &[&FileRelation], _: l::ParseRequest<'_>, _: &Self::File, _: &l::NodeTable, _: &mut Self::Dictionary, _: Option<&dyn l::FileProject>) -> Vec<Result<l::FileRows>> {
        relations.iter().map(|schema| Err(format!("provider does not supply {}", schema.name))).collect()
    }

    fn scoped(&self) -> Option<&dyn l::NumberedVisit> { None }
    fn numbered_projections(&self) -> &'static [l::Relation] {
        self.scoped().map_or(&[], |visit| visit.projections())
    }


    fn begin_render(&self, _: l::ParseRequest<'_>, _: &mut Self::File) -> l::SemanticSlot {
        l::SemanticSlot::Empty
    }
    fn name(&self) -> &str;
    fn identity(&self) -> (&str, &str);
    type File: Default;
    type Dictionary: Default;
    fn lent(_: &Self::File) -> Option<Arc<l::NodeTable>> { None }
    fn reads(&self, path: &str) -> bool;
    fn manifests(&self) -> Vec<&'static str>;
    fn words(&self) -> Vec<l::UnknownName>;
    fn kinds(&self) -> Vec<&'static str>;
    fn fields(&self) -> l::FieldNames { l::FieldNames(Vec::new()) }
    fn expands(&self, rel: l::Relation) -> bool;
    fn available(&self, rel: l::Relation) -> bool;
    /// Whether the language never answers `rel`, for whatever cause: static, so known before any step runs.
    fn absent(&self, rel: l::Relation) -> bool;
    fn build(
        &self,
        req: l::ParseRequest<'_>,
        file: &mut Self::File,
        dict: &mut Self::Dictionary,
        facts: &[l::Relation],
    ) -> Result<()>;
    fn notes(file: &Self::File) -> l::Bytes;
    fn recovery(file: &Self::File) -> Recovery;
    fn tree(
        file: &Self::File,
        dict: &Self::Dictionary,
        bytes: &[u8],
        run: &mut Run,
        tree: &mut l::NodeTable,
        provider: &mut reader::ReaderFacts<'_>,
    );
    fn expanded_summary(
        file: &Self::File,
        dict: &Self::Dictionary,
        bytes: &[u8],
        run: &mut Run,
        unknown: &HashMap<u32, u32>,
        start: usize,
        kinds: Option<&mut FxHashSet<u32>>,
        fields: Option<&mut FxHashSet<u32>>,
        words: Option<&mut FxHashSet<u32>>,
    );
    fn fill(
        &self,
        file: &mut Self::File,
        dict: &mut Self::Dictionary,
        bytes: &[u8],
        id: FileId,
        wanted: &[l::Relation],
        out: &mut l::contract::Collect,
    ) -> Result<()>;
    fn symbols(dict: &Self::Dictionary) -> &l::Symbols;
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
    fn begin_render(&self, request: l::ParseRequest<'_>, file: &mut Self::File) -> l::SemanticSlot {
        let mut slot = ryo_language::domain::elements::service::take_semantics(file);
        self.parser.begin_semantics(&request, file, &mut slot);
        slot
    }
    fn declarations(&self) -> &'static [FileRelation] {
        self.parser.file_relations().map_or(&[], |supplier| supplier.declarations())
    }
    fn named_rows(&self, relations: &[&FileRelation], request: l::ParseRequest<'_>, file: &Self::File, tree: &l::NodeTable, dict: &mut Self::Dictionary, project: Option<&dyn l::FileProject>) -> Vec<Result<l::FileRows>> {
        match self.parser.file_relations() {
            Some(supplier) => supplier.read_covered(relations, request, file, tree, dict, project).into_iter().map(|answer| answer.map_err(|error| error.to_string())).collect(),
            None => relations.iter().map(|schema| Err(format!("no provider for {}", schema.name))).collect(),
        }
    }
    fn identity(&self) -> (&str, &str) { (self.language.build().0.as_ref(), self.language.configuration_identity().0) }
    fn name(&self) -> &str { &self.language.name().0 }
    type File = l::Elements;
    type Dictionary = l::Symbols;
    fn lent(f: &Self::File) -> Option<Arc<l::NodeTable>> { l::native_table(f).cloned() }
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
    fn build(
        &self,
        req: l::ParseRequest<'_>,
        file: &mut Self::File,
        dict: &mut Self::Dictionary,
        wanted: &[l::Relation],
    ) -> Result<()> {
        self.parser
            .build_native_file(req, file, dict, l::NodeAccess::Query(wanted))
            .map_err(|e| format!("{e:?}"))
    }
    fn notes(file: &Self::File) -> l::Bytes {
        ryo_language::domain::elements::service::notes_of(file).clone()
    }
    fn recovery(file: &Self::File) -> Recovery {
        use ryo_language::domain::{
            diagnostics::types::SyntaxCompleteness::Incomplete, elements::service::*,
        };
        if has_incomplete_syntax(file) != Incomplete {
            return Recovery::Whole;
        }
        if has_incomplete_lexical_stream(file) == Incomplete {
            Recovery::SyntaxErrors
        } else {
            Recovery::LexicallyCompleteSyntaxErrors
        }
    }
    fn tree(
        file: &Self::File,
        dict: &Self::Dictionary,
        bytes: &[u8],
        run: &mut Run,
        tree: &mut l::NodeTable,
        facts: &mut reader::ReaderFacts<'_>,
    ) {
        reader::project_tree(file, dict, bytes, run, tree, facts);
    }
    fn expanded_summary(
        file: &Self::File,
        dict: &Self::Dictionary,
        bytes: &[u8],
        run: &mut Run,
        unknown: &HashMap<u32, u32>,
        start: usize,
        mut kinds: Option<&mut FxHashSet<u32>>,
        mut fields: Option<&mut FxHashSet<u32>>,
        mut words: Option<&mut FxHashSet<u32>>,
    ) {
        for i in start as u32..l::end_of(file).0 {
            let at = l::Position(i);
            if let Some(kinds) = kinds.as_deref_mut() {
                kinds.insert(run.kind_sym[usize::from(l::kind_at(file, at).0)]);
            }
            if let Some(fields) = fields.as_deref_mut() {
                fields.insert(reader::symbol(
                    run,
                    unknown,
                    dict,
                    bytes,
                    l::field_name_at(file, at),
                ));
            }
            if let Some(words) = words.as_deref_mut() {
                words.insert(reader::symbol(
                    run,
                    unknown,
                    dict,
                    bytes,
                    l::text_at(file, at),
                ));
            }
        }
    }
    fn fill(
        &self,
        file: &mut Self::File,
        dict: &mut Self::Dictionary,
        bytes: &[u8],
        id: FileId,
        wanted: &[l::Relation],
        out: &mut l::contract::Collect,
    ) -> Result<()> {
        self.parser
            .walk_file(file, dict, l::ByteSlice(bytes), id, wanted, out);
        Ok(())
    }
    fn symbols(dict: &Self::Dictionary) -> &l::Symbols {
        dict
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
