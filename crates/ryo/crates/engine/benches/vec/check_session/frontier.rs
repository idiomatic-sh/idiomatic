//! File-version contributions used by the closed project executor.
use super::{Bound, Program, Result, finish};
use finish::{Run, Table, partition::Partition, population::Block};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    rc::Rc,
};

pub(super) struct FileInputs<'a> {
    columns: BTreeMap<&'a str, Vec<bool>>,
    carried_bases: HashMap<String, Rc<Block>>,
    produced: HashMap<usize, Rc<Block>>,
}
impl<'a> FileInputs<'a> {
    pub(super) fn new(
        active: &[(usize, usize, bool)],
        bound: &super::Bindings<'_>,
        programs: &'a [Result<Program>],
        program_of: &[usize],
        cut_of: &[Option<std::sync::Arc<Partition>>],
    ) -> Self {
        let mut shared_columns = BTreeMap::<&str, Vec<bool>>::new();
        for &(binding, _, _) in active {
            let Some(cut) = &cut_of[binding] else {
                continue;
            };
            let b = bound[&binding].as_ref().expect("active binding");
            let plan = &programs[program_of[binding]].as_ref().unwrap().plan;
            for (&node, columns) in &cut.columns {
                let Some(name) = plan.nodes[node].base() else {
                    continue;
                };
                if b.hosted.iter().any(|(n, _)| n == name) {
                    continue;
                }
                let mask = shared_columns
                    .entry(name)
                    .or_insert_with(|| vec![false; columns.len()]);
                if columns.is_empty() {
                    mask.clear();
                }
                for (a, &b) in mask.iter_mut().zip(columns) {
                    *a |= b;
                }
            }
        }
        Self {
            columns: shared_columns,
            carried_bases: HashMap::new(),
            produced: HashMap::new(),
        }
    }
    pub(super) fn contribute(
        &mut self,
        b: &Bound<'_>,
        exec: &mut finish::Exec<'_>,
        frame: &mut finish::File<'_>,
        run: &mut Run,
        id: u32,
        facts_view: &mut dyn finish::Provider,
        cut: &Partition,
        local_coverage: &BTreeMap<String, Vec<Option<u32>>>,
        world: &mut World,
        carried_symbols: &mut HashMap<u32, u32>,
        reader: Option<usize>,
        words: &HashMap<u32, u32>,
        slot: usize,
    ) -> Result<()> {
        let plan = b.plan;
        let nodes = &plan.nodes;
        let hosted = &b.hosted;
        let shared_base = |n: usize| {
            nodes[n]
                .base()
                .filter(|name| !hosted.iter().any(|(bound, _)| bound.as_str() == *name))
        };
        let outputs = &b.outputs;
        let share = |n: usize| outputs.get(&n).filter(|_| local_coverage.is_empty());
        let absent = b
            .absent
            .as_ref()
            .filter(|(home, _)| *home == id && local_coverage.is_empty())
            .map(|(_, bits)| bits);
        let omitted = |i: usize| absent.is_some_and(|a| a[i]);
        let missing: Vec<_> = cut
            .inputs
            .iter()
            .enumerate()
            .filter(|(i, _)| !omitted(*i))
            .map(|(_, n)| *n)
            .filter(|&n| !share(n).is_some_and(|id| self.produced.contains_key(id)))
            .filter(|&n| !shared_base(n).is_some_and(|name| self.carried_bases.contains_key(name)))
            .collect();
        let tables = {
            let mut derived = BTreeMap::new();
            if !missing.is_empty() {
                let tables = finish::session::gather(
                    exec,
                    run,
                    frame,
                    facts_view,
                    &b.slots,
                    &missing,
                    &b.projections,
                    local_coverage,
                )?;
                for (n, t, projected) in tables {
                    let filtered = b.filters.get(&n).filter(|_| !projected).map(|terms| {
                        let mut rows = Vec::new();
                        for r in 0..t.n {
                            if terms.iter().any(|&(c, w)| t.cols[c][r] == w) {
                                rows.push(r);
                            }
                        }
                        let mut out = Table::retained(t.names.clone(), t.tys.clone(), rows.len());
                        for (dst, src) in out.cols.iter_mut().zip(&t.cols) {
                            dst.extend(rows.iter().map(|&r| src[r]));
                        }
                        out.n = rows.len();
                        out
                    });
                    let columns = shared_base(n)
                        .and_then(|name| self.columns.get(name))
                        .unwrap_or(&cut.columns[&n]);
                    let table = world.carry(
                        run,
                        id,
                        filtered.as_ref().unwrap_or(&t),
                        columns,
                        carried_symbols,
                        reader,
                        words,
                    )?;
                    if let Some(name) = shared_base(n) {
                        self.carried_bases.insert(name.to_owned(), table);
                    } else {
                        if let Some(id) = share(n) { self.produced.insert(*id,table.clone()); }
                        derived.insert(n, table);
                    }
                }
            }
            cut.inputs
                .iter()
                .enumerate()
                .map(|(i, &n)| {
                    if omitted(i) {
                        let t = exec.empty_rc(run, b.slots[n])?;
                        return Ok((n, world.empty_block(&t)));
                    }
                    let t = if let Some(name) = shared_base(n) {
                        self.carried_bases.get(name).cloned()
                    } else {
                        derived.remove(&n).or_else(|| share(n).and_then(|id|self.produced.get(id)).cloned())
                    };
                    Ok((n, t.ok_or("missing local output")?))
                })
                .collect::<Result<Vec<_>>>()?
        };
        world.frontiers.insert(slot, FileFrontier::Rows(tables.into_boxed_slice()));
        Ok(())
    }
}
type Part = (usize, Rc<Block>);
pub(super) type EmptyFrontier = Rc<[Part]>;
enum FileFrontier {
    Rows(Box<[Part]>),
    Empty(EmptyFrontier),
}
impl FileFrontier {
    fn parts(&self) -> &[Part] {
        match self {
            Self::Rows(parts) => parts,
            Self::Empty(parts) => parts,
        }
    }
}
/// A frontier's words cross the reader's symbol pool and its local node number
/// space once. The destination run owns stable file and symbol identities for
/// every request. Positions inside a file remain byte-range pairs.
#[derive(Default)]
pub(crate) struct World {
    pub run: Run,
    pub(super) boundaries: Option<Vec<(usize, usize, usize)>>,
    pub(super) project_words: Vec<HashMap<u32, u32>>,
    pub files: HashMap<String, u32>,
    frontiers: HashMap<usize, FileFrontier>,
    empty_blocks: HashMap<(usize, usize), Rc<finish::population::Block>>,
    pub unavailable: HashMap<usize, BTreeSet<(String, u32)>>,
}

#[cfg(feature = "product-session")]
pub(crate) struct SharedFrontier<'a> {
    slots: HashMap<usize, Vec<(usize, usize)>>,
    blocks: Vec<finish::population::SharedBlock<'a>>,
}
#[cfg(feature = "product-session")]
impl<'a> SharedFrontier<'a> {
    pub(crate) fn new(world: &'a World, pairs: impl Iterator<Item = usize>) -> Self {
        let mut slots = HashMap::new();
        let mut blocks = Vec::new();
        let mut ids = HashMap::new();
        for slot in pairs {
            if let Some(frontier) = world.frontiers.get(&slot) {
                slots.entry(slot).or_insert_with(|| {
                    frontier.parts().iter().map(|(node, block)| {
                        let id = *ids.entry(Rc::as_ptr(block)).or_insert_with(|| {
                            let id = blocks.len();
                            blocks.push(block.share());
                            id
                        });
                        (*node, id)
                    }).collect()
                });
            }
        }
        Self { slots, blocks }
    }

    pub(crate) fn joined(
        &self,
        slots: &[usize],
        cut: &[usize],
        local: &mut HashMap<usize, Rc<finish::population::WorkerBlock>>,
        schemas: &mut finish::population::LocalSchemas,
    ) -> Result<finish::population::Inputs> {
        use finish::population::Population;
        let mut tables: Vec<_> = cut.iter().map(|_| Population::default()).collect();
        for slot in slots {
            let parts = self.slots.get(slot).ok_or("missing file frontier")?;
            if parts.len() != cut.len() {
                return Err("incomplete file frontier".into());
            }
            for ((out, expected), (node, id)) in tables.iter_mut().zip(cut).zip(parts) {
                if node != expected {
                    return Err("unbound column frontier".into());
                }
                let piece = local.entry(*id).or_insert_with(|| self.blocks[*id].local(schemas));
                out.add_worker(piece)?;
            }
        }
        Ok(cut.iter().copied().zip(tables.into_iter().map(Rc::new)).collect())
    }
}

impl World {
    pub(crate) fn frontier_parts(&self, slot: usize) -> Option<&[Part]> {
        self.frontiers.get(&slot).map(FileFrontier::parts)
    }
    pub(crate) fn restore_frontier(&mut self, slot: usize, parts: Vec<Part>) {
        self.frontiers.insert(slot, FileFrontier::Rows(parts.into_boxed_slice()));
    }

    pub fn file(&mut self, path: &str) -> u32 {
        if let Some(&i) = self.files.get(path) {
            return i;
        }
        let i = u32::try_from(self.run.paths.len()).expect("file number");
        std::sync::Arc::make_mut(&mut self.run.paths).push(path.to_owned());
        self.files.insert(path.to_owned(), i);
        i
    }

    fn empty_block(&mut self, t: &Table) -> Rc<Block> {
        let schema = (Rc::as_ptr(&t.names) as usize, Rc::as_ptr(&t.tys) as usize);
        self.empty_blocks.entry(schema).or_insert_with(|| {
            Rc::new(Block::new(0, t.names.clone(), t.tys.clone(), 0, Vec::new()))
        }).clone()
    }
    pub(super) fn insert_empty(
        &mut self, slot: usize, path: &str, binding: &mut Bound<'_>, run: &mut Run, cut: &Partition,
    ) {
        let parts = binding.empty_frontier.get_or_insert_with(|| {
            let tables = finish::session::certificate::empty_frontier(&mut finish::Exec::new(binding.plan), run, &cut.inputs)
                .expect("certified file output schemas");
            tables.iter().map(|(id, t)| (*id, self.empty_block(t))).collect()
        });
        if !parts.is_empty() { self.file(path); }
        self.frontiers.insert(slot, FileFrontier::Empty(parts.clone()));
    }
    pub(super) fn productive(&self, slot: usize) -> bool {
        self.frontiers.get(&slot).is_some_and(|v| v.parts().iter().any(|(_, block)| block.n != 0))
    }
    fn carry(
        &mut self,
        source: &Run,
        home: u32,
        t: &Table,
        needed: &[bool],
        syms: &mut HashMap<u32, u32>,
        reader: Option<usize>,
        words: &HashMap<u32, u32>,
    ) -> Result<Rc<finish::population::Block>> {
        use finish::population::Block;
        if syms.is_empty() {
            if let Some(dictionary) = reader.and_then(|id| self.project_words.get(id)) {
                syms.extend(words.iter().map(|(reason, &word)| (word, dictionary[reason])));
            }
        }
        let origin = self.file(&source.paths[home as usize]);
        if t.n == 0 {
            return Ok(self.empty_block(t));
        }
        let mut columns = Vec::with_capacity(t.cols.len());
        for (c, (src, ty)) in t.cols.iter().zip(t.tys.iter()).enumerate() {
            if needed.get(c) == Some(&false) {
                // An unobserved ordinal stays present in the schema for positional
                // kernels, with no payload to translate, copy or retain.
                columns.push(Vec::new());
                continue;
            }
            let mut dst = Vec::with_capacity(t.n);
            match ty {
                finish::Ty::File => {
                    for &v in src {
                        if v == u64::MAX {
                            dst.push(v);
                            continue;
                        }
                        if v == u64::from(home) {
                            dst.push(u64::from(origin));
                            continue;
                        }
                        let path = source
                            .paths
                            .get(v as usize)
                            .ok_or("unbound file on handoff")?;
                        dst.push(u64::from(self.file(path)));
                    }
                }
                finish::Ty::Node => {
                    for &v in src {
                        if v == u64::MAX {
                            dst.push(v);
                            continue;
                        }
                        let at =
                            u32::try_from(v).map_err(|_| "unbound node origin on file handoff")?;
                        dst.push((u64::from(origin) << 32) | u64::from(at));
                    }
                }
                finish::Ty::Sym => {
                    for &v in src {
                        if v == u64::MAX {
                            dst.push(v);
                            continue;
                        }
                        let v = u32::try_from(v).map_err(|_| "unbound symbol on handoff")?;
                        let sym = if source.absent_word == Some(v) {
                            self.run.absent_word.ok_or("missing public null symbol")?
                        } else if let Some(&s) = syms.get(&v) {
                            s
                        } else {
                            let s = self.run.sym(
                                source
                                    .texts
                                    .get(v as usize)
                                    .ok_or("unbound symbol on handoff")?,
                            );
                            syms.insert(v, s);
                            s
                        };
                        dst.push(u64::from(sym));
                    }
                }
                _ => dst.extend_from_slice(src),
            }
            columns.push(dst);
        }
        Ok(Rc::new(Block::new(
            origin,
            t.names.clone(),
            t.tys.clone(),
            t.n,
            columns,
        )))
    }

    pub fn joined(&self, slots: &[usize], cut: &[usize]) -> Result<finish::population::Inputs> {
        use finish::population::Population;
        let mut tables: Vec<_> = cut.iter().map(|_| Population::default()).collect();
        for slot in slots {
            let parts = self.frontiers.get(slot).ok_or("missing file frontier")?.parts();
            if parts.len() != cut.len() {
                return Err("incomplete file frontier".into());
            }
            for ((out, expected), (node, piece)) in tables.iter_mut().zip(cut).zip(parts) {
                if node != expected {
                    return Err("unbound column frontier".into());
                }
                out.add(piece)?;
            }
        }
        Ok(cut
            .iter()
            .copied()
            .zip(tables.into_iter().map(Rc::new))
            .collect())
    }

    pub fn release(&mut self, slot: usize) {
        self.frontiers.remove(&slot);
        self.unavailable.remove(&slot);
    }
}

/// Slot ordinals survive the join. A repeated block in different request frontiers
/// is transferred once, including the reference to its file and symbol domains.
pub(super) struct Transfer {
    slots: Vec<(usize, Vec<(usize, usize)>)>,
    blocks: Vec<finish::population::CarriedBlock>,
    schemas: finish::population::CarriedSchemas,
    paths: Vec<String>,
    words: Vec<Vec<u8>>,
    prefix: usize,
    boundaries: Vec<(usize, usize, usize)>,
    unavailable: HashMap<usize, BTreeSet<(String, u32)>>,
}

impl World {
    pub(super) fn transfer(mut self, prefix: usize) -> Transfer {
        self.empty_blocks.clear();
        let mut ids = HashMap::new();
        let mut blocks = Vec::new();
        let mut slots = Vec::new();
        for (slot, frontier) in self.frontiers {
            let pieces = frontier.parts()
                .iter().cloned()
                .map(|(node, block)| {
                    let id = *ids.entry(Rc::as_ptr(&block)).or_insert_with(|| {
                        let id = blocks.len();
                        blocks.push(block);
                        id
                    });
                    (node, id)
                })
                .collect();
            slots.push((slot, pieces));
        }
        let mut schemas = finish::population::CarriedSchemas::default();
        Transfer {
            slots,
            blocks: blocks
                .into_iter()
                .map(|b| {
                    Rc::try_unwrap(b)
                        .ok()
                        .expect("worker owns all frontier blocks")
                        .carry(&mut schemas)
                })
                .collect(),
            schemas,
            paths: std::sync::Arc::unwrap_or_clone(self.run.paths),
            prefix,
            boundaries: self.boundaries.unwrap_or_default(),
            words: self
                .run
                .texts
                .into_iter()
                .skip(prefix)
                .map(|s| s.to_vec())
                .collect(),
            unavailable: self.unavailable,
        }
    }

    /// The serial reader visits relative paths lexically. Assign file and symbol
    /// ordinals in that same order, including cross-file references first seen
    /// in another path's frontier. Worker completion order has no effect.
    pub(super) fn receive(&mut self, parts: Vec<Transfer>, sources: &[super::Source]) {
        let mut groups = Vec::new();
        let mut maps = Vec::with_capacity(parts.len());
        for (worker, part) in parts.iter().enumerate() {
            let mut file = 0;
            let mut word = part.prefix;
            for &(source, files, words) in &part.boundaries {
                groups.push((
                    &sources[source].path,
                    worker,
                    file..files,
                    (word - part.prefix)..(words - part.prefix),
                ));
                file = files;
                word = words;
            }
            maps.push((
                Vec::with_capacity(file),
                (0..part.prefix as u32).collect::<Vec<_>>(),
            ));
        }
        groups.sort_unstable_by(|a, b| a.0.cmp(b.0));
        for (_, worker, file, word) in groups {
            let (files, symbols) = &mut maps[worker];
            let part = &parts[worker];
            symbols.extend(part.words[word].iter().map(|w| self.run.sym(w)));
            files.extend(part.paths[file].iter().map(|p| self.file(p)));
        }
        for (part, (files, symbols)) in parts.into_iter().zip(maps) {
            self.receive_blocks(part, &files, &symbols);
        }
    }

    fn receive_blocks(&mut self, part: Transfer, files: &[u32], symbols: &[u32]) {
        let schemas = part.schemas.receive();
        let blocks: Vec<_> = part
            .blocks
            .into_iter()
            .map(|b| Rc::new(b.receive(files, symbols, &schemas)))
            .collect();
        for (slot, ids) in part.slots {
            self.frontiers.insert(
                slot,
                FileFrontier::Rows(ids.into_iter()
                    .map(|(n, i)| (n, blocks[i].clone()))
                    .collect()),
            );
        }
        for (slot, set) in part.unavailable {
            self.unavailable.insert(
                slot,
                set.into_iter()
                    .map(|(name, file)| (name, files[file as usize]))
                    .collect(),
            );
        }
    }
}
