//! Adapt the SDK that the product actually linked. Node positions, unknown names,
//! provider notes and relation widths must come from that same provider revision.
use crate::{check_session::source::{Linked, Source}, finish::Run, reader};
use release_relations::domain::{
    labels::types::NameRef as OldName, relations::service as original,
};
use release_sdk::languages as old;
use rustc_hash::FxHashSet;
use ryo::languages as l;
use ryo_execution_domain::domain::coverage::types::Recovery;
use ryo_relations::domain::{labels::types::NameRef, relations::service as schema};
use std::{collections::HashMap, sync::Arc};

type Result<T> = std::result::Result<T, String>;

#[derive(Clone)]
pub(super) struct Pinned {
    language: old::Language,
    parser: Arc<dyn old::contract::ParseEngine>,
    native: Option<Linked>,
}
impl From<old::Language> for Pinned {
    fn from(language: old::Language) -> Self {
        let native = language.native_provider.clone().map(Linked::from);
        Self {
            native,
            parser: language.parse_engine(),
            language,
        }
    }
}
#[derive(Default)]
pub(super) struct Dictionary {
    source: old::Symbols,
    out: l::Symbols,
}

fn relation(r: l::Relation) -> Result<old::Relation> {
    let name = schema::name(r).0;
    original::relation_named(OldName(name))
        .ok_or_else(|| format!("relation {name} is outside the linked catalog"))
}

fn word(raw: old::Symbol, dict: &old::Symbols, bytes: &[u8], out: &mut l::Symbols) -> l::Symbol {
    if let Some(text) = old::text_of(dict, old::ByteSlice(bytes), raw) {
        l::intern_derived(out, l::ByteSlice(bytes), l::NameText(text.0))
    } else {
        l::Symbol(raw.0)
    }
}

fn project_word(
    raw: old::Symbol,
    dict: &old::Symbols,
    bytes: &[u8],
    run: &mut Run,
    unknown: &HashMap<u32, u32>,
) -> u32 {
    if let Some(text) = old::text_of(dict, old::ByteSlice(bytes), raw) {
        return run.sym(text.0.as_bytes());
    }
    unknown
        .get(&raw.0)
        .copied()
        .unwrap_or_else(|| run.absent_word.expect("reader's absent word"))
}

fn kind(k: old::CellKind) -> l::CellKind {
    match k {
        old::CellKind::File => l::CellKind::File,
        old::CellKind::Node => l::CellKind::Node,
        old::CellKind::Span => l::CellKind::Span,
        old::CellKind::Number => l::CellKind::Number,
        old::CellKind::Flag => l::CellKind::Flag,
        old::CellKind::Symbol => l::CellKind::Symbol,
    }
}

fn collect(
    read: old::contract::Collect,
    symbols: &old::Symbols,
    bytes: &[u8],
    target: &mut l::Symbols,
    out: &mut l::contract::Collect,
) -> Result<()> {
    let to_current = |r| {
        schema::relation_named(NameRef(original::name(r).0))
            .ok_or_else(|| format!("unrecognized provider relation {}", original::name(r).0))
    };
    let mut layouts = HashMap::new();
    let mut words = read.words.as_slice();
    let mut remapped = HashMap::new();
    for r in read.relations {
        if !layouts.contains_key(&r) {
            let rel = to_current(r)?;
            let kinds = original::kinds(r);
            if !original::columns(r)
                .into_iter()
                .map(|c| original::column_name(c).0)
                .eq(schema::columns(rel)
                    .into_iter()
                    .map(|c| schema::column_name(c).0))
                || !kinds
                    .iter()
                    .copied()
                    .map(kind)
                    .eq(schema::kinds(rel).iter().copied())
            {
                return Err(format!(
                    "catalog columns changed for {}",
                    original::name(r).0
                ));
            }
            layouts.insert(r, (rel, kinds));
        }
        let &(rel, kinds) = &layouts[&r];
        let (row, rest) = words.split_at(kinds.len());
        words = rest;
        out.relations.push(rel);
        for (&raw, &k) in row.iter().zip(kinds) {
            let v = if k == old::CellKind::Symbol {
                *remapped.entry(raw).or_insert_with(|| {
                    u64::from(word(old::Symbol(raw as u32), symbols, bytes, target).0)
                })
            } else {
                raw
            };
            out.words.push(v);
        }
    }
    if !words.is_empty() {
        return Err("provider supplied an incomplete row".into());
    }
    for (r, why, how) in read.partials {
        use release_relations::domain::relations::types::Missing as M;
        use ryo_relations::domain::relations::types::{Missing, RowKind};
        out.partials.push((
            to_current(r)?,
            why,
            match how {
                M::Any => Missing::Any,
                M::Kind(k) => Missing::Kind(RowKind(k.0)),
            },
        ));
    }
    for (r, f, why) in read.partials_at {
        out.partials_at.push((to_current(r)?, l::FileId(f.0), why));
    }
    Ok(())
}

fn project_request(r: &l::ProjectRequest) -> Result<old::ProjectRequest> {
    use release_language::domain::project_context as ctx;
    if r.given_inventory != Default::default() {
        return Err("unsupported project entry authority".into());
    }
    let entries = r
        .observed_entries
        .0
        .iter()
        .map(|e| ctx::types::EntryObservation {
            path: old::FilePath(e.path.0.clone()),
            state: match e.state {
                l::ObservedEntryState::File => old::ObservedEntryState::File,
                l::ObservedEntryState::Directory => old::ObservedEntryState::Directory,
                l::ObservedEntryState::Missing => old::ObservedEntryState::Missing,
                l::ObservedEntryState::Other => old::ObservedEntryState::Other,
                l::ObservedEntryState::Unknown => old::ObservedEntryState::Unknown,
            },
        })
        .collect();
    Ok(old::ProjectRequest {
        root: old::RootPath(r.root.0.clone()),
        repository: match r.repository {
            l::Repository::Disk => old::Repository::Disk,
            l::Repository::Given => old::Repository::Given,
        },
        search_scope: match &r.search_scope {
            ryo_language::domain::project_context::types::ProjectSearchScope::Undeclared => Default::default(),
            ryo_language::domain::project_context::types::ProjectSearchScope::Declared(roots) =>
                ctx::types::ProjectSearchScope::Declared(roots.iter().map(|r| old::FilePath(r.0.clone())).collect()),
        },
        given_inventory: Default::default(),
        observed_entries: ctx::service::observed(entries),
        files: r.files.iter().map(|p| old::FilePath(p.0.clone())).collect(),
        manifests: r
            .manifests
            .iter()
            .map(|m| old::ManifestFile {
                path: old::FilePath(m.path.0.clone()),
                text: old::ManifestText(m.text.0.clone()),
            })
            .collect(),
        notes: r.notes.iter().map(|b| old::Bytes(b.0.clone())).collect(),
        wanted: r
            .wanted
            .iter()
            .copied()
            .map(relation)
            .collect::<Result<_>>()?,
    })
}

pub(super) enum File {
    Released(old::Elements),
    Native(l::Elements),
}

impl Default for File {
    fn default() -> Self { Self::Released(Default::default()) }
}

impl Source for Pinned {
    fn source_words(&self) -> Option<l::SourceWords> { self.native.as_ref().and_then(Source::source_words) }
    fn uses_project(&self) -> bool { self.native.as_ref().is_some_and(Source::uses_project) }
    fn scoped(&self) -> Option<&dyn l::NumberedVisit> { self.native.as_ref().and_then(Source::scoped) }
    fn declarations(&self) -> &'static [crate::check_session::source::FileRelation] {
        self.native.as_ref().map_or(&[], Source::declarations)
    }
    fn identity(&self) -> (&str, &str) {
        self.native.as_ref().map_or_else(|| (self.language.build().0.as_ref(), self.language.configuration_identity().0), Source::identity)
    }
    fn name(&self) -> &str { &self.language.name().0 }
    fn reads(&self, path: &str) -> bool {
        // The provider that reads a file says which files it reads: the released recogniser is the answer only for a language
        // with no native provider (the released TypeScript recogniser leaves out the JavaScript endings the native one reads).
        if let Some(parser) = &self.native { return parser.reads(path); }
        self.parser.recognition(&old::ParseRequest {
            file: old::FileId(0),
            path: old::SourcePath(path),
            bytes: old::ByteSlice(&[]),
            notes: old::NoteRequest::Unwanted,
            fields: old::FieldRequest::Unwanted,
        }) == old::Recognition::Reads
    }
    fn manifests(&self) -> Vec<&'static str> {
        self.language
            .project()
            .manifest_names()
            .iter()
            .map(|n| n.0)
            .collect()
    }
    fn fields(&self) -> l::FieldNames { self.native.as_ref().map_or_else(|| l::FieldNames(Vec::new()), Linked::fields) }
    fn kinds(&self) -> Vec<&'static str> {
        if let Some(parser) = &self.native { return parser.kinds(); }
        self.parser
            .kind_names()
            .0
            .into_iter()
            .map(|n| n.0)
            .collect()
    }
    fn words(&self) -> Vec<l::UnknownName> {
        if let Some(parser) = &self.native { return parser.words(); }
        self.parser
            .unknown_reasons()
            .iter()
            .map(|r| l::UnknownName {
                reason: l::UnknownReason(r.reason.0),
                text: l::ReasonName(r.text.0),
            })
            .collect()
    }
    // The pinned SDK reads its native catalog from the concrete tree. Its other
    // relations, including expanded semantic nodes, go through the provider.
    fn expands(&self, relation_id: l::Relation) -> bool {
        if let Some(parser) = &self.native { return parser.expands(relation_id); }
        relation(relation_id).ok().and_then(
            release_execution_domain::domain::table_cells::service::native_form,
        ).is_none()
    }
    fn available(&self, r: l::Relation) -> bool {
        if let Some(parser) = &self.native { return parser.available(r); }
        relation(r)
            .ok()
            .is_some_and(|r| self.parser.absence_of(r) != Some(old::AbsenceCause::NotYetRead))
    }
    fn absent(&self, r: l::Relation) -> bool {
        if let Some(parser) = &self.native { return parser.absent(r); }
        relation(r).ok().is_some_and(|r| self.parser.absence_of(r).is_some())
    }
    fn entries(&self, r: &l::ProjectRequest, out: &mut l::ProjectEntries) -> Result<()> {
        let mut entries = old::ProjectEntries::default();
        self.language
            .project()
            .project_entries(&project_request(r)?, &mut entries);
        out.0
            .extend(entries.0.into_iter().map(|p| l::FilePath(p.0)));
        Ok(())
    }
    fn project(
        &self,
        r: &l::ProjectRequest,
        symbols: &mut l::Symbols,
        out: &mut l::contract::Collect,
    ) -> Result<l::Resolved> {
        let req = project_request(r)?;
        let mut d = old::Symbols::default();
        let mut rows = old::contract::Collect::default();
        let reply = self
            .language
            .project()
            .project(&req, &mut d, &mut rows)
            .map_err(|e| e.to_string())?;
        if reply.observed_entries.as_slice() != req.observed_entries.0.as_ref() {
            return Err("project observation receipt differs from its input".into());
        }
        collect(rows, &d, &[], symbols, out)?;
        Ok(l::Resolved {
            observed_entries: r.observed_entries.0.to_vec(),
            refused: reply
                .refused
                .into_iter()
                .map(|f| l::Refused {
                    path: l::FilePath(f.path.0),
                    reason: l::ToolMessage(f.reason.0),
                })
                .collect(),
            consulted: reply
                .consulted
                .into_iter()
                .map(|p| l::FilePath(p.0))
                .collect(),
            unlisted: reply
                .unlisted
                .into_iter()
                .map(|p| l::FilePath(p.0))
                .collect(),
            derived: (),
        })
    }
}
