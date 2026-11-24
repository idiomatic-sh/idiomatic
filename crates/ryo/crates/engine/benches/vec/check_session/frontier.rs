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
}
impl<'a> FileInputs<'a> {
    pub(super) fn new(
        active: &[(usize, usize, bool)],
        bound: &HashMap<usize, Result<Bound<'_>>>,
        programs: &'a [Result<Program>],
        program_of: &[usize],
        cut_of: &[Option<Rc<Partition>>],
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
        let missing: Vec<_> = cut
            .inputs
            .iter()
            .copied()
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
                    local_coverage,
                )?;
                for (n, t) in tables {
                    let filtered = b.filters.get(&n).map(|terms| {
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
                        derived.insert(n, table);
                    }
                }
            }
            cut.inputs
                .iter()
                .map(|&n| {
                    let t = if let Some(name) = shared_base(n) {
                        self.carried_bases.get(name).cloned()
                    } else {
                        derived.remove(&n)
                    };
                    Ok((n, t.ok_or("missing local output")?))
                })
                .collect::<Result<Vec<_>>>()?
        };
        world.frontiers.insert(slot, tables);
        Ok(())
    }
}
/// A frontier's words cross the reader's symbol pool and its local node number
/// space once. The destination run owns stable file and symbol identities for
/// every request. Positions inside a file remain byte-range pairs.
#[derive(Default)]
pub(crate) struct World {
    pub run: Run,
    pub(super) project_words: Vec<HashMap<u32, u32>>,
    pub files: HashMap<String, u32>,
    frontiers: HashMap<usize, Vec<(usize, Rc<finish::population::Block>)>>,
    empty_blocks: HashMap<(usize, usize), Rc<finish::population::Block>>,
    pub unavailable: HashMap<usize, BTreeSet<(String, u32)>>,
}

impl World {
    pub fn file(&mut self, path: &str) -> u32 {
        if let Some(&i) = self.files.get(path) {
            return i;
        }
        let i = u32::try_from(self.run.paths.len()).expect("file number");
        self.run.paths.push(path.to_owned());
        let sym = self.run.sym(path.as_bytes());
        self.run.path_sym.push(sym);
        self.files.insert(path.to_owned(), i);
        i
    }

    fn empty_block(&mut self, t: &Table) -> Rc<Block> {
        let schema = (Rc::as_ptr(&t.names) as usize, Rc::as_ptr(&t.tys) as usize);
        self.empty_blocks.entry(schema).or_insert_with(|| {
            Rc::new(Block::new(0, t.names.clone(), t.tys.clone(), 0, Vec::new()))
        }).clone()
    }
    pub(super) fn insert_empty(&mut self, slot: usize, path: &str, tables: &[(usize, Rc<Table>)]) {
        if !tables.is_empty() { self.file(path); }
        let blocks = tables.iter().map(|(i, t)| (*i, self.empty_block(t))).collect();
        self.frontiers.insert(slot, blocks);
    }
    pub(super) fn productive(&self, slot: usize) -> bool {
        self.frontiers.get(&slot).is_some_and(|v| v.iter().any(|(_, block)| block.n != 0))
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
            let parts = self.frontiers.get(slot).ok_or("missing file frontier")?;
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
