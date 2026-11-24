//! Immutable file contributions in admission order. A lookup in a node column
//! addresses one source file. A scan materializes the same rows in file order.
use super::{Names, R, Table, Ty, Tys};
use rustc_hash::FxHashMap;
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
    sync::Arc,
};

/// File-local row numbers and identifiers typically occupy only part of a word.
/// All representations decode to the exact canonical u64, including public nulls.
enum Words {
    Constant(u64),
    Sequence(u64),
    Byte(u64, Box<[u8]>),
    Short(u64, Box<[u16]>),
    Int(u64, Box<[u32]>),
    Full(Box<[u64]>),
    Stored(StoredWords),
}

// The snapshot owns the encoded bytes. Blocks hold ranges into that immutable
// allocation and rebind only file and symbol IDs as a join visits a value.
pub(crate) type BlockBytes = Arc<dyn AsRef<[u8]> + Send + Sync>;

pub(crate) struct StoredValues {
    pub(crate) bytes: BlockBytes,
    pub(crate) dictionary: Vec<u32>,
    pub(crate) absent: Option<u32>,
}

pub(crate) struct StoredWords {
    pub(crate) values: Arc<StoredValues>,
    pub(crate) start: u32,
    pub(crate) width: u8,
    pub(crate) base: u64,
    pub(crate) ty: Ty,
}
impl StoredWords {
    fn at(&self, i: usize) -> u64 {
        let width = usize::from(self.width);
        let offset = self.start as usize + i * width;
        let bytes = &self.values.bytes.as_ref().as_ref()[offset..offset + width];
        let delta = match self.width {
            0 => i as u64,
            1 => u64::from(bytes[0]),
            2 => u64::from(u16::from_le_bytes(bytes.try_into().unwrap())),
            4 => u64::from(u32::from_le_bytes(bytes.try_into().unwrap())),
            8 => u64::from_le_bytes(bytes.try_into().unwrap()),
            _ => unreachable!("validated column width"),
        };
        self.values.translate(self.ty, self.base + delta)
    }
}

impl StoredValues {
    pub(crate) fn translate(&self, ty: Ty, word: u64) -> u64 {
        if word == super::NULL {
            return word;
        }
        match ty {
            Ty::Node => {
                (u64::from(self.dictionary[(word >> 32) as usize]) << 32) | (word & 0xffff_ffff)
            }
            Ty::Sym if word == u64::MAX - 1 => {
                u64::from(self.absent.expect("declared absent word"))
            }
            Ty::Sym | Ty::File => u64::from(self.dictionary[word as usize]),
            _ => word,
        }
    }
}

impl Words {
    fn pack(v: Vec<u64>) -> Self {
        let first = v.first().copied().unwrap_or(super::NULL);
        if v.iter().all(|&x| x == first) {
            return Self::Constant(first);
        }
        if v.iter()
            .enumerate()
            .all(|(i, &x)| first.checked_add(i as u64) == Some(x))
        {
            return Self::Sequence(first);
        }
        let (lo, hi) = v
            .iter()
            .fold((first, first), |(lo, hi), &x| (lo.min(x), hi.max(x)));
        match hi - lo {
            0..=255 => Self::Byte(lo, v.into_iter().map(|x| (x - lo) as u8).collect()),
            256..=65535 => Self::Short(lo, v.into_iter().map(|x| (x - lo) as u16).collect()),
            65536..=4294967295 => Self::Int(lo, v.into_iter().map(|x| (x - lo) as u32).collect()),
            _ => Self::Full(v.into_boxed_slice()),
        }
    }

    #[inline]
    fn at(&self, i: usize) -> u64 {
        match self {
            Self::Constant(x) => *x,
            Self::Sequence(x) => x + i as u64,
            Self::Byte(x, v) => x + u64::from(v[i]),
            Self::Short(x, v) => x + u64::from(v[i]),
            Self::Int(x, v) => x + u64::from(v[i]),
            Self::Full(v) => v[i],
            Self::Stored(v) => v.at(i),
        }
    }

    fn append(&self, n: usize, to: &mut Vec<u64>) {
        match self {
            Self::Constant(x) => to.resize(to.len() + n, *x),
            Self::Sequence(x) => to.extend((0..n).map(|i| x + i as u64)),
            Self::Byte(x, v) => to.extend(v.iter().map(|&i| x + u64::from(i))),
            Self::Short(x, v) => to.extend(v.iter().map(|&i| x + u64::from(i))),
            Self::Int(x, v) => to.extend(v.iter().map(|&i| x + u64::from(i))),
            Self::Full(v) => to.extend_from_slice(v),
            Self::Stored(v) => to.extend((0..n).map(|i| v.at(i))),
        }
    }
}

/// A posting lists the source row ordinals in their original order.
enum Posting {
    Ordered,
    Sorted(Box<[usize]>),
}

pub(crate) struct Block {
    pub(crate) names: Names,
    pub(crate) tys: Tys,
    pub(crate) n: usize,
    pub(crate) home: u32,
    columns: Vec<Words>,
    postings: RefCell<FxHashMap<usize, Posting>>,
}

pub(crate) enum FixedColumn {
    Constant(u64),
    Sequence(u64),
    Values { base: u64, width: u8 },
}

impl Block {
    pub(crate) fn constant(&self, column: usize) -> Option<u64> {
        match self.columns[column] {
            Words::Constant(value) => Some(value),
            _ => None,
        }
    }
    pub(crate) fn words(&self,col:usize)->Vec<u64>{
        let mut out=Vec::with_capacity(self.n);
        self.columns[col].append(self.n,&mut out);
        out
    }
    pub(crate) fn new(home: u32, names: Names, tys: Tys, n: usize, columns: Vec<Vec<u64>>) -> Self {
        Self {
            home,
            names,
            tys,
            n,
            columns: columns.into_iter().map(Words::pack).collect(),
            postings: Default::default(),
        }
    }

    // Untranslated numeric columns already have the stable fixed-width form.
    pub(crate) fn append_fixed(&self, column: usize, out: &mut Vec<u8>) -> Option<FixedColumn> {
        let (base, width) = match &self.columns[column] {
            Words::Constant(word) => return Some(FixedColumn::Constant(*word)),
            Words::Sequence(base) => return Some(FixedColumn::Sequence(*base)),
            Words::Byte(base, values) => {
                out.extend_from_slice(values);
                (*base, 1)
            }
            Words::Short(base, values) => {
                out.extend(values.iter().flat_map(|v| v.to_le_bytes()));
                (*base, 2)
            }
            Words::Int(base, values) => {
                out.extend(values.iter().flat_map(|v| v.to_le_bytes()));
                (*base, 4)
            }
            Words::Full(values) => {
                out.extend(values.iter().flat_map(|v| v.to_le_bytes()));
                (0, 8)
            }
            Words::Stored(_) => return None,
        };
        Some(FixedColumn::Values { base, width })
    }

    pub(crate) fn stored(
        home: u32,
        names: Names,
        tys: Tys,
        n: usize,
        columns: Vec<Result<StoredWords, u64>>,
    ) -> Self {
        Self {
            home,
            names,
            tys,
            n,
            columns: columns.into_iter().map(|c| match c {
                Ok(words) => Words::Stored(words),
                Err(constant) => Words::Constant(constant),
            }).collect(),
            postings: Default::default(),
        }
    }

    fn find(&self, col: usize, key: u64, mut hit: impl FnMut(usize) -> bool) -> bool {
        let words = &self.columns[col];
        match words {
            Words::Constant(k) => {
                return key == *k && (0..self.n).any(hit);
            }
            Words::Sequence(first) => {
                if let Some(i) = key
                    .checked_sub(*first)
                    .and_then(|v| usize::try_from(v).ok())
                    && i < self.n
                {
                    return hit(i);
                }
                return false;
            }
            _ => {}
        }
        let mut indices = self.postings.borrow_mut();
        let posting = indices.entry(col).or_insert_with(|| {
            if (1..self.n).all(|i| words.at(i - 1) <= words.at(i)) {
                return Posting::Ordered;
            }
            let mut ids: Vec<_> = (0..self.n).collect();
            ids.sort_unstable_by_key(|&r| (words.at(r as usize), r));
            Posting::Sorted(ids.into_boxed_slice())
        });
        let ordinal = |i: usize| match &*posting {
            Posting::Ordered => i,
            Posting::Sorted(v) => v[i] as usize,
        };
        let (mut lo, mut hi) = (0, self.n);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if words.at(ordinal(mid)) < key {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        for i in lo..self.n {
            let r = ordinal(i);
            if words.at(r) != key {
                break;
            }
            if hit(r) {
                return true;
            }
        }
        false
    }
}

#[derive(Default)]
pub(crate) struct Population {
    schema: Option<(Names, Tys)>,
    pieces: Vec<Rc<Block>>,
    by_file: OnceCell<FxHashMap<u32, Vec<usize>>>,
    pub(crate) n: usize,
    local_files: OnceCell<Vec<bool>>,
    materialized: OnceCell<Rc<Table>>,
}

pub(crate) type Inputs = std::collections::BTreeMap<usize, Rc<Population>>;

impl Population {
    pub(crate) fn add(&mut self, t: &Rc<Block>) -> R<()> {
        let (names, tys) = self
            .schema
            .get_or_insert_with(|| (t.names.clone(), t.tys.clone()));
        if *names != t.names || *tys != t.tys {
            return Err("file frontier schema differs between providers".into());
        }
        if t.n != 0 {
            self.n = self
                .n
                .checked_add(t.n)
                .ok_or("closed population exceeds address space")?;
            self.pieces.push(t.clone());
        }
        Ok(())
    }

    pub(crate) fn agrees(&self, names: &Names, tys: &Tys) -> bool {
        self.schema
            .as_ref()
            .is_none_or(|(n, t)| n == names && t == tys)
    }

    pub(super) fn full(&self, names: &Names, tys: &Tys) -> Rc<Table> {
        self.materialized
            .get_or_init(|| {
                let mut out = Table::retained(names.clone(), tys.clone(), self.n);
                for block in &self.pieces {
                    for (from, to) in block.columns.iter().zip(&mut out.cols) {
                        from.append(block.n, to);
                    }
                }
                out.n = self.n;
                Rc::new(out)
            })
            .clone()
    }

    pub(super) fn is_materialized(&self, t: &Rc<Table>) -> bool {
        self.materialized
            .get()
            .is_some_and(|owned| Rc::ptr_eq(owned, t))
    }

    pub(super) fn can_probe(&self, col: usize) -> bool {
        let Some((_, tys)) = &self.schema else {
            return true;
        };
        if tys[col] == Ty::Node {
            return true;
        }
        if tys[col] != Ty::File {
            return false;
        }
        self.local_files.get_or_init(|| {
            tys.iter()
                .enumerate()
                .map(|(c, ty)| {
                    *ty==Ty::File && self.pieces.iter().all(|t| {
                matches!(t.columns[c],Words::Constant(x) if x==u64::from(t.home) || x==super::NULL)
            })
                })
                .collect()
        })[col]
    }

    fn find(
        &self,
        col: usize,
        key: u64,
        mut visit: impl FnMut(usize, &Block, usize) -> bool,
    ) -> bool {
        let Some((_, tys)) = &self.schema else {
            return false;
        };
        if key != super::NULL && self.can_probe(col) {
            let homes = self.by_file.get_or_init(|| {
                let mut homes = FxHashMap::<u32, Vec<usize>>::default();
                for (i, t) in self.pieces.iter().enumerate() {
                    homes.entry(t.home).or_default().push(i);
                }
                homes
            });
            let file = if tys[col] == Ty::Node { key >> 32 } else { key };
            let Ok(file) = u32::try_from(file) else {
                return false;
            };
            if let Some(pieces) = homes.get(&file) {
                for &i in pieces {
                    let t = &self.pieces[i];
                    if t.find(col, key, |r| visit(i, t, r)) {
                        return true;
                    }
                }
            }
        } else {
            for (i, t) in self.pieces.iter().enumerate() {
                if t.find(col, key, |r| visit(i, t, r)) {
                    return true;
                }
            }
        }
        false
    }

    fn each_match(&self, col: usize, key: u64, mut each: impl FnMut(usize, &Block, usize)) {
        self.find(col, key, |i, t, r| {
            each(i, t, r);
            false
        });
    }

    pub(super) fn contains_keys(
        &self,
        key: usize,
        left: &Table,
        row: usize,
        lk: &[usize],
        rk: &[usize],
    ) -> bool {
        self.find(rk[key], left.cols[lk[key]][row], |_, t, r| {
            lk.iter()
                .zip(rk)
                .all(|(&lc, &rc)| t.columns[rc].at(r) == left.cols[lc][row])
        })
    }

    pub(super) fn neighbors(&self, by: usize, to: usize, key: u64, out: &mut Vec<u64>) {
        self.each_match(by, key, |_, block, row| out.push(block.columns[to].at(row)));
    }

    /// Match the table index: probe keys in caller order, then source order within each key.
    pub(super) fn probe(&self, names: &Names, tys: &Tys, col: usize, keys: &[u64]) -> Rc<Table> {
        let mut rows = Vec::new();
        for &key in keys {
            self.each_match(col, key, |i, _, r| rows.push((i, r)));
        }
        let mut out = Table::retained(names.clone(), tys.clone(), rows.len());
        for (c, to) in out.cols.iter_mut().enumerate() {
            to.extend(rows.iter().map(|&(i, r)| self.pieces[i].columns[c].at(r)));
        }
        out.n = rows.len();
        Rc::new(out)
    }
}

/// A completed file block can leave its worker after all local references are released.
/// Lookup postings are built only by the receiving project executor.
pub(crate) struct CarriedBlock {
    schema: usize,
    rows: usize,
    home: u32,
    columns: Vec<Words>,
}

/// File tables in one worker share column declarations. Retain one owned copy
/// for the join even when many frontier blocks use the same declaration.
#[derive(Default)]
pub(crate) struct CarriedSchemas {
    ids: FxHashMap<(usize, usize), usize>,
    schemas: Vec<(Vec<String>, Vec<Ty>)>,
}
impl CarriedSchemas {
    fn intern(&mut self, names: Names, types: Tys) -> usize {
        let key = (Rc::as_ptr(&names) as usize, Rc::as_ptr(&types) as usize);
        *self.ids.entry(key).or_insert_with(|| {
            let id = self.schemas.len();
            self.schemas
                .push((Rc::unwrap_or_clone(names), Rc::unwrap_or_clone(types)));
            id
        })
    }
    pub(crate) fn receive(self) -> Vec<(Names, Tys)> {
        self.schemas
            .into_iter()
            .map(|(n, t)| (Rc::new(n), Rc::new(t)))
            .collect()
    }
}
impl Block {
    pub(crate) fn carry(self, shapes: &mut CarriedSchemas) -> CarriedBlock {
        CarriedBlock {
            schema: shapes.intern(self.names, self.tys),
            rows: self.n,
            home: self.home,
            columns: self.columns,
        }
    }
}

impl CarriedBlock {
    pub(crate) fn receive(
        mut self,
        files: &[u32],
        symbols: &[u32],
        schemas: &[(Names, Tys)],
    ) -> Block {
        let (names, types) = &schemas[self.schema];
        for (words, ty) in self.columns.iter_mut().zip(types.iter()) {
            if !matches!(ty, Ty::File | Ty::Node | Ty::Sym) {
                continue;
            }
            let remap = |v: u64| {
                if v == u64::MAX {
                    return v;
                }
                match ty {
                    Ty::File => u64::from(files[v as usize]),
                    Ty::Node => {
                        (u64::from(files[(v >> 32) as usize]) << 32) | (v & u64::from(u32::MAX))
                    }
                    Ty::Sym => u64::from(symbols[v as usize]),
                    _ => unreachable!(),
                }
            };
            if let Words::Constant(value) = words {
                *value = remap(*value);
            } else {
                *words = Words::pack((0..self.rows).map(|i| remap(words.at(i))).collect());
            }
        }
        Block {
            names: names.clone(),
            tys: types.clone(),
            n: self.rows,
            home: if self.rows == 0 {
                0
            } else {
                files[self.home as usize]
            },
            columns: self.columns,
            postings: Default::default(),
        }
    }
}
