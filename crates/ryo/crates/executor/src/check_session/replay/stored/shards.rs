//! Content-addressed postcard shards. The manifest selects a complete generation.
use super::{Key, Res, atomic, load, table};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    ops::Range,
    path::Path,
    sync::Arc,
};

pub type Index = BTreeMap<u8, Key>;

pub struct Shards<T> {
    pub name: &'static str,
    pub read_bytes: usize,
    pub read_cpu: f64,
    pub write_bytes: usize,
    pub index: Index,
    pub opened: BTreeMap<u8, BTreeMap<Key, T>>,
    pub changed: BTreeSet<u8>,
    // Key bits dropped when choosing a shard: a table every run reads whole keeps few files.
    pub shift: u8,
}
impl<T: Serialize + DeserializeOwned> Shards<T> {
    pub fn new(name: &'static str, index: Index) -> Self {
        Self {
            name,
            read_bytes: 0,
            read_cpu: 0.0,
            write_bytes: 0,
            index,
            opened: BTreeMap::new(),
            changed: BTreeSet::new(),
            shift: 0,
        }
    }
    /// Keep `256 >> shift` shards. Small rows that an unchanged run reads together share their files.
    pub fn coarse(mut self, shift: u8) -> Self {
        self.shift = shift;
        self
    }
    pub fn shard(&self, key: &Key) -> u8 {
        key[0] >> self.shift
    }
    pub fn open(&mut self, dir: &Path, key: &Key) {
        let shard = self.shard(key);
        self.opened.entry(shard).or_insert_with(|| {
            if let Some(d) = self.index.get(&shard).copied() {
                let path = table(dir, self.name, d);
                let start = crate::thread_cpu();
                let rows = load(&path, d);
                self.read_cpu += crate::thread_cpu() - start;
                if let Some(rows) = rows {
                    self.read_bytes += fs::metadata(path).map_or(0, |m| m.len() as usize);
                    return rows;
                }
                self.index.remove(&shard);
            }
            BTreeMap::new()
        });
    }
    pub fn get(&self, key: &Key) -> Option<&T> {
        self.opened.get(&self.shard(key))?.get(key)
    }
    pub fn insert(&mut self, key: Key, value: T) {
        let shard = self.shard(&key);
        self.changed.insert(shard);
        self.opened
            .get_mut(&shard)
            .expect("open before updating")
            .insert(key, value);
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

/// Answer packets have immutable keys. Append new packets as a postcard table
/// and fold the append tables into their affected shards after a bounded number.
pub struct Packets<T> {
    pub name: &'static str,
    pub base: Shards<T>,
    pub layers: Vec<Key>,
    pub pending: BTreeMap<Key, T>,
    pub seen: BTreeMap<Key, T>,
    pub opened: bool,
    pub read_bytes: usize,
    pub read_cpu: f64,
    pub write_bytes: usize,
}
impl<T: Serialize + DeserializeOwned> Packets<T> {
    pub fn new(name: &'static str, index: Index, layers: Vec<Key>) -> Self {
        Self {
            name,
            base: Shards::new(name, index),
            layers,
            pending: BTreeMap::new(),
            seen: BTreeMap::new(),
            opened: false,
            read_bytes: 0,
            read_cpu: 0.0,
            write_bytes: 0,
        }
    }

    pub fn coarse(mut self, shift: u8) -> Self {
        self.base = self.base.coarse(shift);
        self
    }

    pub fn open(&mut self, dir: &Path, key: &Key) {
        if !self.opened {
            let start = crate::thread_cpu();
            self.layers.retain(|&digest| {
                let path = table(dir, self.name, digest);
                let Some(mut values): Option<BTreeMap<Key, T>> = load(&path, digest) else {
                    return false;
                };
                self.read_bytes += fs::metadata(path).map_or(0, |m| m.len() as usize);
                self.seen.append(&mut values);
                true
            });
            self.read_cpu += crate::thread_cpu() - start;
            self.opened = true;
        }
        if self.pending.contains_key(key) || self.seen.contains_key(key) {
            return;
        }
        let (bytes, cpu) = (self.base.read_bytes, self.base.read_cpu);
        self.base.open(dir, key);
        self.read_bytes += self.base.read_bytes - bytes;
        self.read_cpu += self.base.read_cpu - cpu;
    }

    pub fn get(&self, key: &Key) -> Option<&T> {
        self.pending.get(key).or_else(|| self.seen.get(key)).or_else(|| self.base.get(key))
    }

    pub fn insert(&mut self, key: Key, value: T) {
        self.pending.insert(key, value);
    }

    pub fn dirty(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn index(&self) -> &Index {
        &self.base.index
    }

    pub fn flush(&mut self, dir: &Path, obsolete: &mut Vec<std::path::PathBuf>) -> Res<usize> {
        if self.pending.is_empty() {
            return Ok(0);
        }
        let written = if self.base.index.is_empty() || self.layers.len() >= 32 {
            self.seen.append(&mut self.pending);
            for (key, value) in std::mem::take(&mut self.seen) {
                self.base.open(dir, &key);
                self.base.insert(key, value);
            }
            let bytes = self.base.flush(dir, obsolete)?;
            for digest in self.layers.drain(..) {
                if !self.base.index.values().any(|key| *key == digest) {
                    obsolete.push(table(dir, self.name, digest));
                }
            }
            bytes
        } else {
            let bytes = postcard::to_stdvec(&self.pending).map_err(io::Error::other)?;
            let digest = *blake3::hash(&bytes).as_bytes();
            atomic(dir, &table(dir, self.name, digest), &bytes)?;
            self.layers.push(digest);
            self.seen.append(&mut self.pending);
            bytes.len()
        };
        self.write_bytes += written;
        Ok(written)
    }
}

/// File contributions are much larger than request indexes. A shard has a sorted
/// postcard directory followed by independent postcard records. A closure can
/// read one record without deserializing any neighboring file's columns.
pub type Location = (u64, usize, Key);
pub type Directory = BTreeMap<Key, Location>;
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Segment {
    pub name: Key,
    pub directory: Key,
    pub bytes: usize,
    pub length: usize,
}
pub type Segments = BTreeMap<u8, Segment>;
pub struct Records<T> {
    pub name: &'static str,
    pub index: Segments,
    pub directories: BTreeMap<u8, (fs::File, Directory, u64)>,
    pub rows: BTreeMap<Key, T>,
    pub mapped: BTreeMap<u8, Arc<memmap2::Mmap>>,
    pub unavailable: BTreeSet<u8>,
    pub changed: BTreeSet<Key>,
    pub read_bytes: usize,
    pub read_cpu: f64,
    pub write_bytes: usize,
}
impl<T: Serialize + Default> Records<T> {
    pub fn new(name: &'static str, index: Segments) -> Self {
        Self {
            name,
            index,
            directories: BTreeMap::new(),
            rows: BTreeMap::new(),
            mapped: BTreeMap::new(),
            unavailable: BTreeSet::new(),
            changed: BTreeSet::new(),
            read_bytes: 0,
            read_cpu: 0.0,
            write_bytes: 0,
        }
    }
    pub fn part(&mut self, dir: &Path, prefix: u8) -> Option<&(fs::File, Directory, u64)> {
        use std::os::unix::fs::FileExt;
        if self.unavailable.contains(&prefix) {
            return None;
        }
        if !self.directories.contains_key(&prefix) {
            let read = || {
                let s = self.index.get(&prefix)?;
                let file = fs::File::open(table(dir, self.name, s.name)).ok()?;
                if s.bytes > s.length || s.length as u64 != file.metadata().ok()?.len() {
                    return None;
                }
                let mut raw = vec![0; s.bytes];
                file.read_exact_at(&mut raw, 0).ok()?;
                if blake3::hash(&raw).as_bytes() != &s.directory {
                    return None;
                }
                let entries = postcard::from_bytes(&raw).ok()?;
                Some((file, entries, s.bytes as u64))
            };
            if let Some(part) = read() {
                self.read_bytes += part.2 as usize;
                self.directories.insert(prefix, part);
            } else {
                self.unavailable.insert(prefix);
                return None;
            }
        }
        self.directories.get(&prefix)
    }
    pub fn open(&mut self, dir: &Path, k: &Key) where T: DeserializeOwned {
        use std::os::unix::fs::FileExt;
        if !self.index.contains_key(&k[0]) || self.rows.contains_key(k) {
            return;
        }
        let start = crate::thread_cpu();
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
        self.read_cpu += crate::thread_cpu() - start;
    }
    pub fn open_mapped(&mut self, dir: &Path, key: &Key, decode: impl FnOnce(MappedRecord) -> Option<T>) {
        if !self.index.contains_key(&key[0]) || self.rows.contains_key(key) {
            return;
        }
        let start = crate::thread_cpu();
        if let Some((record, len)) = self.mapped_record(dir, key)
            && let Some(value) = decode(record)
        {
            self.read_bytes += len;
            self.rows.insert(*key, value);
        }
        self.read_cpu += crate::thread_cpu() - start;
    }

    pub fn mapped_record(&mut self, dir: &Path, key: &Key) -> Option<(MappedRecord, usize)> {
        let prefix = key[0];
        self.part(dir, prefix)?;
        let (file, entries, base) = self.directories.get(&prefix)?;
        let (offset, len, digest) = *entries.get(key)?;
        let first = usize::try_from(base.checked_add(offset)?).ok()?;
        let range = first..first.checked_add(len)?;
        let map = match self.mapped.get(&prefix) {
            Some(map) => map.clone(),
            None => {
                // Published segment inodes are immutable. The writer creates a
                // new inode and can unlink the old name, which leaves this map live.
                let map = Arc::new(unsafe { memmap2::MmapOptions::new().map(file).ok()? });
                self.mapped.insert(prefix, map.clone());
                map
            }
        };
        let bytes = map.get(range.clone())?;
        if blake3::hash(bytes).as_bytes() != &digest {
            return None;
        }
        Some((MappedRecord { map, range }, len))
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
            atomic(dir, &table(dir, self.name, name), &bytes)?;
            written += bytes.len();
            if let Some(old) = self.index.insert(
                prefix,
                Segment {
                    name,
                    directory,
                    bytes: len,
                    length: bytes.len(),
                },
            ) && old.name != name
            {
                obsolete.push(table(dir, self.name, old.name));
            }
            self.directories.remove(&prefix);
            self.mapped.remove(&prefix);
            self.unavailable.remove(&prefix);
        }
        self.changed.clear();
        self.write_bytes += written;
        Ok(written)
    }
}


pub struct MappedRecord {
    pub map: Arc<memmap2::Mmap>,
    pub range: Range<usize>,
}
impl MappedRecord {
    pub fn postcard_bytes(mut self) -> Option<Self> {
        let bytes = postcard::from_bytes::<&[u8]>(self.as_ref()).ok()?;
        self.range.start = self.range.end.checked_sub(bytes.len())?;
        Some(self)
    }
}
impl AsRef<[u8]> for MappedRecord {
    fn as_ref(&self) -> &[u8] {
        &self.map[self.range.clone()]
    }
}
