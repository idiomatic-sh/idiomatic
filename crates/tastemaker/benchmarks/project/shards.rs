//! Content-addressed postcard shards. The manifest selects a complete generation.
use super::{Key, Res, atomic, load, table};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::Path,
};

pub(super) type Index = BTreeMap<u8, Key>;

pub(super) struct Shards<T> {
    name: &'static str,
    pub read_bytes: usize,
    pub write_bytes: usize,
    pub index: Index,
    opened: BTreeMap<u8, BTreeMap<Key, T>>,
    changed: BTreeSet<u8>,
}
impl<T: Serialize + DeserializeOwned> Shards<T> {
    pub fn new(name: &'static str, index: Index) -> Self {
        Self {
            name,
            read_bytes: 0,
            write_bytes: 0,
            index,
            opened: BTreeMap::new(),
            changed: BTreeSet::new(),
        }
    }
    pub fn open(&mut self, dir: &Path, key: &Key) {
        self.opened.entry(key[0]).or_insert_with(|| {
            if let Some(d) = self.index.get(&key[0]).copied() {
                let path = table(dir, self.name, d);
                if let Some(rows) = load(&path, d) {
                    self.read_bytes += fs::metadata(path).map_or(0, |m| m.len() as usize);
                    return rows;
                }
                self.index.remove(&key[0]);
            }
            BTreeMap::new()
        });
    }
    pub fn get(&self, key: &Key) -> Option<&T> {
        self.opened.get(&key[0])?.get(key)
    }
    pub fn insert(&mut self, key: Key, value: T) {
        self.changed.insert(key[0]);
        self.opened
            .get_mut(&key[0])
            .expect("open before updating")
            .insert(key, value);
    }
    pub fn change(&mut self, key: &Key) -> &mut T
    where
        T: Default,
    {
        self.changed.insert(key[0]);
        self.opened
            .get_mut(&key[0])
            .expect("open before updating")
            .entry(*key)
            .or_default()
    }
    pub fn dirty(&self) -> bool {
        !self.changed.is_empty()
    }
    pub fn flush(&mut self, dir: &Path, obsolete: &mut Vec<std::path::PathBuf>) -> Res<usize> {
        let mut written = 0;
        for &prefix in &self.changed {
            let bytes = postcard::to_stdvec(&self.opened[&prefix]).map_err(io::Error::other)?;
            let digest = *blake3::hash(&bytes).as_bytes();
            let dst = table(dir, self.name, digest);
            if self.index.get(&prefix) != Some(&digest)
                || !fs::metadata(&dst).is_ok_and(|m| m.len() == bytes.len() as u64)
            {
                atomic(dir, &dst, &bytes)?;
                written += bytes.len();
            }
            if let Some(old) = self.index.insert(prefix, digest)
                && old != digest
            {
                obsolete.push(table(dir, self.name, old));
            }
        }
        self.changed.clear();
        self.write_bytes += written;
        Ok(written)
    }
}

/// File contributions are much larger than request indexes. A shard has a sorted
/// postcard directory followed by independent postcard records. A closure can
/// read one record without deserializing any neighboring file's columns.
type Location = (u64, usize, Key);
type Directory = BTreeMap<Key, Location>;
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(super) struct Segment {
    name: Key,
    directory: Key,
    bytes: usize,
}
pub(super) type Segments = BTreeMap<u8, Segment>;
pub(super) struct Records<T> {
    pub index: Segments,
    directories: BTreeMap<u8, (fs::File, Directory, u64)>,
    rows: BTreeMap<Key, T>,
    changed: BTreeSet<Key>,
    pub read_bytes: usize,
    pub write_bytes: usize,
}
impl<T: Serialize + DeserializeOwned + Default> Records<T> {
    pub fn new(index: Segments) -> Self {
        Self {
            index,
            directories: BTreeMap::new(),
            rows: BTreeMap::new(),
            changed: BTreeSet::new(),
            read_bytes: 0,
            write_bytes: 0,
        }
    }
    fn part(&mut self, dir: &Path, prefix: u8) -> Option<&(fs::File, Directory, u64)> {
        use std::os::unix::fs::FileExt;
        if !self.directories.contains_key(&prefix) {
            let s = self.index.get(&prefix)?;
            let file = fs::File::open(table(dir, "pairs", s.name)).ok()?;
            if s.bytes as u64 > file.metadata().ok()?.len() {
                return None;
            }
            let mut raw = vec![0; s.bytes];
            file.read_exact_at(&mut raw, 0).ok()?;
            if blake3::hash(&raw).as_bytes() != &s.directory {
                return None;
            }
            let entries = postcard::from_bytes(&raw).ok()?;
            self.read_bytes += raw.len();
            self.directories
                .insert(prefix, (file, entries, s.bytes as u64));
        }
        self.directories.get(&prefix)
    }
    pub fn open(&mut self, dir: &Path, k: &Key) {
        use std::os::unix::fs::FileExt;
        if self.rows.contains_key(k) {
            return;
        }
        if let Some((f, entries, base)) = self.part(dir, k[0])
            && let Some((off, len, hash)) = entries.get(k)
        {
            let mut bytes = vec![0; *len];
            if f.read_exact_at(&mut bytes, base + off).is_ok()
                && blake3::hash(&bytes).as_bytes() == hash
                && let Ok(row) = postcard::from_bytes(&bytes)
            {
                self.read_bytes += bytes.len();
                self.rows.insert(*k, row);
            }
        }
    }
    pub fn get(&self, k: &Key) -> Option<&T> {
        self.rows.get(k)
    }
    pub fn change(&mut self, k: &Key) -> &mut T {
        self.changed.insert(*k);
        self.rows.entry(*k).or_default()
    }
    pub fn dirty(&self) -> bool {
        !self.changed.is_empty()
    }
    pub fn flush(&mut self, dir: &Path, obsolete: &mut Vec<std::path::PathBuf>) -> Res<usize> {
        use std::os::unix::fs::FileExt;
        let mut written = 0;
        let mut parts = BTreeMap::<u8, Vec<Key>>::new();
        for &k in &self.changed {
            parts.entry(k[0]).or_default().push(k);
        }
        for (prefix, keys) in parts {
            let mut entries = Directory::new();
            let mut body = Vec::new();
            if let Some((f, prior, base)) = self.part(dir, prefix) {
                for (k, (off, len, hash)) in prior {
                    if keys.binary_search(k).is_ok() {
                        continue;
                    }
                    let pos = body.len();
                    body.resize(pos + len, 0);
                    if f.read_exact_at(&mut body[pos..], base + off).is_ok()
                        && blake3::hash(&body[pos..]).as_bytes() == hash
                    {
                        entries.insert(*k, (pos as u64, *len, *hash));
                    } else {
                        body.truncate(pos);
                    }
                }
            }
            for k in keys {
                let bytes = postcard::to_stdvec(&self.rows[&k]).map_err(io::Error::other)?;
                entries.insert(
                    k,
                    (
                        body.len() as u64,
                        bytes.len(),
                        *blake3::hash(&bytes).as_bytes(),
                    ),
                );
                body.extend(bytes);
            }
            let mut bytes = postcard::to_stdvec(&entries).map_err(io::Error::other)?;
            let directory = *blake3::hash(&bytes).as_bytes();
            let len = bytes.len();
            bytes.extend(body);
            let name = *blake3::hash(&bytes).as_bytes();
            atomic(dir, &table(dir, "pairs", name), &bytes)?;
            written += bytes.len();
            if let Some(old) = self.index.insert(
                prefix,
                Segment {
                    name,
                    directory,
                    bytes: len,
                },
            ) && old.name != name
            {
                obsolete.push(table(dir, "pairs", old.name));
            }
            self.directories.remove(&prefix);
        }
        self.changed.clear();
        self.write_bytes += written;
        Ok(written)
    }
}
