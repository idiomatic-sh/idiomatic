//! One file-scoped row packet, indexed once for both phases. Resolving a column
//! or translating its symbols into the executor dictionary waits for demand.
use super::super::{finish, reader};
use rustc_hash::FxHashMap;
use ryo::languages::{self as l, ByteSlice, CellKind, CellWord, FileWords, FilledColumns};
use ryo_relations::domain::relations::service as catalog;
use std::rc::Rc;

const RELATIONS: usize = catalog::ALL.len();

/// A file's typed fact columns and the extent of the fill that produced them.
pub(crate) struct LocalTable {
    pub(crate) rows: usize,
    pub(crate) columns: FilledColumns,
    pub(crate) answer: l::FactAnswer,
    pub(crate) wanted: l::Wanted,
}

/// Borrow the file's own IDs and buffers until the consumer finishes its visit.
pub(crate) struct FileColumns<'f> {
    pub(crate) home: l::FileId,
    pub(crate) tree: &'f l::NodeTable,
    pub(crate) words: &'f FileWords,
    pub(crate) tables: &'f [Option<LocalTable>],
}

#[derive(Debug)]
pub(crate) struct BadNode;

// Relations may carry file-owned diagnostic identities beyond the syntax tree.
fn file_node(home: u32, raw: u64) -> Result<u64, BadNode> {
    if raw == finish::NULL {
        Ok(raw)
    } else if raw >> 32 == u64::from(home) {
        Ok(u64::from(raw as u32))
    } else {
        Err(BadNode)
    }
}

impl FileColumns<'_> {
    pub(crate) fn column(&self, relation: usize, column: usize) -> Option<&[CellWord]> {
        let table = self.tables.get(relation)?.as_ref()?;
        table.columns.0.get(column)?.0.get(..table.rows)
    }

    pub(crate) fn coverage(&self, relation: usize) -> Option<(&l::FactAnswer, l::Wanted)> {
        let table = self.tables.get(relation)?.as_ref()?;
        Some((&table.answer, table.wanted))
    }

    pub(crate) fn text(&self, word: CellWord) -> Option<l::NameText<'_>> {
        local_word_text(self.words, word)
    }

    pub(crate) fn position(&self, word: CellWord) -> Result<Option<l::Position>, BadNode> {
        if word.0 == finish::NULL {
            return Ok(None);
        }
        let position = word.0 as u32;
        if word.0 >> 32 != u64::from(self.home.0)
            || position as usize >= l::node_table::element_count(self.tree).0
        {
            return Err(BadNode);
        }
        Ok(Some(l::Position(position)))
    }
}

fn local_word_text(words: &FileWords, word: CellWord) -> Option<l::NameText<'_>> {
    u32::try_from(word.0).ok().and_then(|code| {
        ryo_relations::domain::terms::service::term_text(l::Symbol(code))
    }).or_else(|| (word < l::facts::words_end(words)).then(|| l::facts::text(words, word)))
}

pub(crate) struct OperatorColumns<'file> {
    file: FileColumns<'file>,
    symbols: std::cell::OnceCell<Vec<u32>>,
    public: Rc<FxHashMap<u32,u32>>,
    absent: u32,
}

impl finish::fact_view::FileFacts for OperatorColumns<'_> {
    fn bind(&self, relation: l::Relation, run: &mut finish::Run) -> Result<Option<finish::fact_view::Fact<'_>>, String> {
        let slot = catalog::number_of(relation).0;
        let Some(table) = self.file.tables.get(slot).and_then(Option::as_ref) else {
            return Ok(None);
        };
        if !matches!(table.answer, l::FactAnswer::Answered(_)) || table.wanted != l::Wanted::Whole {
            return Ok(None);
        }
        for (column, kind) in catalog::kinds(relation).iter().enumerate() {
            if *kind == CellKind::Symbol {
                self.symbols.get_or_init(|| (0..l::facts::words_end(self.file.words).0)
                    .map(|id| run.sym(l::facts::text(self.file.words, CellWord(id)).0.as_bytes())).collect());
            }
            if *kind == CellKind::Node && self.file.column(slot, column).unwrap_or_default().iter()
                .any(|&value| file_node(self.file.home.0, value.0).is_err()) {
                return Err(format!("node outside this file in {}", catalog::name(relation).0));
            }
        }
        let kinds = catalog::kinds(relation);
        Ok(Some(finish::fact_view::Fact::Lent {
            rows: table.rows, kinds,
            columns: finish::fact_view::Columns::Lent(&table.columns.0, kinds, self.symbols.get().map_or(&[], Vec::as_slice), &self.public, self.absent),
        }))
    }
}

fn widths() -> &'static [usize; RELATIONS] {
    static WIDTHS: std::sync::OnceLock<[usize; RELATIONS]> = std::sync::OnceLock::new();
    WIDTHS.get_or_init(|| {
        let mut widths = [0; RELATIONS];
        for &rel in catalog::ALL {
            widths[catalog::number_of(rel).0] = catalog::kinds(rel).len();
        }
        widths
    })
}

pub(crate) struct Packet {
    pub(crate) rows: l::contract::Collect,
    offsets: Vec<Vec<usize>>,
    local: Vec<Option<LocalTable>>,
    spare: FxHashMap<usize, FilledColumns>,
    requested: Vec<bool>,
    indexed: Vec<usize>,
    pub(crate) ready: bool,
}
impl Packet {
    pub(crate) fn new() -> Self {
        Self {
            rows: Default::default(),
            offsets: vec![Vec::new(); RELATIONS],
            local: Vec::new(),
            spare: Default::default(),
            requested: vec![false; RELATIONS],
            indexed: Vec::new(),
            ready: false,
        }
    }
    pub(crate) fn clear(&mut self) {
        self.rows.clear();
        for slot in self.indexed.drain(..) {
            self.offsets[slot].clear();
            if let Some(table) = self.local.get_mut(slot).and_then(Option::take) {
                self.spare.insert(slot, table.columns);
            }
            self.requested[slot] = false;
        }
        self.ready = false;
    }
    /// The structural projection has its own provider contract. Every filled
    /// catalog relation is published directly in columns, once for this file.
    pub(crate) fn fill_numbered(
        &mut self, supplier: &mut dyn l::NumberedFacts, relations: &[l::Relation],
        symbols: &mut l::Symbols, words: &mut FileWords,
    ) -> Result<(), String> {
        use ryo_relations::domain::{catalog::service::fact_form, relations::types::FactForm};
        let mut structural = Vec::new();
        self.local.resize_with(RELATIONS, || None);
        for &relation in relations {
            if fact_form(relation) == FactForm::NodeTable {
                structural.push(relation);
                continue;
            }
            let slot = catalog::number_of(relation).0;
            let mut columns = self.spare.remove(&slot).unwrap_or_default();
            for column in &mut columns.0 { column.0.clear(); }
            let wanted = l::Wanted::Whole;
            let answer = supplier.fill_columns(relation, wanted, words, &mut columns);
            match &answer {
                l::FactAnswer::Partial(part) => self.rows.partials.push((relation, part.cause.0.to_string(), part.missing)),
                l::FactAnswer::Refused(l::AbsenceCause::NotYetRead) => {
                    self.rows.partials.push((relation, "catalog relation has no numbered filler".into(), l::Missing::Any));
                }
                _ => {}
            }
            let rows = columns.0.first().map_or(0, |column| column.0.len());
            if columns.0.iter().any(|column| column.0.len() != rows) {
                return Err(format!("unequal column lengths in {}", catalog::name(relation).0));
            }
            self.local[slot] = Some(LocalTable { rows, columns, answer, wanted });
        }
        if !structural.is_empty() {
            supplier.project_structure(&structural, symbols, &mut self.rows).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub(crate) fn operator_columns<'a>(
        &'a self, file: l::FileId, tree: &'a l::NodeTable, words: &'a FileWords, run: &mut finish::Run,
    ) -> OperatorColumns<'a> {
        let file = FileColumns { home: file, tree, words, tables: &self.local };
        OperatorColumns { file, symbols: Default::default(), public: run.public_word_ids.clone(), absent: run.absent_word.expect("reader dictionary") }
    }
    fn local(&self, slot: usize) -> Option<&LocalTable> {
        self.local.get(slot)?.as_ref()
    }
    fn count(&self, slot: usize) -> usize {
        self.local(slot).map_or_else(|| self.offsets[slot].len(), |t| t.rows)
    }
    pub(crate) fn index(&mut self, wanted: &[l::Relation]) {
        debug_assert!(!self.ready);
        for &rel in wanted {
            let slot = catalog::number_of(rel).0;
            if !self.requested[slot] {
                self.requested[slot] = true;
                self.indexed.push(slot);
            }
        }
        let widths = widths();
        let mut at = 0;
        for &rel in &self.rows.relations {
            let i = catalog::number_of(rel).0;
            debug_assert!(self.requested[i]);
            self.offsets[i].push(at);
            at += widths[i];
        }
        debug_assert_eq!(at, self.rows.words.len());
        self.ready = true;
    }
    /// An indexed, requested relation already has an exact row count. The
    /// caller withholds this observation when the provider declared a partial.
    pub(crate) fn has_rows(&self, relation: l::Relation) -> Option<bool> {
        let slot = catalog::number_of(relation).0;
        (self.ready && self.requested[slot]).then(|| self.count(slot) != 0)
    }
    /// A span may be absent. Every supplied interval must otherwise be an
    /// ordered slice of the admitted bytes.
    pub(crate) fn bounded_spans(&self, relation: l::Relation, column: usize, len: usize) -> bool {
        if self.has_rows(relation).is_none() || catalog::kinds(relation).get(column) != Some(&CellKind::Span) {
            return false;
        }
        let valid = |span: u64| span == finish::NULL || (span >> 32 <= u64::from(span as u32) && span as u32 as usize <= len);
        let slot = catalog::number_of(relation).0;
        if let Some(t) = self.local(slot) {
            return t.rows == 0 || t.columns.0[column].0.iter().all(|w| valid(w.0));
        }
        self.offsets[slot].iter().all(|&row| valid(self.rows.words[row + column]))
    }
    /// Visit a complete symbol column without interpreting unrelated node IDs.
    pub(crate) fn any_symbol(
        &self, relation: l::Relation, column: usize, symbols: &l::Symbols, bytes: &[u8],
        local_words: Option<&FileWords>, mut matches: impl FnMut(Option<l::NameText<'_>>, l::Symbol) -> bool,
    ) -> Option<bool> {
        self.has_rows(relation)?;
        if catalog::kinds(relation).get(column) != Some(&CellKind::Symbol) { return None; }
        let slot = catalog::number_of(relation).0;
        if let Some(table) = self.local(slot) {
            let words = local_words?;
            return Some(table.rows != 0 && table.columns.0[column].0.iter().any(|&raw| {
                matches(local_word_text(words, raw), l::Symbol(raw.0 as u32))
            }));
        }
        Some(self.offsets[slot].iter().any(|&at| {
            let word = l::Symbol(self.rows.words[at + column] as u32);
            matches(l::text_of(symbols, ByteSlice(bytes), word), word)
        }))
    }
    pub(crate) fn columns(
        &self,
        relation: l::Relation,
        symbols: &l::Symbols,
        bytes: &[u8],
        id: l::FileId,
        tree: &l::NodeTable,
        local_words: Option<&FileWords>,
        words: &mut FileWords,
        out: &mut FilledColumns,
    ) -> bool {
        let n = l::node_table::element_count(tree).0;
        if self.has_rows(relation).is_none()
            || self.rows.partials.iter().any(|(r, _, _)| *r == relation)
            || self.rows.partials_at.iter().any(|(r, _, _)| *r == relation)
        {
            return false;
        }
        let starts = &self.offsets[catalog::number_of(relation).0];
        let kinds = catalog::kinds(relation);
        out.0.resize_with(kinds.len(), Default::default);
        let slot = catalog::number_of(relation).0;
        if self.local(slot).is_some() {
            let loan = FileColumns { home: id, tree, words: local_words.expect("file dictionary"), tables: &self.local };
            if !matches!(loan.coverage(slot), Some((l::FactAnswer::Answered(_), l::Wanted::Whole))) {
                return false;
            }
            for (index, (dst, kind)) in out.0.iter_mut().zip(kinds).enumerate() {
                for &raw in loan.column(slot, index).unwrap_or(&[]) {
                    dst.0.push(match kind {
                        CellKind::Node => match loan.position(raw) {
                            Ok(Some(pos)) => CellWord(u64::from(pos.0)),
                            _ => return false,
                        },
                        CellKind::Symbol => {
                            let Some(text) = loan.text(raw) else { return false };
                            l::facts::intern(words, text)
                        }
                        _ => raw,
                    });
                }
            }
            return true;
        }
        for (index, (column, kind)) in out.0.iter_mut().zip(kinds).enumerate() {
            column.0.reserve(starts.len());
            let cells = starts.iter().map(|&at| self.rows.words[at + index]);
            match kind {
                CellKind::Node => {
                    for raw in cells {
                        if raw >> 32 != u64::from(id.0) || raw as u32 as usize >= n {
                            return false;
                        }
                        column.0.push(CellWord(u64::from(raw as u32)));
                    }
                }
                CellKind::Symbol => {
                    for raw in cells {
                        let Some(text) =
                            l::text_of(symbols, ByteSlice(bytes), l::Symbol(raw as u32))
                        else {
                            return false;
                        };
                        column.0.push(l::facts::intern(words, text));
                    }
                }
                _ => column.0.extend(cells.map(CellWord)),
            }
        }
        true
    }
    pub(crate) fn view<'a, 'words>(
        &'a self,
        parsed: &'a mut reader::ReaderFacts<'words>,
        symbols: &'a l::Symbols,
        bytes: &'a [u8],
        local_words: Option<&'a FileWords>,
    ) -> View<'a, 'words> {
        View {
            packet: self,
            parsed,
            symbols,
            bytes,
            resolved: FileSymbols::default(),
            materialized: FxHashMap::default(),
            local_words,
            local_symbols: FxHashMap::default(),
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
/// Run-wide IDs for the words of the current file's symbol dictionary.
#[derive(Default)]
pub(crate) struct FileSymbols(FxHashMap<u32, u32>);

impl FileSymbols {
    pub(crate) fn resolve(
        &mut self,
        run: &mut finish::Run,
        unknown: &std::collections::HashMap<u32, u32>,
        symbols: &l::Symbols,
        bytes: &[u8],
        word: l::Symbol,
    ) -> u32 {
        *self.0.entry(word.0).or_insert_with(|| reader::symbol(run, unknown, symbols, bytes, word))
    }
}

pub(crate) struct View<'a, 'words> {
    pub(crate) renderer: Option<&'a mut dyn FnMut(l::RenderRequest) -> Result<l::RenderOutcome, String>>,
    packet: &'a Packet,
    local_words: Option<&'a FileWords>,
    local_symbols: FxHashMap<u64, u32>,
    parsed: &'a mut reader::ReaderFacts<'words>,
    symbols: &'a l::Symbols,
    bytes: &'a [u8],
    pub(crate) resolved: FileSymbols,
    materialized: FxHashMap<l::Relation, Rc<finish::Table>>,
}
impl finish::Provider for View<'_, '_> {
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
            .then(|| self.packet.count(i))
    }
    fn fill(
        &mut self,
        run: &mut finish::Run,
        name: &str,
        scope: finish::Scope<'_>,
    ) -> Result<finish::Table, String> {
        self.shared_fill(run, name, scope).map(|table| table.copy())
    }

    fn shared_fill(
        &mut self,
        run: &mut finish::Run,
        name: &str,
        scope: finish::Scope<'_>,
    ) -> Result<Rc<finish::Table>, String> {
        let (rel, names, tys) = self.layout(run, name, scope)?;
        if let Some(table) = self.materialized.get(&rel) {
            return Ok(Rc::clone(table));
        }
        let table = Rc::new(self.transpose(run, rel, names, tys)?);
        self.materialized.insert(rel, Rc::clone(&table));
        Ok(table)
    }
}

impl View<'_, '_> {
    fn layout(
        &self,
        run: &mut finish::Run,
        name: &str,
        scope: finish::Scope<'_>,
    ) -> Result<(l::Relation, finish::Names, finish::Tys), String> {
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
        if !self.packet.requested[catalog::number_of(rel).0] {
            return Err(format!("no reader for {name}"));
        }
        Ok((rel, names, tys))
    }

    fn transpose(
        &mut self,
        run: &mut finish::Run,
        rel: l::Relation,
        names: finish::Names,
        tys: finish::Tys,
    ) -> Result<finish::Table, String> {
        let home = self.parsed.file;
        let position = |raw: u64| -> Result<u64, String> {
            file_node(home, raw).map_err(|_| format!("foreign node in file-scoped {}", catalog::name(rel).0))
        };
        let slot = catalog::number_of(rel).0;
        if let Some(t) = self.packet.local(slot) {
            let mut table = finish::Table::retained(names, tys, 0);
            let words = self.local_words.expect("scoped file dictionary");
            for ((dst, src), ty) in table.cols.iter_mut().zip(&t.columns.0).zip(table.tys.iter()) {
                *dst = src.0.iter().map(|raw| Ok(match ty {
                    finish::Ty::Sym if raw.0 == u64::from(l::NO_SYMBOL.0) =>
                        u64::from(run.absent_word.expect("reader word table")),
                    finish::Ty::Sym => u64::from(*self.local_symbols.entry(raw.0).or_insert_with(
                        || match local_word_text(words, *raw) {
                            Some(text) => run.sym(text.0.as_bytes()),
                            None => run.public_word_ids.get(&(raw.0 as u32)).copied().unwrap_or(run.absent_word.expect("reader dictionary")),
                        })),
                    finish::Ty::Node => position(raw.0)?,
                    _ => raw.0,
                })).collect::<Result<_, String>>()?;
            }
            table.n = t.rows;
            self.parsed.fills_made += 1;
            return Ok(table);
        }
        let offsets = &self.packet.offsets[catalog::number_of(rel).0];
        let words = &self.packet.rows.words;
        let mut table = finish::Table::with_rows(names, tys, offsets.len());
        for (index, (column, ty)) in table.cols.iter_mut().zip(table.tys.iter()).enumerate() {
            let cells = offsets.iter().map(|&at| words[at + index]);
            match ty {
                finish::Ty::Sym => column.extend(cells.map(|raw| {
                    u64::from(self.resolved.resolve(
                        run, self.parsed.unknown, self.symbols, self.bytes, l::Symbol(raw as u32),
                    ))
                })),
                finish::Ty::Node => {
                    for raw in cells { column.push(position(raw)?); }
                }
                _ => column.extend(cells),
            }
        }
        table.n = offsets.len();
        self.parsed.fills_made += 1;
        Ok(table)
    }
}

#[cfg(test)]
mod file_node_tests {
    use super::*;
    #[test]
    fn a_diagnostic_identity_is_local_to_its_file_without_a_tree_position() {
        let diagnostic = catalog::diagnostic_id(l::FileId(19), l::ChildOrdinal(7));
        assert_eq!(file_node(19, diagnostic.0).unwrap(), u64::from(catalog::node_position(diagnostic).0));
        assert!(file_node(18, diagnostic.0).is_err());
        assert_eq!(file_node(19, finish::NULL).unwrap(), finish::NULL);
    }
}
