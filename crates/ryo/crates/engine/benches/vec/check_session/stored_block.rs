//! A postcard block header followed by independently addressable little-endian columns.
use super::{World, finish};
use finish::population::{Block, BlockBytes, FixedColumn, StoredValues, StoredWords};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::{
    hash::{Hash, Hasher},
    rc::Rc,
    sync::Arc,
};

#[derive(Serialize, Deserialize)]
enum Column {
    Constant(u64),
    Sequence(u64),
    Values { base: u64, width: u8 },
}

#[derive(Serialize, Deserialize)]
enum Word<'a> {
    #[serde(borrow)]
    Path(&'a str),
    #[serde(borrow)]
    Symbol(&'a [u8]),
}

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
enum WordId {
    Path(u32),
    Symbol(u32),
}

#[derive(Serialize, Deserialize)]
struct Header<'a, Names, Types> {
    #[serde(borrow)]
    home: &'a str,
    names: Names,
    types: Types,
    count: usize,
    #[serde(borrow)]
    words: Vec<Word<'a>>,
    columns: Vec<Column>,
}

pub(crate) fn encode(block: &Block, world: &World) -> Vec<u8> {
    use finish::Ty;
    let mut words = Vec::new();
    let mut lookup = FxHashMap::default();
    let mut intern = |id: WordId| {
        *lookup.entry(id).or_insert_with(|| {
            let word = match id {
                WordId::Path(i) => Word::Path(&world.run.paths[i as usize]),
                WordId::Symbol(i) => Word::Symbol(world.run.text(i)),
            };
            let index = words.len() as u64;
            words.push(word);
            index
        })
    };
    let mut values = Vec::new();
    let columns = block
        .tys
        .iter()
        .enumerate()
        .filter(|_| block.n != 0)
        .map(|(i, ty)| {
            if !matches!(ty, Ty::Node | Ty::File | Ty::Sym)
                && let Some(column) = block.append_fixed(i, &mut values)
            {
                return match column {
                    FixedColumn::Constant(v) => Column::Constant(v),
                    FixedColumn::Sequence(v) => Column::Sequence(v),
                    FixedColumn::Values { base, width } => Column::Values { base, width },
                };
            }
            let mut column = block
                .constant(i)
                .map_or_else(|| block.words(i), |v| vec![v]);
            for v in &mut column {
                if *v != u64::MAX {
                    *v = match ty {
                        Ty::File => intern(WordId::Path(*v as u32)),
                        Ty::Node => {
                            (intern(WordId::Path((*v >> 32) as u32)) << 32) | (*v & 0xffff_ffff)
                        }
                        Ty::Sym if world.run.absent_word == Some(*v as u32) => u64::MAX - 1,
                        Ty::Sym => intern(WordId::Symbol(*v as u32)),
                        _ => *v,
                    };
                }
            }
            pack_column(&column, &mut values)
        })
        .collect();
    let header = Header {
        home: if block.n == 0 {
            ""
        } else {
            &world.run.paths[block.home as usize]
        },
        names: block.names.as_slice(),
        types: block.tys.as_slice(),
        count: block.n,
        words,
        columns,
    };
    let mut data = postcard::to_stdvec(&header).expect("typed block header");
    data.extend(values);
    data
}

fn pack_column(column: &[u64], bytes: &mut Vec<u8>) -> Column {
    let first = column[0];
    if column.iter().all(|&v| v == first) {
        return Column::Constant(first);
    }
    if column
        .iter()
        .enumerate()
        .all(|(i, &v)| first.checked_add(i as u64) == Some(v))
    {
        return Column::Sequence(first);
    }
    let (base, high) = column
        .iter()
        .fold((first, first), |(low, high), &v| (low.min(v), high.max(v)));
    let width = match high - base {
        0..=255 => 1,
        256..=65535 => 2,
        65536..=4294967295 => 4,
        _ => 8,
    };
    bytes.reserve(column.len() * width);
    for &word in column {
        bytes.extend_from_slice(&(word - base).to_le_bytes()[..width]);
    }
    Column::Values {
        base,
        width: width as u8,
    }
}

#[derive(Default)]
pub(crate) struct Schemas(FxHashMap<u64, Vec<(finish::Names, finish::Tys)>>);

impl Schemas {
    fn intern(&mut self, names: &[&str], types: Vec<finish::Ty>) -> (finish::Names, finish::Tys) {
        let mut hasher = rustc_hash::FxHasher::default();
        (names, &types).hash(&mut hasher);
        let entries = self.0.entry(hasher.finish()).or_default();
        if let Some(found) = entries
            .iter()
            .find(|(n, t)| n.iter().map(String::as_str).eq(names.iter().copied()) && **t == types)
        {
            return found.clone();
        }
        let entry = (
            Rc::new(names.iter().map(|&n| n.to_owned()).collect()),
            Rc::new(types),
        );
        entries.push(entry.clone());
        entry
    }
}

// The borrowed names are used to look up a shared schema. The block retains
// owned names only on the first occurrence of a given shape.
#[derive(Deserialize)]
struct MappedHeader<'a> {
    #[serde(borrow)]
    home: &'a str,
    #[serde(borrow)]
    names: Vec<&'a str>,
    types: Vec<finish::Ty>,
    count: usize,
    #[serde(borrow)]
    words: Vec<Word<'a>>,
    columns: Vec<Column>,
}

pub(crate) fn restore(
    bytes: BlockBytes,
    world: &mut World,
    schemas: &mut Schemas,
) -> Option<Rc<Block>> {
    let data = bytes.as_ref().as_ref();
    let (header, tail) = postcard::take_from_bytes::<MappedHeader<'_>>(data).ok()?;
    let mut start = data.len() - tail.len();
    if header.names.len() != header.types.len()
        || (header.count != 0 && header.columns.len() != header.types.len())
        || (header.count == 0 && !header.columns.is_empty())
    {
        return None;
    }
    let dictionary = header
        .words
        .into_iter()
        .map(|word| match word {
            Word::Path(path) => world.file(path),
            Word::Symbol(text) => world.run.sym(text),
        })
        .collect();
    let home = if header.count == 0 {
        0
    } else {
        world.file(header.home)
    };
    let values = Arc::new(StoredValues {
        bytes: bytes.clone(),
        dictionary,
        absent: world.run.absent_word,
    });
    let mut columns = Vec::with_capacity(header.types.len());
    for (column, &ty) in header.columns.into_iter().zip(&header.types) {
        let offset = u32::try_from(start).ok()?;
        let mapped = |base, width| StoredWords {
            values: values.clone(),
            start: offset,
            ty,
            width,
            base,
        };
        columns.push(match column {
            Column::Constant(word) => Err(values.translate(ty, word)),
            Column::Sequence(base) => Ok(mapped(base, 0)),
            Column::Values { base, width } => {
                if !matches!(width, 1 | 2 | 4 | 8) {
                    return None;
                }
                let words = mapped(base, width);
                start = start.checked_add(header.count.checked_mul(usize::from(width))?)?;
                if start > data.len() {
                    return None;
                }
                Ok(words)
            }
        });
    }
    if start != data.len() {
        return None;
    }
    if header.count == 0 {
        columns.resize_with(header.types.len(), || Err(u64::MAX));
    }
    let (names, types) = schemas.intern(&header.names, header.types);
    Some(Rc::new(Block::stored(
        home,
        names,
        types,
        header.count,
        columns,
    )))
}
