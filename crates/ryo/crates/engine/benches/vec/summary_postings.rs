//! Invert the pack's symbol equality tests by (fact, column). A demanded
//! column is scanned once. Only symbols used in a test on that column receive
//! postings. Position buffers and single-symbol masks are shared by all uses.
use super::*;
use std::ops::Range;

struct Column {
    slot: usize,
    col: usize,
    by_word: Vec<u32>,
}
struct Test {
    column: usize,
    lists: Range<usize>,
}

pub(crate) struct Layout {
    columns: Vec<Column>,
    tests: Vec<Option<Test>>,
    uses: Vec<usize>,
    count: usize,
}
/// Request predicates refer to the lists already maintained for the pair checks.
#[derive(Default)]
pub(crate) struct RequestUses(Vec<(u32, usize, Box<[usize]>)>);

impl RequestUses {
    pub(crate) fn bind(&mut self, layout: &Layout, p: &Program, guard: u32,
        name: &str, col: usize, words: impl Iterator<Item = u64>) {
        let Some((i, indexed)) = layout.columns.iter().enumerate()
            .find(|(_, c)| c.col == col && p.facts[c.slot].name == name && matches!(p.facts[c.slot].source, Source::Catalog(_)))
        else { return };
        let lists: Option<Box<[usize]>> = words.map(|w| {
            let list = indexed.by_word.get(usize::try_from(w).ok()?)?.checked_sub(1)?;
            Some(list as usize)
        }).collect();
        if let Some(lists) = lists { self.0.push((guard, i, lists)); }
    }

    pub(crate) fn absent(&self, scratch: &Scratch, out: &mut Vec<u32>) {
        for (guard, col, lists) in &self.0 {
            if scratch.postings.loaded[*col] == Some(true)
                && lists.iter().all(|&p| scratch.postings.positions[p].is_empty())
            {
                out.push(*guard);
            }
        }
    }
}

impl Layout {
    pub(crate) fn new(p: &Program, codes: &[Option<Id>]) -> Self {
        let mut columns = Vec::<Column>::new();
        let mut locations = Map::default();
        let mut postings = Map::default();
        let mut tests = Vec::with_capacity(p.domains.len());
        let mut uses = Vec::new();
        for domain in &p.domains {
            let Domain::Fact(slot, Some((col, symbols))) = domain else {
                tests.push(None);
                continue;
            };
            let column = *locations.entry((*slot, *col)).or_insert_with(|| {
                let at = columns.len();
                columns.push(Column {
                    slot: *slot,
                    col: *col,
                    by_word: vec![0; codes.len()],
                });
                at
            });
            let start = uses.len();
            for &symbol in symbols {
                let next = postings.len();
                let post = *postings.entry((column, symbol)).or_insert(next);
                uses.push(post);
            }
            tests.push(Some(Test {
                column,
                lists: start..uses.len(),
            }));
        }
        for column in 0..columns.len() {
            for (word, &symbol) in codes.iter().enumerate() {
                if let Some(&list) = symbol.as_ref().and_then(|s| postings.get(&(column, *s))) {
                    columns[column].by_word[word] =
                        u32::try_from(list + 1).expect("query posting fits u32");
                }
            }
        }
        Self {
            columns,
            tests,
            uses,
            count: postings.len(),
        }
    }
}
#[derive(Default)]
pub(super) struct Cache {
    loaded: Vec<Option<bool>>,
    positions: Vec<Vec<u32>>,
    masks: Vec<Option<Bits>>,
}
impl Cache {
    pub(super) fn reset(&mut self, plan: &Layout) {
        self.loaded.clear();
        self.loaded.resize(plan.columns.len(), None);
        self.positions.resize_with(plan.count, Vec::new);
        for positions in &mut self.positions {
            positions.clear();
        }
        self.masks.clear();
        self.masks.resize(plan.count, None);
    }
}

impl<const PROFILE: bool> File<'_, PROFILE> {
    pub(super) fn posted(&mut self, id: Id, first: bool) -> Nodes {
        let plan = &self.order.postings;
        let test = plan.tests[id].as_ref().expect("compiled symbol predicate");
        let col = &plan.columns[test.column];
        let table = &mut self.facts[col.slot];
        let cache = &mut *self.postings;
        if cache.loaded[test.column].is_none() {
            let spec = &self.p.facts[col.slot];
            table.load(
                spec,
                node_table::element_count(self.t).0 as u32,
                self.words,
                self.read,
            );
            cache.loaded[test.column] = Some(table.complete);
            if table.complete {
                let keys = spec.key.and_then(|k| table.cells.0.get(k));
                let end = languages::facts::words_end(self.words).0;
                if let Some(values) = table.cells.0.get(col.col) {
                    for (row, cell) in values.0.iter().enumerate() {
                        // Tagged cells render as the dictionary's empty word.
                        // Dynamic text that was a query constant is interned
                        // in the fixed prefix and cannot occur in the tail.
                        let word = if cell.0 >= end { 0 } else { cell.0 as usize };
                        let index = col.by_word.get(word).copied().unwrap_or(0);
                        if index != 0 {
                            cache.positions[index as usize - 1]
                                .push(keys.map_or(0, |c| c.0[row].0 as u32));
                        }
                    }
                }
            }
        }
        if cache.loaded[test.column] != Some(true) {
            return None;
        }
        let lists = &plan.uses[test.lists.clone()];
        if first {
            let found = lists.iter().any(|&i| !cache.positions[i].is_empty());
            return if found { None } else { Some(Bits::default()) };
        }
        if let [index] = *lists {
            let bits = *cache.masks[index]
                .get_or_insert_with(|| self.arena.bits(cache.positions[index].iter().copied()));
            return Some(bits);
        }
        Some(
            self.arena.bits(
                lists
                    .iter()
                    .flat_map(|&i| cache.positions[i].iter().copied()),
            ),
        )
    }
}
