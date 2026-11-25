//! Supply a native, preorder parse tree to the vector executor. The row reader and the
//! node table retain exactly the parser's positions, including positions of lexical leaves.
use super::finish::Run;
use std::collections::HashMap;
use ryo::languages::{self as l, ByteSlice, FieldCode, NodeTable, node_table};

pub(super) struct ReaderFacts<'a> {
    pub(super) unknown: &'a HashMap<u32, u32>,
    pub file: u32,
    pub(super) fills_made: u64,
    pub text: Vec<u32>,
    pub projects_words: bool,
    pub projects_fields: bool,
}

impl<'a> ReaderFacts<'a> {
    pub(super) fn new(file: u32, unknown: &'a HashMap<u32, u32>) -> Self {
        Self {
            file,
            unknown,
            fills_made: 0,
            text: Vec::new(),
            projects_words: true,
            projects_fields: true,
        }
    }
}

/// Reserve distinct identities for missing text and unresolved names. They are
/// outside the interner, so an ordinary string can have the same displayed bytes.
pub(super) fn public_words(run: &mut Run, reasons: &[l::UnknownName]) -> HashMap<u32, u32> {
    let mut unknown = HashMap::new();
    if run.absent_word.is_none() {
        let absent = u32::try_from(run.texts.len()).expect("word id");
        run.texts.push(Default::default());
        run.absent_word = Some(absent);
    }
    for reason in reasons {
        let number = u32::try_from(run.texts.len()).expect("word id");
        run.texts.push(format!("unknown: {}", reason.text.0).into_bytes().into());
        run.unresolved_words.push(number);
        unknown.insert(l::unknown_symbol(reason.reason).0, number);
    }
    let mut public: rustc_hash::FxHashMap<_, _> = unknown.iter().map(|(&raw, &id)| (raw, id)).collect();
    for (raw, text) in ryo_relations::domain::terms::service::declared_terms() {
        public.insert(raw.0, run.sym(text.0.as_bytes()));
    }
    run.public_word_ids = std::rc::Rc::new(public);
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

pub(super) fn project_tree(
    elements: &l::Elements,
    symbols: &l::Symbols,
    bytes: &[u8],
    run: &mut Run,
    table: &mut NodeTable,
    facts: &mut ReaderFacts<'_>,
) {
    if let Some(native) = l::native_table(elements) {
        if !facts.projects_fields && !facts.projects_words { return; }
        let mut field_names = Vec::new();
        let mut spelling = rustc_hash::FxHashMap::default();
        for (index, &field) in node_table::field_codes(native).iter().enumerate() {
            let at = l::Position(index.try_into().expect("native position"));
            if facts.projects_fields {
                let slot = usize::from(field);
                if field_names.len() <= slot { field_names.resize(slot + 1, None); }
                let _ = field_names[slot].get_or_insert_with(|| symbol(run, facts.unknown, symbols, bytes, l::field_name_at(elements, at)));
            }
            if facts.projects_words {
                let word = l::text_at(elements, at);
                facts.text.push(*spelling.entry(word.0).or_insert_with(|| symbol(run, facts.unknown, symbols, bytes, word)));
            }
        }
        for (slot, name) in field_names.into_iter().enumerate() {
            if let Some(name) = name {
                run.field_sym.resize(run.field_sym.len().max(slot + 1), run.empty);
                run.field_sym[slot] = name;
            }
        }
        return;
    }
    let mut builder = node_table::builder(table, l::Count(run.kind_sym.len()));
    let mut open = Vec::new();
    for at in l::positions(elements) {
        while open.last().is_some_and(|&(last, _)| last < at) {
            let (_, end) = open.pop().unwrap();
            node_table::leave_node(&mut builder, end);
        }
        let field = l::field_at(elements, at).0;
        if facts.projects_fields {
            let name = symbol(run, facts.unknown, symbols, bytes, l::field_name_at(elements, at));
            run.field_sym
                .resize(run.field_sym.len().max(usize::from(field) + 1), run.empty);
            run.field_sym[usize::from(field)] = name;
        }
        // The parser's published scalar may also belong to a structural or implied node.
        if facts.projects_words {
            facts.text.push(symbol(run, facts.unknown, symbols, bytes, l::text_at(elements, at)));
        }
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
