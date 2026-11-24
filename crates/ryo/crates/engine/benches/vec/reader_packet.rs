//! One file-scoped row packet, indexed once for both phases. Resolving a column
//! or translating its symbols into the executor dictionary waits for demand.
use super::super::{finish, reader};
use rustc_hash::FxHashMap;
use ryo::languages::{self as l, ByteSlice, CellKind, CellWord, FileWords, FilledColumns};
use ryo_relations::domain::relations::service as catalog;

pub(crate) struct Packet {
    pub(crate) rows: l::contract::Collect,
    widths: Vec<usize>,
    offsets: Vec<Vec<usize>>,
    requested: Vec<bool>,
    pub(crate) ready: bool,
}
impl Packet {
    pub(crate) fn new() -> Self {
        let count = catalog::ALL.len();
        let mut widths = vec![0; count];
        for &rel in catalog::ALL {
            widths[catalog::number_of(rel).0] = catalog::kinds(rel).len();
        }
        Self {
            rows: Default::default(),
            widths,
            offsets: vec![Vec::new(); count],
            requested: vec![false; count],
            ready: false,
        }
    }
    pub(crate) fn clear(&mut self) {
        self.rows.clear();
        self.offsets.iter_mut().for_each(Vec::clear);
        self.requested.fill(false);
        self.ready = false;
    }
    pub(crate) fn index(&mut self, wanted: &[l::Relation]) {
        for &rel in wanted {
            self.requested[catalog::number_of(rel).0] = true;
        }
        let mut at = 0;
        for &rel in &self.rows.relations {
            let i = catalog::number_of(rel).0;
            debug_assert!(self.requested[i]);
            self.offsets[i].push(at);
            at += self.widths[i];
        }
        debug_assert_eq!(at, self.rows.words.len());
        self.ready = true;
    }
    /// An indexed, requested relation already has an exact row count. The
    /// caller withholds this observation when the provider declared a partial.
    pub(crate) fn has_rows(&self, relation: l::Relation) -> Option<bool> {
        let slot = catalog::number_of(relation).0;
        (self.ready && self.requested[slot]).then(|| !self.offsets[slot].is_empty())
    }
    /// Visit a complete symbol column without interpreting unrelated node IDs.
    pub(crate) fn any_symbol(
        &self, relation: l::Relation, column: usize, mut matches: impl FnMut(l::Symbol) -> bool,
    ) -> Option<bool> {
        self.has_rows(relation)?;
        if catalog::kinds(relation).get(column) != Some(&CellKind::Symbol) {
            return None;
        }
        let starts = &self.offsets[catalog::number_of(relation).0];
        Some(starts.iter().any(|&at| matches(l::Symbol(self.rows.words[at + column] as u32))))
    }
    pub(crate) fn columns(
        &self,
        relation: l::Relation,
        symbols: &l::Symbols,
        bytes: &[u8],
        id: l::FileId,
        n: usize,
        words: &mut FileWords,
        out: &mut FilledColumns,
    ) -> bool {
        if self.has_rows(relation).is_none()
            || self.rows.partials.iter().any(|(r, _, _)| *r == relation)
            || self.rows.partials_at.iter().any(|(r, _, _)| *r == relation)
        {
            return false;
        }
        let starts = &self.offsets[catalog::number_of(relation).0];
        let kinds = catalog::kinds(relation);
        out.0.resize_with(kinds.len(), Default::default);
        for (i, (column, kind)) in out.0.iter_mut().zip(kinds).enumerate() {
            column.0.reserve(starts.len());
            for start in starts {
                let raw = self.rows.words[start + i];
                let cell = match kind {
                    CellKind::Node if raw >> 32 != u64::from(id.0) || raw as u32 as usize >= n => {
                        return false;
                    }
                    CellKind::Node => CellWord(u64::from(raw as u32)),
                    CellKind::Symbol => {
                        let Some(text) =
                            l::text_of(symbols, ByteSlice(bytes), l::Symbol(raw as u32))
                        else {
                            return false;
                        };
                        l::facts::intern(words, text)
                    }
                    _ => CellWord(raw),
                };
                column.0.push(cell);
            }
        }
        true
    }
    pub(crate) fn view<'a, 'words>(
        &'a self,
        parsed: &'a mut reader::ReaderFacts<'words>,
        symbols: &'a l::Symbols,
        bytes: &'a [u8],
    ) -> View<'a, 'words> {
        View {
            packet: self,
            parsed,
            symbols,
            bytes,
            resolved: FxHashMap::default(),
            renderer: None,
        }
    }
    fn partial(&self, rel: l::Relation) -> Option<&str> {
        self.rows
            .partials_at
            .iter()
            .rev()
            .find(|r| r.0 == rel)
            .map(|r| r.2.as_str())
            .or_else(|| {
                self.rows
                    .partials
                    .iter()
                    .rev()
                    .find(|r| r.0 == rel)
                    .map(|r| r.1.as_str())
            })
    }
}
pub(crate) struct View<'a, 'words> {
    pub(crate) renderer: Option<&'a mut dyn FnMut(l::RenderRequest) -> Result<l::RenderOutcome, String>>,
    packet: &'a Packet,
    parsed: &'a mut reader::ReaderFacts<'words>,
    symbols: &'a l::Symbols,
    bytes: &'a [u8],
    resolved: FxHashMap<u32, u32>,
}
impl finish::Provider for View<'_, '_> {
    fn syntax_text(&self) -> Option<&[u32]> {
        self.parsed.projects_words.then_some(self.parsed.text.as_slice())
    }
    fn render(&mut self, node: u32, context: &str) -> Result<Option<Vec<u8>>, String> {
        let role = match context {
            "assignment-target" => l::RenderContext::AssignmentTarget,
            "attribute-receiver" => l::RenderContext::AttributeReceiver,
            "positional-argument" => l::RenderContext::PositionalArgument,
            other => return Err(format!("unknown render context {other}")),
        };
        let render = self.renderer.as_mut().ok_or_else(|| format!("no renderer for {context}"))?;
        let node = l::NodeId((u64::from(self.parsed.file) << 32) | u64::from(node));
        match render(l::RenderRequest { node, context: role })? {
            l::RenderOutcome::Known(text) => Ok(Some(text.0.as_bytes().to_vec())),
            l::RenderOutcome::NotApplicable => Ok(None),
            l::RenderOutcome::Unsupported(reason) => Err(format!("unresolved render in {context}: {}", reason.0)),
        }
    }
    fn fill_len(
        &mut self,
        run: &mut finish::Run,
        name: &str,
        scope: finish::Scope<'_>,
    ) -> Option<usize> {
        if !matches!(scope, finish::Scope::File(f) if f == self.parsed.file) {
            return None;
        }
        let rel = run.catalog(name)?.0?;
        let i = catalog::number_of(rel).0;
        (self.packet.requested[i] && self.packet.partial(rel).is_none())
            .then(|| self.packet.offsets[i].len())
    }
    fn fill(
        &mut self,
        run: &mut finish::Run,
        name: &str,
        scope: finish::Scope<'_>,
    ) -> Result<finish::Table, String> {
        if !matches!(scope, finish::Scope::File(f) if f == self.parsed.file) {
            return Err(format!(
                "relation {name} requested outside this reader's file"
            ));
        }
        let (Some(rel), names, tys) = run
            .catalog(name)
            .ok_or_else(|| format!("undeclared relation {name}"))?
        else {
            return Err(format!("undeclared relation {name}"));
        };
        if let Some(cause) = self.packet.partial(rel) {
            return Err(format!("partial {name}: {cause}"));
        }
        let i = catalog::number_of(rel).0;
        if !self.packet.requested[i] {
            return Err(format!("no reader for {name}"));
        }
        let offsets = &self.packet.offsets[i];
        let width = self.packet.widths[i];
        let mut table = finish::Table::with_rows(names, tys, offsets.len());
        for &at in offsets {
            let row = &self.packet.rows.words[at..at + width];
            for ((column, &raw), ty) in table.cols.iter_mut().zip(row).zip(table.tys.iter()) {
                let value = match ty {
                    finish::Ty::Sym => {
                        let id = raw as u32;
                        u64::from(*self.resolved.entry(id).or_insert_with(|| {
                            reader::symbol(run, self.parsed.unknown, self.symbols, self.bytes, l::Symbol(id))
                        }))
                    }
                    finish::Ty::Node => {
                        assert_eq!(
                            raw >> 32,
                            u64::from(self.parsed.file),
                            "foreign node in file-scoped answer"
                        );
                        u64::from(raw as u32)
                    }
                    _ => raw,
                };
                column.push(value);
            }
        }
        table.n = offsets.len();
        self.parsed.fills_made += 1;
        Ok(table)
    }
}
