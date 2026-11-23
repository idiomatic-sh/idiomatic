//! Supply a native, preorder parse tree to the vector executor. The row reader and the
//! node table retain exactly the parser's positions, including positions of lexical leaves.
use super::finish::{self, Provider, Run, Scope, Table};
use ryo::languages::{self as l, ByteSlice, FieldCode, NodeTable, node_table};
use ryo_relations::domain::relations::service as catalog;
use std::collections::HashMap;

pub(super) struct ReaderFacts<'a> {
    unknown: &'a HashMap<u32, u32>,
    pub file: u32,
    pub(super) fills_made: u64,
    pub text: Vec<u32>,
    tables: HashMap<l::Relation, Table>,
    partial: HashMap<l::Relation, String>,
}

impl<'a> ReaderFacts<'a> {
    pub(super) fn new(file: u32, unknown: &'a HashMap<u32, u32>) -> Self {
        Self {
            file,
            unknown,
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
            for ((column, &word), ty) in table.cols.iter_mut().zip(row).zip(table.tys.iter()) {
                column.push(match ty {
                    finish::Ty::Sym => {
                        u64::from(symbol(run, self.unknown, symbols, bytes, l::Symbol(word as u32)))
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

/// Reserve distinct identities for missing text and unresolved names. They are
/// outside the interner, so an ordinary string can have the same displayed bytes.
pub(super) fn public_words(run: &mut Run, reasons: &[l::UnknownName]) -> HashMap<u32, u32> {
    let mut unknown = HashMap::new();
    let absent = u32::try_from(run.texts.len()).expect("word id");
    run.texts.push(Box::default());
    run.absent_word = Some(absent);
    for reason in reasons {
        let number = u32::try_from(run.texts.len()).expect("word id");
        run.texts.push(format!("unknown: {}", reason.text.0).into_bytes().into_boxed_slice());
        unknown.insert(l::unknown_symbol(reason.reason).0, number);
    }
    unknown
}

pub(super) fn symbol(run: &mut Run, unknown: &HashMap<u32, u32>, symbols: &l::Symbols, bytes: &[u8], word: l::Symbol) -> u32 {
    if let Some(text) = l::text_of(symbols, ByteSlice(bytes), word) {
        return run.sym(text.0.as_bytes());
    }
    if let Some(&published) = unknown.get(&word.0) {
        return published;
    }
    run.absent_word.expect("reader word table")
}

impl Provider for ReaderFacts<'_> {
    fn syntax_text(&self) -> Option<&[u32]> {
        Some(&self.text)
    }

    fn fill(&mut self, run: &mut Run, name: &str, scope: Scope<'_>) -> Result<Table, String> {
        if !matches!(scope, Scope::File(f) if f == self.file) {
            return Err(format!(
                "relation {name} requested outside this reader's file"
            ));
        }
        let rel = run
            .catalog(name)
            .and_then(|d| d.0)
            .ok_or_else(|| format!("undeclared relation {name}"))?;
        if let Some(cause) = self.partial.get(&rel) {
            return Err(format!("partial {name}: {cause}"));
        }
        let table = self.tables.get(&rel).ok_or_else(|| format!("no reader for {name}"))?;
        self.fills_made += 1;
        Ok(table.copy())
    }

    fn fill_len(&mut self, run: &mut Run, name: &str, scope: Scope<'_>) -> Option<usize> {
        if !matches!(scope, Scope::File(f) if f == self.file) {
            return None;
        }
        let rel = run.catalog(name)?.0?;
        if self.partial.contains_key(&rel) {
            return None;
        }
        Some(self.tables.get(&rel)?.n)
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
    facts: &mut ReaderFacts<'_>,
) {
    let mut builder = node_table::builder(table, l::Count(run.kind_sym.len()));
    let mut open = Vec::new();
    for at in l::positions(elements) {
        while open.last().is_some_and(|&(last, _)| last < at) {
            let (_, end) = open.pop().unwrap();
            node_table::leave_node(&mut builder, end);
        }
        let field = l::field_at(elements, at).0;
        let name = symbol(run, facts.unknown, symbols, bytes, l::field_name_at(elements, at));
        run.field_sym
            .resize(run.field_sym.len().max(usize::from(field) + 1), run.empty);
        run.field_sym[usize::from(field)] = name;
        // The parser's published scalar may also belong to a structural or implied node.
        let text = symbol(run, facts.unknown, symbols, bytes, l::text_at(elements, at));
        facts.text.push(text);
        let kind = l::kind_at(elements, at);
        if l::start_at(elements, at) == l::NO_SPAN.start && l::end_at(elements, at) == l::NO_SPAN.end {
            node_table::enter_unlocated_node(&mut builder, kind, FieldCode(field));
        } else {
            node_table::enter_node(&mut builder, kind, FieldCode(field), l::start_at(elements, at));
        }
        open.push((l::last_at(elements, at), l::end_at(elements, at)));
    }
    while let Some((_, end)) = open.pop() {
        node_table::leave_node(&mut builder, end);
    }
    node_table::set_syntax_acceptance(&mut builder, l::syntax_acceptance(elements));
    let _ = node_table::finished(&mut builder);
}
