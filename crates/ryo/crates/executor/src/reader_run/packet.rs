//! One file-scoped row packet, indexed once for both phases. Resolving a column
//! or translating its symbols into the executor dictionary waits for demand.
use super::super::{finish, reader};
use rustc_hash::FxHashMap;
use crate::languages::{self as l, ByteSlice, CellKind, CellWord, FileWords, FilledColumns};
use ryo_relations::domain::relations::service as catalog;
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

pub const RELATIONS: usize = catalog::ALL.len();

/// A file's typed fact columns and the extent of the fill that produced them.
pub struct LocalTable {
    pub rows: usize,
    pub columns: FilledColumns,
    pub answer: l::FactAnswer,
    pub wanted: l::Wanted,
}

/// Borrow the file's own IDs and buffers until the consumer finishes its visit.
pub struct FileColumns<'f> {
    pub home: l::FileId,
    pub packet: &'f Packet,
}

/// The file's word dictionary. A relation filled the first time a step reads it can grow it, so everything that
/// reads it borrows it only for the moment it reads.
pub type Dictionary<'d> = RefCell<&'d mut FileWords>;

/// The provider that fills a relation of this file the first time a step loads it. Only a file whose every relation is a
/// complete answer has one.
pub struct Lazy<'a, 'd> {
    pub supplier: &'a RefCell<&'d mut dyn l::NumberedFacts>,
    /// A late fill that was not a complete answer: the provider said the file was complete and it was not.
    pub failed: Cell<bool>,
}

impl<'a, 'd> Lazy<'a, 'd> {
    pub fn new(supplier: &'a RefCell<&'d mut dyn l::NumberedFacts>) -> Self {
        Self { supplier, failed: Cell::new(false) }
    }
}

#[derive(Debug)]
pub struct BadNode;

// Relations may carry file-owned diagnostic identities beyond the syntax tree.
pub fn file_node(home: u32, raw: u64) -> Result<u64, BadNode> {
    if raw == finish::NULL {
        Ok(raw)
    } else if raw >> 32 == u64::from(home) {
        Ok(u64::from(raw as u32))
    } else {
        Err(BadNode)
    }
}

impl FileColumns<'_> {
    pub fn column(&self, relation: usize, column: usize) -> Option<&[CellWord]> {
        let table = self.packet.local(relation)?;
        table.columns.0.get(column)?.0.get(..table.rows)
    }


}

pub fn local_word_text(words: &FileWords, word: CellWord, end: CellWord) -> Option<l::NameText<'_>> {
    u32::try_from(word.0).ok().and_then(|code| {
        ryo_relations::domain::terms::service::term_text(l::Symbol(code))
    }).or_else(|| (word < end).then(|| l::facts::text(words, word)))
}

pub struct OperatorColumns<'file, 'd> {
    pub file: FileColumns<'file>,
    pub lazy: Option<&'file Lazy<'file, 'd>>,
    pub dictionary: Option<&'file Dictionary<'d>>,
    pub caches: Vec<OnceCell<finish::fact_view::ColumnCache>>,
    /// The run's word for each own word of the dictionary seen so far, in order.
    pub own: RefCell<Vec<u32>>,
    /// One snapshot of the dictionary's words as run words for each relation bound.
    pub symbols: Vec<OnceCell<Vec<u32>>>,
    pub public: Rc<FxHashMap<u32,u32>>,
    pub absent: u32,
}

impl finish::fact_view::FileFacts for OperatorColumns<'_, '_> {
    fn bind(&self, relation: l::Relation, run: &mut finish::Run) -> Result<Option<finish::fact_view::Fact<'_>>, String> {
        let slot = catalog::number_of(relation).0;
        let table = match self.dictionary {
            Some(dictionary) => self.file.packet.table(relation, &mut dictionary.borrow_mut(), self.lazy),
            None => self.file.packet.local(slot),
        };
        let Some(table) = table else {
            return Ok(None);
        };
        if !matches!(table.answer, l::FactAnswer::Answered(_)) || table.wanted != l::Wanted::Whole {
            return Ok(None);
        }
        for (column, kind) in catalog::kinds(relation).iter().enumerate() {
            if *kind == CellKind::Symbol && table.rows != 0 {
                self.symbols[slot].get_or_init(|| {
                    let dictionary = self.dictionary.expect("a numbered table has its dictionary").borrow();
                    let words: &FileWords = &dictionary;
                    let base = l::facts::base_count(words).0;
                    if l::facts::base_changed(words, &mut run.fact_word_base) {
                        run.base_word_sym = (0..base)
                            .map(|id| run.sym(l::facts::text(words, CellWord(id)).0.as_bytes()))
                            .collect();
                    }
                    let mut own = self.own.borrow_mut();
                    let end = l::facts::words_end(words).0;
                    let next = base + own.len() as u64;
                    own.extend((next..end).map(|id| run.sym(l::facts::text(words, CellWord(id)).0.as_bytes())));
                    let mut ids = run.base_word_sym.clone();
                    ids.extend_from_slice(&own);
                    ids
                });
            }
            if *kind == CellKind::Node && self.file.column(slot, column).unwrap_or_default().iter()
                .any(|&value| file_node(self.file.home.0, value.0).is_err()) {
                return Err(format!("node outside this file in {}", catalog::name(relation).0));
            }
        }
        let kinds = catalog::kinds(relation);
        Ok(Some(finish::fact_view::Fact::Lent {
            rows: table.rows, kinds,
            columns: finish::fact_view::Columns::Lent(&table.columns.0, kinds, self.symbols[slot].get().map_or(&[], Vec::as_slice), &self.public, self.absent,
                self.caches[slot].get_or_init(|| finish::fact_view::ColumnCache::new(kinds.len()))),
        }))
    }
}

pub fn widths() -> &'static [usize; RELATIONS] {
    static WIDTHS: std::sync::OnceLock<[usize; RELATIONS]> = std::sync::OnceLock::new();
    WIDTHS.get_or_init(|| {
        let mut widths = [0; RELATIONS];
        for &rel in catalog::ALL {
            widths[catalog::number_of(rel).0] = catalog::kinds(rel).len();
        }
        widths
    })
}

pub struct Packet {
    /// The relations the language read in part, each with the cause and what is lacking.
    pub partials: Vec<(l::Relation, String, l::Missing)>,
    pub local: Vec<OnceCell<LocalTable>>,
    pub spare: RefCell<FxHashMap<usize, FilledColumns>>,
    pub requested: Vec<bool>,
    pub indexed: Vec<usize>,
    /// The file fills its numbered relations when a step first loads them: a relation not yet filled has no row count.
    pub lazy_fills: bool,
    pub ready: bool,
}
impl Packet {
    pub fn new() -> Self {
        Self {
            partials: Vec::new(),
            local: (0..RELATIONS).map(|_| OnceCell::new()).collect(),
            spare: Default::default(),
            requested: vec![false; RELATIONS],
            indexed: Vec::new(),
            lazy_fills: false,
            ready: false,
        }
    }
    pub fn touch(&mut self, slot: usize) {
        if !self.requested[slot] {
            self.requested[slot] = true;
            self.indexed.push(slot);
        }
    }

    pub fn clear(&mut self) {
        self.partials.clear();
        for slot in self.indexed.drain(..) {
            self.requested[slot] = false;
        }
        for (slot, cell) in self.local.iter_mut().enumerate() {
            if let Some(table) = cell.take() {
                self.spare.get_mut().insert(slot, table.columns);
            }
        }
        self.ready = false;
        self.lazy_fills = false;
    }
    /// The structural projection has its own provider contract. Every filled
    /// catalog relation is published directly in columns, once for this file.
    pub fn fill_numbered(
        &mut self, supplier: &mut dyn l::NumberedFacts, relations: &[l::Relation],
        words: &mut FileWords,
    ) -> Result<(), String> {
        use ryo_relations::domain::{catalog::service::fact_form, relations::types::FactForm};
        for &relation in relations {
            // The node-table relations are read from the numbered table by the executor: nothing is filled for them.
            if fact_form(relation) == FactForm::NodeTable {
                continue;
            }
            let slot = catalog::number_of(relation).0;
            let table = self.fill_into(relation, slot, words, |relation, words, columns| {
                supplier.fill_columns(relation, l::Wanted::Whole, words, columns)
            })?;
            match &table.answer {
                l::FactAnswer::Partial(part) => self.partials.push((relation, part.cause.0.to_string(), part.missing)),
                l::FactAnswer::Refused(l::AbsenceCause::NotYetRead) => {
                    self.partials.push((relation, "catalog relation has no numbered filler".into(), l::Missing::Any));
                }
                _ => {}
            }
            // A later fill may refuse before index(). The next visit must still
            // reclaim every population already written for this one.
            self.touch(slot);
            assert!(self.local[slot].set(table).is_ok(), "a relation is filled once for a file");
        }
        Ok(())
    }
    /// Fill one relation into reusable columns and check their shape.
    pub fn fill_into(
        &self, relation: l::Relation, slot: usize, words: &mut FileWords,
        fill: impl FnOnce(l::Relation, &mut FileWords, &mut FilledColumns) -> l::FactAnswer,
    ) -> Result<LocalTable, String> {
        let mut columns = self.spare.borrow_mut().remove(&slot).unwrap_or_default();
        for column in &mut columns.0 { column.0.clear(); }
        let answer = fill(relation, words, &mut columns);
        let rows = columns.0.first().map_or(0, |column| column.0.len());
        if rows > 0 && columns.0.len() != widths()[slot] {
            return Err(format!("invalid column count in {}", catalog::name(relation).0));
        }
        if columns.0.iter().any(|column| column.0.len() != rows) {
            return Err(format!("unequal column lengths in {}", catalog::name(relation).0));
        }
        Ok(LocalTable { rows, columns, answer, wanted: l::Wanted::Whole })
    }
    /// The relation's numbered table: the one filled up front, or the one filled now because a step loads it first.
    pub fn table(&self, relation: l::Relation, words: &mut FileWords, lazy: Option<&Lazy<'_, '_>>) -> Option<&LocalTable> {
        let slot = catalog::number_of(relation).0;
        if let Some(table) = self.local[slot].get() {
            return Some(table);
        }
        let lazy = lazy?;
        let table = self.fill_into(relation, slot, words, |relation, words, columns| {
            lazy.supplier.borrow_mut().fill_columns(relation, l::Wanted::Whole, words, columns)
        });
        let Ok(table) = table else {
            lazy.failed.set(true);
            return None;
        };
        // The same outcome as the fill up front (`fill_numbered`): a relation the language does not have, answered `Refused` with its
        // cause, is the relation's absence (the table keeps its answer and `bind` finds no facts); only an answer the late fill
        // cannot report (a partial one, or a relation with no filler at all) fails the request, for the file's partial coverage is
        // taken before any step runs.
        if matches!(table.answer, l::FactAnswer::Partial(_) | l::FactAnswer::Refused(l::AbsenceCause::NotYetRead)) {
            lazy.failed.set(true);
        }
        let _ = self.local[slot].set(table);
        self.local[slot].get()
    }
    pub fn operator_columns<'a, 'd>(
        &'a self, file: l::FileId, dictionary: Option<&'a Dictionary<'d>>, lazy: Option<&'a Lazy<'a, 'd>>,
        run: &mut finish::Run,
    ) -> OperatorColumns<'a, 'd> {
        let file = FileColumns { home: file, packet: self };
        OperatorColumns {
            file, lazy, dictionary,
            caches: (0..RELATIONS).map(|_| OnceCell::new()).collect(),
            own: RefCell::new(Vec::new()),
            symbols: (0..RELATIONS).map(|_| OnceCell::new()).collect(),
            public: run.public_word_ids.clone(), absent: run.absent_word.expect("reader dictionary"),
        }
    }
    pub fn local(&self, slot: usize) -> Option<&LocalTable> {
        self.local[slot].get()
    }
    pub fn count(&self, slot: usize) -> usize {
        self.local(slot).map_or(0, |t| t.rows)
    }
    pub fn index(&mut self, wanted: &[l::Relation]) {
        debug_assert!(!self.ready);
        for &rel in wanted {
            let slot = catalog::number_of(rel).0;
            self.touch(slot);
        }
        self.ready = true;
    }
    /// An indexed, requested relation already has an exact row count. The
    /// caller withholds this observation when the provider declared a partial.
    pub fn has_rows(&self, relation: l::Relation) -> Option<bool> {
        let slot = catalog::number_of(relation).0;
        if self.lazy_fills && self.local[slot].get().is_none()
            && ryo_relations::domain::catalog::service::fact_form(relation) != ryo_relations::domain::relations::types::FactForm::NodeTable
        {
            return None;
        }
        (self.ready && self.requested[slot]).then(|| self.count(slot) != 0)
    }
    /// Visit a complete symbol column without interpreting unrelated node IDs.
    pub fn any_symbol(
        &self, relation: l::Relation, column: usize, symbols: &l::Symbols, bytes: &[u8],
        local_words: Option<(&FileWords, CellWord)>, mut matches: impl FnMut(Option<l::NameText<'_>>, l::Symbol) -> bool,
    ) -> Option<bool> {
        self.has_rows(relation)?;
        if catalog::kinds(relation).get(column) != Some(&CellKind::Symbol) { return None; }
        let slot = catalog::number_of(relation).0;
        if let Some(table) = self.local(slot) {
            let (words, end) = local_words?;
            return Some(table.rows != 0 && table.columns.0[column].0.iter().any(|&raw| {
                matches(local_word_text(words, raw, end), l::Symbol(raw.0 as u32))
            }));
        }
        None
    }
    pub fn columns(
        &self,
        relation: l::Relation,
        symbols: &l::Symbols,
        bytes: &[u8],
        id: l::FileId,
        numbered_end: Option<CellWord>,
        unknown: &std::collections::HashMap<u32, u32>,
        words: &mut FileWords,
        lazy: Option<&Lazy<'_, '_>>,
        out: &mut FilledColumns,
    ) -> bool {
        if numbered_end.is_some() {
            let _ = self.table(relation, words, lazy);
        }
        let numbered_end = numbered_end.map(|_| l::facts::words_end(words));
        if self.has_rows(relation).is_none()
            || self.partials.iter().any(|(r, _, _)| *r == relation)
        {
            return false;
        }
        let reserved = |raw: u64| raw == u64::from(l::NO_SYMBOL.0)
            || u32::try_from(raw).is_ok_and(|code| unknown.contains_key(&code));
        let kinds = catalog::kinds(relation);
        out.0.resize_with(kinds.len(), Default::default);
        let slot = catalog::number_of(relation).0;
        if let Some(table) = self.local(slot) {
            let Some(end) = numbered_end else { return false };
            if !matches!(table.answer, l::FactAnswer::Answered(_)) || table.wanted != l::Wanted::Whole {
                return false;
            }
            let home = u64::from(id.0);
            for (index, (dst, kind)) in out.0.iter_mut().zip(kinds).enumerate() {
                let Some(cells) = table.columns.0.get(index) else { continue };
                dst.0.reserve(table.rows);
                match kind {
                    CellKind::Node => {
                        // A node cell is this file's own identity or null; one test per cell, one verdict per column.
                        let mut foreign = false;
                        dst.0.extend(cells.0.iter().map(|&cell| {
                            if cell.0 == finish::NULL {
                                return cell;
                            }
                            foreign |= cell.0 >> 32 != home;
                            CellWord(u64::from(cell.0 as u32))
                        }));
                        if foreign {
                            return false;
                        }
                    }
                    CellKind::Symbol => {
                        // Words of the file's own dictionary pass as they are; a closed term outside it is interned.
                        let before = dst.0.len();
                        let mut beyond = false;
                        dst.0.extend(cells.0.iter().map(|&cell| {
                            beyond |= cell >= end;
                            cell
                        }));
                        if beyond {
                            dst.0.truncate(before);
                            for &raw in &cells.0 {
                                dst.0.push(if raw >= end && !reserved(raw.0) {
                                    let Some(term) = u32::try_from(raw.0).ok().and_then(|code|
                                        ryo_relations::domain::terms::service::term_text(l::Symbol(code)))
                                    else { return false };
                                    l::facts::intern(words, term)
                                } else {
                                    raw
                                });
                            }
                        }
                    }
                    _ => dst.0.extend_from_slice(&cells.0),
                }
            }
            return true;
        }
        false
    }
    pub fn view<'a, 'words, 'd>(
        &'a self,
        parsed: &'a mut reader::ReaderFacts<'words>,
        symbols: &'a l::Symbols,
        bytes: &'a [u8],
        local_words: Option<&'a Dictionary<'d>>,
        lazy: Option<&'a Lazy<'a, 'd>>,
    ) -> View<'a, 'words, 'd> {
        View {
            packet: self,
            parsed,
            symbols,
            bytes,
            resolved: FileSymbols::default(),
            materialized: FxHashMap::default(),
            local_words,
            lazy,
            local_symbols: FxHashMap::default(),
            renderer: None,
        }
    }
    pub fn partial(&self, rel: l::Relation) -> Option<&str> {
        self.partials.iter().rev().find(|r| r.0 == rel).map(|r| r.1.as_str())
    }
}
/// Run-wide IDs for the words of the current file's symbol dictionary.
#[derive(Default)]
pub struct FileSymbols(Vec<u32>);

impl FileSymbols {
    pub fn resolve(
        &mut self,
        run: &mut finish::Run,
        unknown: &std::collections::HashMap<u32, u32>,
        symbols: &l::Symbols,
        bytes: &[u8],
        word: l::Symbol,
    ) -> u32 {
        if word == l::NO_SYMBOL {
            return run.absent_word.expect("reader word table");
        }
        let index = word.0 as usize;
        if index >= self.0.len() {
            let count = l::contract::name_count(symbols).0;
            if index >= count {
                // Published terms and unknowns have their own namespace. A
                // reserved code must never size the file dictionary.
                return reader::symbol(run, unknown, symbols, bytes, word);
            }
            self.0.resize(count, u32::MAX);
        }
        let cell = &mut self.0[index];
        if *cell == u32::MAX {
            *cell = reader::symbol(run, unknown, symbols, bytes, word);
        }
        *cell
    }
}

pub struct View<'a, 'words, 'd> {
    pub renderer: Option<&'a mut dyn FnMut(l::RenderRequest) -> Result<l::RenderOutcome, String>>,
    pub packet: &'a Packet,
    pub local_words: Option<&'a Dictionary<'d>>,
    pub lazy: Option<&'a Lazy<'a, 'd>>,
    pub local_symbols: FxHashMap<u64, u32>,
    pub parsed: &'a mut reader::ReaderFacts<'words>,
    pub symbols: &'a l::Symbols,
    pub bytes: &'a [u8],
    pub resolved: FileSymbols,
    pub materialized: FxHashMap<l::Relation, Rc<finish::Table>>,
}
impl finish::Provider for View<'_, '_, '_> {
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
        if let Some(words) = self.local_words {
            let _ = self.packet.table(rel, &mut words.borrow_mut(), self.lazy);
        }
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

impl View<'_, '_, '_> {
    pub fn layout(
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

    pub fn transpose(
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
        if let Some(words) = self.local_words {
            let _ = self.packet.table(rel, &mut words.borrow_mut(), self.lazy);
        }
        if let Some(t) = self.packet.local(slot) {
            let mut table = finish::Table::retained(names, tys, 0);
            let dictionary = self.local_words.expect("scoped file dictionary").borrow();
            let words: &FileWords = &dictionary;
            for ((dst, src), ty) in table.cols.iter_mut().zip(&t.columns.0).zip(table.tys.iter()) {
                *dst = src.0.iter().map(|raw| Ok(match ty {
                    finish::Ty::Sym if raw.0 == u64::from(l::NO_SYMBOL.0) =>
                        u64::from(run.absent_word.expect("reader word table")),
                    finish::Ty::Sym => u64::from(*self.local_symbols.entry(raw.0).or_insert_with(
                        || match local_word_text(words, *raw, l::facts::words_end(words)) {
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
        Err(format!("no numbered table for {}", catalog::name(rel).0))
    }
}

#[cfg(test)]
mod file_node_tests {
    use super::*;
    #[test]
    fn resolving_published_terms_leaves_room_only_for_file_symbols() {
        let mut run = finish::Run::default();
        let unknown = reader::public_words(&mut run, &[]);
        let mut symbols = l::Symbols::default();
        let mut lookup = FileSymbols::default();
        let reserved = ryo_relations::domain::terms::service::declared_terms().next().unwrap().0;
        assert_eq!(lookup.resolve(&mut run, &unknown, &symbols, b"", reserved),
            reader::symbol(&mut run, &unknown, &symbols, b"", reserved));
        assert!(lookup.0.is_empty());
        for text in ["one", "two"] {
            let id = l::intern_derived(&mut symbols, ByteSlice(b""), l::NameText(text));
            assert_eq!(lookup.resolve(&mut run, &unknown, &symbols, b"", id), run.sym(text.as_bytes()));
        }
        assert_eq!(lookup.0.len(), 2);
        assert_eq!(lookup.resolve(&mut run, &unknown, &symbols, b"", l::NO_SYMBOL), run.absent_word.unwrap());
    }
    #[test]
    fn reset_reclaims_a_local_population_even_if_the_visit_failed_before_index() {
        // A language whose second fill is malformed (one column where the relation has more): the visit fails after the first fill
        // was kept, before the packet is indexed.
        struct RefusesSecondFill;
        impl l::NumberedFacts for RefusesSecondFill {
            fn fill_columns(&mut self, relation: l::Relation, _: l::Wanted, _: &mut FileWords,
                columns: &mut FilledColumns) -> l::FactAnswer {
                let width = if relation == l::Relation::SyntaxError { catalog::kinds(relation).len() } else { 1 };
                columns.0.resize_with(width, Default::default);
                for column in &mut columns.0 { column.0.push(CellWord(0)); }
                l::FactAnswer::Answered(l::FillExtent::Asked)
            }
            fn project_structure(&mut self, _: &[l::Relation], _: &mut l::Symbols,
                _: &mut dyn ryo_relations::domain::relations::types::RowWriter) -> Result<(), l::ParseError> {
                Err(l::ParseError::Unsupported)
            }
            fn named(&mut self, _: &[&l::FileRelation], _: &mut l::Symbols) -> Vec<Result<l::FileRows, l::ParseError>> {
                unreachable!()
            }
            fn render(&mut self, _: l::RenderRequest) -> Result<l::RenderOutcome, l::ParseError> {
                unreachable!()
            }
        }
        let mut packet = Packet::new();
        let id = catalog::number_of(l::Relation::SyntaxError).0;
        let result = packet.fill_numbered(&mut RefusesSecondFill,
            &[l::Relation::SyntaxError, l::Relation::Comment],
            &mut FileWords::default());
        assert!(result.is_err());
        assert!(packet.local(id).is_some());
        assert!(!packet.ready);
        packet.clear();
        assert!(packet.local(id).is_none());
        assert!(!packet.requested[id]);
        assert!(packet.spare.borrow().contains_key(&id));
    }

    #[test]
    fn a_diagnostic_identity_is_local_to_its_file_without_a_tree_position() {
        let diagnostic = catalog::diagnostic_id(l::FileId(19), l::ChildOrdinal(7));
        assert_eq!(file_node(19, diagnostic.0).unwrap(), u64::from(catalog::node_position(diagnostic).0));
        assert!(file_node(18, diagnostic.0).is_err());
        assert_eq!(file_node(19, finish::NULL).unwrap(), finish::NULL);
    }
}
