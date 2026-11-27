//! Immutable file contributions in admission order. A lookup in a node column
//! addresses one source file. A scan materializes the same rows in file order.
use super::{Names, R, Table, Ty, Tys};
use rustc_hash::FxHashMap;
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
    sync::Arc,
};


pub struct SharedWords<'a>(Vec<(&'a [u8], bool)>);

impl super::Run {
    pub fn share_words(&self) -> SharedWords<'_> {
        SharedWords(self.texts.iter().enumerate().map(|(slot, word)| {
            // The absent and unresolved words deliberately occupy distinct
            // slots even if their spelling matches an interned string.
            (word.as_ref(), self.ids.get(word.as_ref()) == Some(&(slot as u32)))
        }).collect())
    }
}

impl SharedWords<'_> {
    pub fn install(&self, run: &mut super::Run) {
        run.texts.reserve(self.0.len());
        for (slot, &(bytes, interned)) in self.0.iter().enumerate() {
            let word = Rc::<[u8]>::from(bytes);
            if interned {
                run.ids.insert(word.clone(), slot as u32);
            }
            run.texts.push(word);
        }
    }
}

/// File-local row numbers and identifiers typically occupy only part of a word.
/// All representations decode to the exact canonical u64, including public nulls.
#[derive(Clone)]
pub enum Words {
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
pub type BlockBytes = Arc<dyn AsRef<[u8]> + Send + Sync>;

pub struct StoredValues {
    pub bytes: BlockBytes,
    pub dictionary: Vec<u32>,
    pub absent: Option<u32>,
}

#[derive(Clone)]
pub struct StoredWords {
    pub values: Arc<StoredValues>,
    pub start: u32,
    pub width: u8,
    pub base: u64,
    pub ty: Ty,
}
impl StoredWords {
    pub fn at(&self, i: usize) -> u64 {
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
    pub fn translate(&self, ty: Ty, word: u64) -> u64 {
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
    pub fn pack(v: std::borrow::Cow<'_, [u64]>) -> Self {
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
            0..=255 => Self::Byte(lo, v.iter().map(|&x| (x - lo) as u8).collect()),
            256..=65535 => Self::Short(lo, v.iter().map(|&x| (x - lo) as u16).collect()),
            65536..=4294967295 => Self::Int(lo, v.iter().map(|&x| (x - lo) as u32).collect()),
            _ => Self::Full(v.into_owned().into_boxed_slice()),
        }
    }

    #[inline]
    pub fn at(&self, i: usize) -> u64 {
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

    pub fn append(&self, n: usize, to: &mut Vec<u64>) {
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
pub enum Posting {
    Ordered,
    Sorted(Box<[usize]>),
}

pub struct ColumnBlock<C> {
    pub names: Names,
    pub tys: Tys,
    pub n: usize,
    pub home: u32,
    pub columns: C,
    pub postings: RefCell<FxHashMap<usize, Posting>>,
}

pub type Block = ColumnBlock<Vec<Words>>;

pub type WorkerBlock = ColumnBlock<Arc<[Words]>>;

/// The immutable cells and schema can outlive their original project executor.
pub struct SharedBlock<'a> {
    pub block: Arc<[Words]>,
    pub schema: (usize, usize),
    pub names: &'a [String],
    pub types: &'a [Ty],
    pub len: usize,
    pub home: u32,
}
impl SharedBlock<'_> {
    pub fn local(&self, schemas: &mut LocalSchemas) -> Rc<WorkerBlock> {
        let (names, tys) = schemas.0.entry(self.schema).or_insert_with(|| {
            (Rc::new(self.names.to_vec()), Rc::new(self.types.to_vec()))
        });
        Rc::new(ColumnBlock {
            names: names.clone(),
            tys: tys.clone(),
            n: self.len,
            home: self.home,
            columns: self.block.clone(),
            postings: Default::default(),
        })
    }
}

#[derive(Default)]
pub struct LocalSchemas(FxHashMap<(usize, usize), (Names, Tys)>);

pub enum FixedColumn {
    Constant(u64),
    Sequence(u64),
    Values { base: u64, width: u8 },
}

impl Block {
    pub fn constant(&self, column: usize) -> Option<u64> {
        match self.columns[column] {
            Words::Constant(value) => Some(value),
            _ => None,
        }
    }
    pub fn share(&self) -> SharedBlock<'_> {
        SharedBlock {
            block: Arc::from(self.columns.as_slice()),
            schema: (Rc::as_ptr(&self.names) as usize, Rc::as_ptr(&self.tys) as usize),
            names: &self.names,
            types: &self.tys,
            len: self.n,
            home: self.home,
        }
    }

    pub fn words(&self,col:usize)->Vec<u64>{
        let mut out=Vec::with_capacity(self.n);
        self.columns[col].append(self.n,&mut out);
        out
    }
    pub fn new(home: u32, names: Names, tys: Tys, n: usize, columns: Vec<Words>) -> Self {
        Self {
            home,
            names,
            tys,
            n,
            columns,
            postings: Default::default(),
        }
    }

    // Untranslated numeric columns already have the stable fixed-width form.
    pub fn append_fixed(&self, column: usize, out: &mut Vec<u8>) -> Option<FixedColumn> {
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

    pub fn stored(
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

}

impl<C: AsRef<[Words]>> ColumnBlock<C> {
    pub fn find(&self, col: usize, key: u64, mut hit: impl FnMut(usize) -> bool) -> bool {
        let words = &self.columns.as_ref()[col];
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

pub enum Piece {
    File(Rc<Block>),
    Worker(Rc<WorkerBlock>),
}

impl Piece {
    pub fn columns(&self) -> &[Words] {
        match self {
            Self::File(block) => &block.columns,
            Self::Worker(block) => &block.columns,
        }
    }
    pub fn file(&self) -> u32 {
        match self {
            Self::File(block) => block.home,
            Self::Worker(block) => block.home,
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::File(block) => block.n,
            Self::Worker(block) => block.n,
        }
    }
    pub fn find(&self, col: usize, key: u64, hit: impl FnMut(usize) -> bool) -> bool {
        match self {
            Self::File(block) => block.find(col, key, hit),
            Self::Worker(block) => block.find(col, key, hit),
        }
    }
}

#[derive(Default)]
pub struct Population {
    pub schema: Option<(Names, Tys)>,
    pub pieces: Vec<Piece>,
    pub by_file: OnceCell<FxHashMap<u32, Vec<usize>>>,
    pub offsets: OnceCell<Vec<u32>>,
    pub n: usize,
    pub local_files: OnceCell<Vec<bool>>,
    pub materialized: OnceCell<Rc<Table>>,
}

pub type Inputs = std::collections::BTreeMap<usize, Rc<Population>>;

impl Population {
    pub fn add(&mut self, block: &Rc<Block>) -> R<()> {
        self.add_piece(block, || Piece::File(block.clone()))
    }

    pub fn add_worker(&mut self, block: &Rc<WorkerBlock>) -> R<()> {
        self.add_piece(block, || Piece::Worker(block.clone()))
    }

    pub fn add_piece<C>(
        &mut self, table: &ColumnBlock<C>, piece: impl FnOnce() -> Piece,
    ) -> R<()> {
        let (names, tys) = self.schema.get_or_insert_with(|| (table.names.clone(), table.tys.clone()));
        if *names != table.names || *tys != table.tys {
            return Err(format!("file frontier schema differs between providers: existing {names:?} {tys:?}, incoming {:?} {:?}", table.names, table.tys).into());
        }
        if table.n != 0 {
            self.n = self.n.checked_add(table.n).ok_or("closed population exceeds address space")?;
            self.pieces.push(piece());
        }
        Ok(())
    }

    pub fn agrees(&self, names: &Names, tys: &Tys) -> bool {
        self.schema
            .as_ref()
            .is_none_or(|(n, t)| n == names && t == tys)
    }

    pub fn full(&self, names: &Names, tys: &Tys) -> Rc<Table> {
        self.materialized
            .get_or_init(|| {
                let mut out = Table::retained(names.clone(), tys.clone(), self.n);
                for block in &self.pieces {
                    for (from, to) in block.columns().iter().zip(&mut out.cols) {
                        from.append(block.len(), to);
                    }
                }
                out.n = self.n;
                Rc::new(out)
            })
            .clone()
    }

    pub fn same_rows(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || (
            self.n == other.n && self.schema == other.schema
                && self.pieces.len() == other.pieces.len()
                && self.pieces.iter().zip(&other.pieces).all(|(a,b)| {
                    a.len() == b.len() && a.file() == b.file() && (
                        std::ptr::eq(a.columns(), b.columns()) || a.columns().iter().zip(b.columns()).all(|(x,y)| {
                                (0..a.len()).all(|i| x.at(i) == y.at(i))
                            })
                    )
                })
        )
    }

    pub fn cells(&self) -> usize {
        self.n.saturating_mul(self.schema.as_ref().map_or(1, |(names,_)| names.len().max(1)))
    }

    /// Use the carried posting while preserving the whole table's row order.
    pub fn matching_rows(&self, col: usize, key: u64, hits: &mut Vec<u32>) {
        let offsets = self.offsets.get_or_init(|| {
            let mut start = 0;
            self.pieces.iter().map(|piece| {
                let offset = start;
                start += piece.len() as u32;
                offset
            }).collect()
        });
        hits.clear();
        self.each_match(col, key, |piece, _, row| hits.push(offsets[piece] + row as u32));
    }

    pub fn is_materialized(&self, t: &Rc<Table>) -> bool {
        self.materialized
            .get()
            .is_some_and(|owned| Rc::ptr_eq(owned, t))
    }

    pub fn can_probe(&self, col: usize) -> bool {
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
                matches!(t.columns()[c],Words::Constant(x) if x==u64::from(t.file()) || x==super::NULL)
            })
                })
                .collect()
        })[col]
    }

    pub fn find(
        &self,
        col: usize,
        key: u64,
        mut visit: impl FnMut(usize, &Piece, usize) -> bool,
    ) -> bool {
        let Some((_, tys)) = &self.schema else {
            return false;
        };
        if key != super::NULL && self.can_probe(col) {
            let homes = self.by_file.get_or_init(|| {
                let mut homes = FxHashMap::<u32, Vec<usize>>::default();
                for (i, t) in self.pieces.iter().enumerate() {
                    homes.entry(t.file()).or_default().push(i);
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

    pub fn each_match(&self, col: usize, key: u64, mut each: impl FnMut(usize, &Piece, usize)) {
        self.find(col, key, |i, t, r| {
            each(i, t, r);
            false
        });
    }

    pub fn contains_keys(
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
                .all(|(&lc, &rc)| t.columns()[rc].at(r) == left.cols[lc][row])
        })
    }

    pub fn neighbors(&self, by: usize, to: usize, key: u64, out: &mut Vec<u64>) {
        self.each_match(by, key, |_, block, row| out.push(block.columns()[to].at(row)));
    }

    /// Match the table index: probe keys in caller order, then source order within each key.
    pub fn probe(&self, names: &Names, tys: &Tys, col: usize, keys: &[u64]) -> Rc<Table> {
        let mut rows = Vec::new();
        for &key in keys {
            self.each_match(col, key, |i, _, r| rows.push((i, r)));
        }
        let mut out = Table::retained(names.clone(), tys.clone(), rows.len());
        for (c, to) in out.cols.iter_mut().enumerate() {
            to.extend(rows.iter().map(|&(i, r)| self.pieces[i].columns()[c].at(r)));
        }
        out.n = rows.len();
        Rc::new(out)
    }
}

/// A completed file block can leave its worker after all local references are released.
/// Lookup postings are built only by the receiving project executor.
pub struct CarriedBlock {
    pub schema: usize,
    pub rows: usize,
    pub home: u32,
    pub columns: Vec<Words>,
}

/// File tables in one worker share column declarations. Retain one owned copy
/// for the join even when many frontier blocks use the same declaration.
#[derive(Default)]
pub struct CarriedSchemas {
    /// The schemas made so far that have each content hash: a declaration is told by what it declares, never by where the
    /// block that held it kept it (the room of a declaration given up is taken by the next one).
    pub ids: FxHashMap<u64, Vec<usize>>,
    pub schemas: Vec<(Vec<String>, Vec<Ty>)>,
}
impl CarriedSchemas {
    pub fn content_hash(names: &[String], types: &[Ty]) -> u64 {
        use std::hash::{Hash as _, Hasher as _};
        let mut hasher = rustc_hash::FxHasher::default();
        names.hash(&mut hasher);
        types.hash(&mut hasher);
        hasher.finish()
    }
    pub fn intern(&mut self, names: Names, types: Tys) -> usize {
        let candidates = self.ids.entry(Self::content_hash(&names, &types)).or_default();
        if let Some(&id) = candidates
            .iter()
            .find(|&&id| self.schemas[id].0 == *names && self.schemas[id].1 == *types)
        {
            return id;
        }
        let id = self.schemas.len();
        self.schemas
            .push((Rc::unwrap_or_clone(names), Rc::unwrap_or_clone(types)));
        candidates.push(id);
        id
    }
    pub fn receive(self) -> Vec<(Names, Tys)> {
        self.schemas
            .into_iter()
            .map(|(n, t)| (Rc::new(n), Rc::new(t)))
            .collect()
    }
}
impl Block {
    pub fn carry(self, shapes: &mut CarriedSchemas) -> CarriedBlock {
        CarriedBlock {
            schema: shapes.intern(self.names, self.tys),
            rows: self.n,
            home: self.home,
            columns: self.columns,
        }
    }
}

impl CarriedBlock {
    pub fn receive(
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
                *words = Words::pack(std::borrow::Cow::Owned((0..self.rows).map(|i| remap(words.at(i))).collect()));
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

#[cfg(test)]
mod carried_schemas_tests {
    use super::*;

    /// A block's declaration is given up when its schema is carried, and the room it held goes to the next declaration of the
    /// same size: two declarations that differ must stay two, whatever room they were in.
    #[test]
    fn a_declaration_is_told_by_its_content_not_by_the_room_it_was_in() {
        let mut shapes = CarriedSchemas::default();
        // Blocks of one worker share the declaration of their types; the names are the declaration given up here.
        let types = Rc::new(vec![Ty::Sym]);
        let names = Rc::new(vec!["a".to_owned()]);
        let names_at = Rc::as_ptr(&names);
        // The room the schema's own bookkeeping takes is taken beforehand, so the room of the names is left to the loop below.
        shapes.ids.entry(CarriedSchemas::content_hash(&names, &types)).or_default().reserve(8);
        let first = shapes.intern(names, types.clone());
        // The room of the names given up goes to a later declaration of its size: allocate until one takes it.
        let mut rooms = Vec::new();
        let mut taken = None;
        for _ in 0..65536 {
            let other = Rc::new(vec!["b".to_owned()]);
            if Rc::as_ptr(&other) == names_at {
                taken = Some(other);
                break;
            }
            rooms.push(other);
        }
        let names = taken.expect("the allocator hands the room given up to a later declaration of its size: the case this test is about");
        let second = shapes.intern(names, types.clone());
        assert_ne!(first, second, "another declaration is another schema");
        let again = shapes.intern(Rc::new(vec!["a".to_owned()]), types);
        assert_eq!(again, first, "the same declaration is the same schema");
        let received = shapes.receive();
        assert_eq!(received.len(), 2);
        assert_eq!(*received[second].0, vec!["b".to_owned()]);
    }
}
