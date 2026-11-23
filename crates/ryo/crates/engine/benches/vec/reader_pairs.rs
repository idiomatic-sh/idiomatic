//! Adapt one SDK row packet and its actual syntax text to the shared pair program.
use super::super::{
    finish, first,
    pair_checks::SyntaxText,
    pair_summary::{Program, Source},
};
use rustc_hash::FxHashMap;
use ryo::languages::{self as l, ByteSlice, CellKind, CellWord, FileWords, FilledColumns};
use ryo_relations::domain::relations::service as catalog;

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

#[derive(Default)]
pub(super) struct Packet {
    pub rows: l::contract::Collect,
    offsets: FxHashMap<l::Relation, Vec<usize>>,
    pub ready: bool,
}

impl Packet {
    pub fn new(program: &Program) -> Self {
        let offsets = program
            .facts
            .iter()
            .filter_map(|fact| match fact.source {
                Source::Catalog(rel) => Some((rel, Vec::new())),
                _ => None,
            })
            .collect();
        Self {
            offsets,
            ..Self::default()
        }
    }
    pub fn clear(&mut self) {
        self.ready = false;
        self.rows.clear();
        self.offsets.values_mut().for_each(Vec::clear);
    }
    fn index(&mut self) {
        let mut at = 0;
        for relation in &self.rows.relations {
            if let Some(offsets) = self.offsets.get_mut(relation) {
                offsets.push(at);
            }
            at += catalog::kinds(*relation).len();
        }
        debug_assert_eq!(at, self.rows.words.len());
    }
    fn columns(
        &self,
        relation: l::Relation,
        symbols: &l::Symbols,
        bytes: &[u8],
        id: l::FileId,
        n: usize,
        words: &mut FileWords,
        out: &mut FilledColumns,
    ) -> bool {
        if self.rows.partials.iter().any(|(r, _, _)| *r == relation)
            || self.rows.partials_at.iter().any(|(r, _, _)| *r == relation)
        {
            return false;
        }
        let Some(starts) = self.offsets.get(&relation) else {
            return false;
        };
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
                        let text = l::text_of(symbols, ByteSlice(bytes), l::Symbol(raw as u32))
                            .unwrap_or(l::NameText(""));
                        l::facts::intern(words, text)
                    }
                    _ => CellWord(raw),
                };
                column.0.push(cell);
            }
        }
        true
    }
}

pub(super) struct Decisions {
    pub program: Program,
    pub roots: Vec<usize>,
    words: FileWords,
    native_only: bool,
    pub steps: u64,
}
impl Decisions {
    pub fn new(program: Program, plan: &finish::Plan) -> Self {
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
        Self {
            program,
            roots,
            words: l::facts::file_words(&l::facts::base_words(Default::default(), &[])),
            native_only: std::env::var_os("VEC_READER_PAIR_NATIVE").is_some(),
            steps: 0,
        }
    }
    pub fn filter(
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
        self.steps += self.program.file_columns(
            table,
            bytes,
            Some(path.as_bytes()),
            Some(text),
            &mut self.words,
            |source, words, out| {
                let Source::Catalog(relation) = source else {
                    return false;
                };
                if self.native_only || !requested.contains(&relation) {
                    return false;
                }
                if !packet.ready {
                    walk(elements, symbols, &mut packet.rows);
                    packet.index();
                    packet.ready = true;
                }
                packet.columns(relation, symbols, bytes, id, n, words, out)
            },
            &mut live,
            None,
        );
        wanted.retain(|&i| live[self.roots[i]]);
    }
}
