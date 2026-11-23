//! Supply a native, preorder parse tree to the vector executor. The row reader and the
//! node table retain exactly the parser's positions, including positions of lexical leaves.
use super::finish::{self, Provider, Run, Scope, Table};
use ryo::languages::{self as l, ByteSlice, FieldCode, NodeTable, node_table};
use ryo_relations::domain::relations::service as catalog;
use std::collections::HashMap;

pub(super) struct ReaderFacts {
    pub file: u32,
    pub(super) fills_made: u64,
    pub text: Vec<u32>,
    tables: HashMap<l::Relation, Table>,
    partial: HashMap<l::Relation, String>,
}

impl ReaderFacts {
    pub(super) fn new(file: u32) -> Self {
        Self {
            file,
            fills_made: 0,
            text: Vec::new(),
            tables: HashMap::new(),
            partial: HashMap::new(),
        }
    }

    /// Partition the provider's answer by relation, resolving symbols against this file's pool.
    pub(super) fn receive(
        &mut self,
        run: &mut Run,
        wanted: &[l::Relation],
        projections: &HashMap<l::Relation, l::ColumnProjection>,
        rows: &l::contract::Collect,
        symbols: &l::Symbols,
        bytes: &[u8],
    ) {
        for &relation in wanted {
            let (_, names, types) = run
                .catalog(catalog::name(relation).0)
                .expect("a catalog relation");
            self.tables
                .insert(relation, Table::with_rows(names, types, 0));
        }
        for (rel, cause, _) in &rows.partials {
            self.partial.insert(*rel, cause.clone());
        }
        for (rel, _, cause) in &rows.partials_at {
            self.partial.insert(*rel, cause.clone());
        }
        let mut words = rows.words.as_slice();
        for rel in &rows.relations {
            let table = self.tables.get_mut(rel).expect("requested relation");
            let (row, rest) = words.split_at(table.cols.len());
            words = rest;
            for (index, ((column, &word), ty)) in table.cols.iter_mut().zip(row).zip(table.tys.iter()).enumerate() {
                if !projections[rel].contains(index) { continue; }
                column.push(match ty {
                    finish::Ty::Sym => {
                        u64::from(symbol(run, symbols, bytes, l::Symbol(word as u32)))
                    }
                    finish::Ty::Node => {
                        assert_eq!(
                            word >> 32,
                            u64::from(self.file),
                            "foreign node in file-scoped answer"
                        );
                        word & 0xffff_ffff
                    }
                    _ => word,
                });
            }
            table.n += 1;
        }
        assert!(words.is_empty());
    }
}

pub(super) fn symbol(run: &mut Run, symbols: &l::Symbols, bytes: &[u8], word: l::Symbol) -> u32 {
    let text = l::text_of(symbols, ByteSlice(bytes), word);
    run.sym(text.map_or(b"", |t| t.0.as_bytes()))
}

pub(super) struct Packet<'a> {
    pub reader: &'a mut ReaderFacts,
    pub read: &'a mut dyn FnMut(&mut Run, &mut ReaderFacts, &[l::Relation], &HashMap<l::Relation, l::ColumnProjection>),
}

impl Provider for Packet<'_> {
    fn syntax_text(&self) -> Option<&[u32]> {
        Some(&self.reader.text)
    }

    fn answer(&mut self, run: &mut Run, request: &finish::request::Request<'_>, out: &mut Vec<Result<Table, String>>) -> Result<(), String> {
        let mut selections = HashMap::new();
        let mut asks = Vec::with_capacity(request.relations.len());
        let mut facts = Vec::new();
        let mut remaining = HashMap::new();
        for asked in request.relations {
            let valid = match asked.scope {
                Scope::File(file) => file == self.reader.file,
                Scope::Files(files) => files == [self.reader.file],
            };
            let entry = if valid {
                catalog::relation_named(ryo_relations::domain::labels::types::NameRef(asked.rel))
                    .ok_or_else(|| format!("undeclared relation {}", asked.rel))
            } else {
                Err(format!("relation {} requested outside this reader's file", asked.rel))
            };
            if let Ok(rel) = entry {
                let projection = selections.entry(rel).or_insert_with(|| {
                    facts.push(rel);
                    l::ColumnProjection::First(0)
                });
                projection.extend(asked.columns);
                *remaining.entry(rel).or_insert(0) += 1;
            }
            asks.push(entry);
        }
        (self.read)(run, self.reader, &facts, &selections);
        for (asked, entry) in request.relations.iter().zip(asks) {
            out.push(entry.and_then(|rel| {
                if let Some(cause) = self.reader.partial.get(&rel) {
                    return Err(format!("partial {}: {cause}", asked.rel));
                }
                let left = remaining.get_mut(&rel).expect("requested relation");
                *left -= 1;
                let mut table = if *left == 0 { self.reader.tables.remove(&rel) }
                                else { self.reader.tables.get(&rel).map(Table::copy) }
                    .ok_or_else(|| format!("no reader for {}", asked.rel))?;
                for (index, column) in table.cols.iter_mut().enumerate() {
                    if !asked.columns.contains(index) { column.clear(); }
                }
                self.reader.fills_made += 1;
                Ok(table)
            }));
        }
        Ok(())
    }

    fn render(&mut self, _: u32, context: &str) -> Result<Option<Vec<u8>>, String> {
        Err(format!(
            "reader does not supply syntax rendering in {context}"
        ))
    }
}

pub(super) fn project_tree(
    elements: &l::Elements,
    symbols: &l::Symbols,
    bytes: &[u8],
    run: &mut Run,
    table: &mut NodeTable,
    facts: &mut ReaderFacts,
) {
    let mut builder = node_table::builder(table, l::Count(run.kind_sym.len()));
    let mut open = Vec::new();
    for at in l::positions(elements) {
        while open.last().is_some_and(|&(last, _)| last < at) {
            let (_, end) = open.pop().unwrap();
            node_table::leave_node(&mut builder, end);
        }
        let field = l::field_at(elements, at).0;
        let name = symbol(run, symbols, bytes, l::field_name_at(elements, at));
        run.field_sym
            .resize(run.field_sym.len().max(usize::from(field) + 1), run.empty);
        run.field_sym[usize::from(field)] = name;
        let text = if l::flags_at(elements, at).0 & l::TOKEN.0 != 0 {
            symbol(run, symbols, bytes, l::text_at(elements, at))
        } else {
            run.empty
        };
        facts.text.push(text);
        let kind = l::kind_at(elements, at);
        node_table::enter_node(
            &mut builder,
            kind,
            FieldCode(field),
            l::start_at(elements, at),
        );
        open.push((l::last_at(elements, at), l::end_at(elements, at)));
    }
    while let Some((_, end)) = open.pop() {
        node_table::leave_node(&mut builder, end);
    }
    node_table::set_syntax_acceptance(&mut builder, l::syntax_acceptance(elements));
    let _ = node_table::finished(&mut builder);
}
