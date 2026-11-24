//! The source adapter keeps the linked provider's native tree and symbol storage.
//! Both walks feed the same numbered node table, catalog rows and executor.
use super::{FileId, Result, Run, l, reader};
use rustc_hash::FxHashSet;
use ryo_execution_domain::domain::coverage::types::Recovery;
use ryo_language::domain::files::types::ProjectEntries;
use std::{collections::HashMap, sync::Arc};

pub(crate) trait Source: Clone {
    type File: Default;
    type Dictionary: Default;
    fn reads(&self, path: &str) -> bool;
    fn manifests(&self) -> Vec<&'static str>;
    fn words(&self) -> Vec<l::UnknownName>;
    fn kinds(&self) -> Vec<&'static str>;
    fn expands(&self, rel: l::Relation) -> bool;
    fn available(&self, rel: l::Relation) -> bool;
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
        kinds: &mut FxHashSet<u32>,
        fields: &mut FxHashSet<u32>,
        words: &mut FxHashSet<u32>,
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
    type File = l::Elements;
    type Dictionary = l::Symbols;
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
    fn build(
        &self,
        req: l::ParseRequest<'_>,
        file: &mut Self::File,
        dict: &mut Self::Dictionary,
        wanted: &[l::Relation],
    ) -> Result<()> {
        self.parser
            .build_file(req, file, dict, wanted)
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
        kinds: &mut FxHashSet<u32>,
        fields: &mut FxHashSet<u32>,
        words: &mut FxHashSet<u32>,
    ) {
        for i in start as u32..l::end_of(file).0 {
            let at = l::Position(i);
            kinds.insert(run.kind_sym[usize::from(l::kind_at(file, at).0)]);
            fields.insert(reader::symbol(
                run,
                unknown,
                dict,
                bytes,
                l::field_name_at(file, at),
            ));
            words.insert(reader::symbol(
                run,
                unknown,
                dict,
                bytes,
                l::text_at(file, at),
            ));
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
