//! Adapt one SDK row packet and its actual syntax text to the shared pair program.
use super::super::{
    finish, first,
    pair_checks::SyntaxText,
    pair_summary::{Program, Source},
};
use super::packet::Packet;
use rustc_hash::FxHashMap;
use ryo::languages::{self as l, FileWords};

pub(super) fn vocabulary(kinds: &[l::NameText<'_>], fields: l::FieldNames) -> first::Vocab {
    fn codes(names: &[l::NameText<'_>]) -> std::collections::HashMap<String, u16> {
        let mut codes = std::collections::HashMap::new();
        let mut ambiguous = std::collections::HashSet::new();
        for (i, name) in names.iter().enumerate() {
            if codes
                .insert(
                    name.0.to_owned(),
                    u16::try_from(i).expect("vocabulary code"),
                )
                .is_some()
            {
                ambiguous.insert(name.0);
            }
        }
        codes.retain(|name, _| !ambiguous.contains(name.as_str()));
        codes
    }
    first::Vocab {
        kinds: codes(kinds),
        kind_names: kinds.iter().map(|n| n.0.to_owned()).collect(),
        fields: codes(&fields.0),
        field_names: fields.0.iter().map(|n| n.0.to_owned()).collect(),
    }
}

pub(super) struct Decisions {
    pub(super) program: Program,
    roots: Vec<usize>,
    words: FileWords,
    order: super::super::summary_schedule::Schedule,
    scratch: super::super::pair_checks::Scratch,
    pub(super) steps: u64,
}
impl Decisions {
    pub(super) fn new(program: Program, plan: &finish::Plan) -> Self {
        let index: FxHashMap<_, _> = program
            .roots
            .iter()
            .enumerate()
            .map(|(i, (name, _))| (name.as_str(), i))
            .collect();
        let roots = plan
            .emits
            .iter()
            .map(|emit| index[emit.0.as_str()])
            .collect();
        let (words, codes) = super::super::summary_values::register_symbols(&program, &[]);
        let order = super::super::summary_schedule::Schedule::new(&program, None, codes);
        Self {
            program,
            roots,
            words,
            order,
            scratch: Default::default(),
            steps: 0,
        }
    }
    pub(super) fn filter(
        &mut self,
        wanted: &mut Vec<usize>,
        table: &l::NodeTable,
        text: SyntaxText<'_>,
        bytes: &[u8],
        path: &str,
        elements: &mut l::Elements,
        symbols: &mut l::Symbols,
        id: l::FileId,
        requested: &[l::Relation],
        packet: &mut Packet,
        mut walk: impl FnMut(&mut l::Elements, &mut l::Symbols, &mut l::contract::Collect),
    ) {
        let n = text.ids.len();
        let mut live = vec![false; self.program.roots.len()];
        for &i in wanted.iter() {
            live[self.roots[i]] = true;
        }
        self.order.literals.syntax(text.ids, text.symbols, &mut self.scratch.literals);
        self.steps += self.program.eval(
            &mut self.scratch,
            &self.order,
            table,
            bytes,
            Some(path.as_bytes()),
            Some(text),
            &mut self.words,
            |source, query, words, out| {
                if !matches!(query, super::super::pair_checks::Query::Rows) { return None; }
                let Source::Catalog(relation) = source else {
                    return Some(false);
                };
                if !requested.contains(&relation) {
                    return Some(false);
                }
                if !packet.ready {
                    walk(elements, symbols, &mut packet.rows);
                    packet.index(requested);
                }
                Some(packet.columns(relation, symbols, bytes, id, n, words, out))
            },
            &mut live,
            id,
            None,
        );
        wanted.retain(|&i| live[self.roots[i]]);
    }
}
