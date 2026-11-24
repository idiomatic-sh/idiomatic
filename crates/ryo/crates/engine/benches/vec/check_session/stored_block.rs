//! Stable names and typed columns for the portion of one source consumed by a closed query.
use super::{World, finish};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, rc::Rc};

#[derive(Serialize, Deserialize)]
enum Words {
    Constant(u64),
    Sequence(u64),
    Values(Vec<u64>),
}
impl Words {
    fn pack(v: Vec<u64>) -> Self {
        let first = v.first().copied().unwrap_or(0);
        if v.iter().all(|x| *x == first) {
            Self::Constant(first)
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
    fn unpack(&self, n: usize) -> Vec<u64> {
        match self {
            Self::Constant(v) => vec![*v; n],
            Self::Sequence(v) => (0..n).map(|i| v + i as u64).collect(),
            Self::Values(v) => v.clone(),
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
        let mut lookup = HashMap::new();
        let mut intern = |word: &[u8]| {
            if let Some(i) = lookup.get(word) {
                return *i;
            }
            let i = words.len() as u64;
            lookup.insert(word.to_vec(), i);
            words.push(word.to_vec());
            i
        };
        let columns = block
            .tys
            .iter()
            .enumerate()
            .filter(|_| block.n != 0)
            .map(|(i, ty)| {
                let mut column = block.words(i);
                for v in &mut column {
                    if *v == u64::MAX {
                        continue;
                    }
                    *v = match ty {
                        Ty::File => intern(world.run.paths[*v as usize].as_bytes()),
                        Ty::Node => {
                            (intern(world.run.paths[(*v >> 32) as usize].as_bytes()) << 32)
                                | (*v & 0xffff_ffff)
                        }
                        Ty::Sym if world.run.absent_word == Some(*v as u32) => u64::MAX - 1,
                        Ty::Sym => intern(world.run.text(*v as u32)),
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
    pub(crate) fn restore(&self, world: &mut World) -> Rc<finish::population::Block> {
        use finish::Ty;
        let columns = self
            .columns
            .iter()
            .zip(&self.types)
            .map(|(packed, ty)| {
                let mut column = packed.unpack(self.count);
                for v in &mut column {
                    if *v == u64::MAX {
                        continue;
                    }
                    *v = match ty {
                        Ty::File => u64::from(
                            world.file(
                                std::str::from_utf8(&self.words[*v as usize])
                                    .expect("stored file name"),
                            ),
                        ),
                        Ty::Node => {
                            (u64::from(
                                world.file(
                                    std::str::from_utf8(&self.words[(*v >> 32) as usize])
                                        .expect("stored origin"),
                                ),
                            ) << 32)
                                | (*v & 0xffff_ffff)
                        }
                        Ty::Sym if *v == u64::MAX - 1 => {
                            u64::from(world.run.absent_word.expect("declared absent word"))
                        }
                        Ty::Sym => u64::from(world.run.sym(&self.words[*v as usize])),
                        _ => *v,
                    }
                }
                column
            })
            .collect();
        let home = if self.count == 0 {
            0
        } else {
            world.file(&self.home)
        };
        Rc::new(finish::population::Block::new(
            home,
            Rc::new(self.names.clone()),
            Rc::new(self.types.clone()),
            self.count,
            columns,
        ))
    }
}
