//! Stable names and typed columns for the portion of one source consumed by a closed query.
use super::{World, finish};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Serialize, Deserialize)]
enum Words {
    Constant(u64),
    Sequence(u64),
    Values(Vec<u64>),
    Null,
}
impl Words {
    fn pack(v: Vec<u64>) -> Self {
        let first = v.first().copied().unwrap_or(0);
        if v.iter().all(|x| *x == first) {
            if first == u64::MAX {
                Self::Null
            } else {
                Self::Constant(first)
            }
        } else if v
            .iter()
            .enumerate()
            .all(|(i, x)| first.checked_add(i as u64) == Some(*x))
        {
            Self::Sequence(first)
        } else {
            Self::Values(v)
        }
    }
    fn unpack(self, n: usize) -> Vec<u64> {
        match self {
            // Block::new packs a singleton into a constant with the block's full height.
            Self::Constant(v) => vec![v],
            Self::Null => Vec::new(),
            Self::Sequence(v) => (0..n).map(|i| v + i as u64).collect(),
            Self::Values(v) => v,
        }
    }
}
#[derive(Serialize, Deserialize)]
pub(crate) struct StoredBlock {
    home: String,
    names: Vec<String>,
    types: Vec<finish::Ty>,
    count: usize,
    words: Vec<Vec<u8>>,
    columns: Vec<Words>,
}
impl StoredBlock {
    pub(crate) fn save(block: &finish::population::Block, world: &World) -> Self {
        use finish::Ty;
        let mut words = Vec::new();
        #[derive(Clone, Copy, Hash, Eq, PartialEq)]
        enum Word {
            Path(u32),
            Symbol(u32),
        }
        let mut lookup = FxHashMap::default();
        let mut intern = |id: Word| {
            *lookup.entry(id).or_insert_with(|| {
                let bytes = match id {
                    Word::Path(i) => world.run.paths[i as usize].as_bytes(),
                    Word::Symbol(i) => world.run.text(i),
                };
                let index = words.len() as u64;
                words.push(bytes.to_vec());
                index
            })
        };
        let columns = block
            .tys
            .iter()
            .enumerate()
            .filter(|_| block.n != 0)
            .map(|(i, ty)| {
                let mut column = block
                    .constant(i)
                    .map_or_else(|| block.words(i), |value| vec![value]);
                for v in &mut column {
                    if *v == u64::MAX {
                        continue;
                    }
                    *v = match ty {
                        Ty::File => intern(Word::Path(*v as u32)),
                        Ty::Node => {
                            (intern(Word::Path((*v >> 32) as u32)) << 32) | (*v & 0xffff_ffff)
                        }
                        Ty::Sym if world.run.absent_word == Some(*v as u32) => u64::MAX - 1,
                        Ty::Sym => intern(Word::Symbol(*v as u32)),
                        _ => *v,
                    };
                }
                Words::pack(column)
            })
            .collect();
        Self {
            home: if block.n == 0 {
                String::new()
            } else {
                world.run.paths[block.home as usize].clone()
            },
            names: (*block.names).clone(),
            types: (*block.tys).clone(),
            count: block.n,
            words,
            columns,
        }
    }
    pub(crate) fn restore(self, world: &mut World) -> Rc<finish::population::Block> {
        use finish::Ty;
        // The writer interns paths and symbols under distinct dictionary entries.
        // Rebind each entry on first use, preserving insertion order in the run.
        let mut translated = vec![None; self.words.len()];
        let mut columns: Vec<Vec<u64>> = self
            .columns
            .into_iter()
            .zip(&self.types)
            .map(|(packed, ty)| {
                let mut column = packed.unpack(self.count);
                if matches!(ty, Ty::Node | Ty::File | Ty::Sym) {
                    for v in &mut column {
                        if *v == u64::MAX {
                            continue;
                        }
                        if *ty == Ty::Sym && *v == u64::MAX - 1 {
                            *v = u64::from(world.run.absent_word.expect("declared absent word"));
                            continue;
                        }
                        let index = if *ty == Ty::Node { *v >> 32 } else { *v } as usize;
                        let id = *translated[index].get_or_insert_with(|| {
                            if *ty == Ty::Sym {
                                world.run.sym(&self.words[index])
                            } else {
                                world.file(
                                    std::str::from_utf8(&self.words[index]).expect("stored origin"),
                                )
                            }
                        });
                        *v = if *ty == Ty::Node {
                            (u64::from(id) << 32) | (*v & 0xffff_ffff)
                        } else {
                            u64::from(id)
                        };
                    }
                }
                column
            })
            .collect();
        if self.count == 0 {
            columns.resize_with(self.types.len(), Vec::new);
        }
        let home = if self.count == 0 {
            0
        } else {
            world.file(&self.home)
        };
        Rc::new(finish::population::Block::new(
            home,
            Rc::new(self.names),
            Rc::new(self.types),
            self.count,
            columns,
        ))
    }
}
