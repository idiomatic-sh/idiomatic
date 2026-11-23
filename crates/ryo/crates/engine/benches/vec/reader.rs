//! Supply a native, preorder parse tree to the vector executor. The row reader and the
//! node table retain exactly the parser's positions, including positions of lexical leaves.
use super::finish::Run;
use ryo::languages::{self as l, ByteSlice, FieldCode, NodeTable, node_table};

pub(super) struct ReaderFacts {
    pub file: u32,
    pub(super) fills_made: u64,
    pub text: Vec<u32>,
}

impl ReaderFacts {
    pub(super) fn new(file: u32) -> Self {
        Self {
            file,
            fills_made: 0,
            text: Vec::new(),
        }
    }
}

pub(super) fn symbol(run: &mut Run, symbols: &l::Symbols, bytes: &[u8], word: l::Symbol) -> u32 {
    let text = l::text_of(symbols, ByteSlice(bytes), word);
    run.sym(text.map_or(b"", |t| t.0.as_bytes()))
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
