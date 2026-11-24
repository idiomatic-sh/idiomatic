//! Share ordered path lists independently of their source authorities and versions.
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    ops::Index,
    rc::Rc,
};

#[derive(Default)]
pub(crate) struct InputLists {
    lists: Vec<Rc<Vec<String>>>,
    by_hash: HashMap<u64, Vec<usize>>,
}

impl InputLists {
    pub(crate) fn intern(&mut self, paths: &[&str]) -> usize {
        let (hash, prior) = self.lookup(paths);
        prior.unwrap_or_else(|| {
            self.insert(
                hash,
                Rc::new(paths.iter().map(|path| (*path).to_owned()).collect()),
            )
        })
    }

    pub(crate) fn intern_owned(&mut self, paths: Rc<Vec<String>>) -> Rc<Vec<String>> {
        let (hash, prior) = self.lookup(paths.as_slice());
        let id = prior.unwrap_or_else(|| self.insert(hash, paths));
        self[id].clone()
    }

    fn lookup(&self, paths: &[impl AsRef<str>]) -> (u64, Option<usize>) {
        let mut hasher = DefaultHasher::new();
        paths.len().hash(&mut hasher);
        for path in paths {
            path.as_ref().hash(&mut hasher);
        }
        let hash = hasher.finish();
        let found = self.by_hash.get(&hash).and_then(|entries| {
            entries.iter().copied().find(|&id| {
                self[id]
                    .iter()
                    .map(String::as_str)
                    .eq(paths.iter().map(AsRef::as_ref))
            })
        });
        (hash, found)
    }

    fn insert(&mut self, hash: u64, paths: Rc<Vec<String>>) -> usize {
        let id = self.lists.len();
        self.lists.push(paths);
        self.by_hash.entry(hash).or_default().push(id);
        id
    }
}

impl Index<usize> for InputLists {
    type Output = Rc<Vec<String>>;

    fn index(&self, id: usize) -> &Self::Output {
        &self.lists[id]
    }
}
