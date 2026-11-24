//! A packet preserves caller order, repeated rows and typed nulls. The capture
//! boundary also consumes the canonical findings digest; live calls use the rows.
use super::super::Receipt;
use super::{Key, Shards, finish, key, kind, session};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    hash::Hash,
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub(super) enum Probe {
    Source,
    Bytes,
    Kind,
    Effective,
    Inventory,
}
pub(super) type Group = Vec<(u32, Probe)>;
pub(super) type Proof = (Key, Key);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
struct Stamp {
    device: u64,
    inode: u64,
    length: u64,
    mode: u32,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl Stamp {
    fn from_metadata(m: &fs::Metadata) -> Self {
        Self {
            device: m.dev(),
            inode: m.ino(),
            length: m.len(),
            mode: m.mode(),
            modified: (m.mtime(), m.mtime_nsec()),
            changed: (m.ctime(), m.ctime_nsec()),
        }
    }

    // A whole second is a conservative tick even when this filesystem's
    // timestamps carry only second precision. Include future timestamps.
    fn older_than(self, second: i64) -> bool {
        self.modified.0 < second && self.changed.0 < second
    }
}
#[derive(Serialize, Deserialize)]
pub(super) struct SavedStamp {
    stamp: Stamp,
    bytes: Key,
}

struct FileObservation {
    bytes: Key,
    executable: u32,
    source: Option<Key>,
    stamp: Stamp,
}
impl FileObservation {
    fn digest(&mut self, mode: Probe) -> Key {
        if mode == Probe::Source {
            *self
                .source
                .get_or_insert_with(|| key((self.bytes, self.executable)))
        } else {
            self.bytes
        }
    }
}

pub(super) struct Observations {
    dir: PathBuf,
    started: i64,
    pub stamps: Shards<SavedStamp>,
    pub stats: usize,
    pub stamp_hits: usize,
    pub young: usize,
    pub stamp_changed: usize,
    pub hash_cpu: f64,
    pub hash_bytes: usize,
    pub open_cpu: f64,
    files: HashMap<PathBuf, FileObservation>,
    prior_sources: HashMap<PathBuf, Key>,
    kinds: HashMap<PathBuf, Option<Key>>,
    buffer: Vec<u8>,
    pub path_hits: usize,
    pub inode_hits: usize,
    pub reads: [usize; 3],
    pub bytes: [usize; 3],
    pub group_hits: usize,
    inodes: HashMap<Stamp, Key>,
    values: FxHashMap<(u32, Probe), Option<Key>>,
    pub root: Option<PathBuf>,
    pub inventory: Key,
    pub overlay: HashMap<PathBuf, Key>,
    pub groups: FxHashMap<Key, Option<Key>>,
    effective_groups: HashSet<Key>,
    inventory_groups: HashSet<Key>,
}
impl Observations {
    pub fn new(dir: PathBuf, index: super::Index) -> Self {
        Self {
            dir,
            started: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|t| i64::try_from(t.as_secs()).ok())
                .unwrap_or(i64::MIN),
            stamps: Shards::new("stamps", index),
            stats: 0,
            stamp_hits: 0,
            young: 0,
            stamp_changed: 0,
            hash_cpu: 0.0,
            hash_bytes: 0,
            open_cpu: 0.0,
            files: HashMap::new(),
            prior_sources: HashMap::new(),
            kinds: HashMap::new(),
            buffer: Vec::new(),
            path_hits: 0,
            inode_hits: 0,
            reads: [0; 3],
            bytes: [0; 3],
            group_hits: 0,
            inodes: HashMap::new(),
            values: FxHashMap::default(),
            root: None,
            inventory: [0; 32],
            overlay: HashMap::new(),
            groups: FxHashMap::default(),
            effective_groups: HashSet::new(),
            inventory_groups: HashSet::new(),
        }
    }
    pub fn overlay(
        &mut self,
        root: &Path,
        inventory: Key,
        peers: Key,
        values: &BTreeMap<String, std::sync::Arc<[u8]>>,
    ) {
        let overlay = values
            .iter()
            .map(|(p, data)| (p.into(), *blake3::hash(data).as_bytes()))
            .collect();
        let effective_changed = self.root.as_deref() != Some(root) || self.overlay != overlay;
        let inventory = key((inventory, peers));
        let inventory_changed = self.inventory != inventory;
        if effective_changed {
            self.overlay = overlay;
            self.root = Some(root.to_owned());
            for id in self.effective_groups.drain() {
                self.groups.remove(&id);
            }
        }
        if inventory_changed {
            self.inventory = inventory;
            for id in self.inventory_groups.drain() {
                self.groups.remove(&id);
            }
        }
        if effective_changed || inventory_changed {
            self.values.retain(|(_, mode), _| {
                !(effective_changed && *mode == Probe::Effective
                    || inventory_changed && *mode == Probe::Inventory)
            });
        }
    }
    pub fn observe(&mut self, p: &Path, mode: Probe) -> Option<Key> {
        if matches!(mode, Probe::Inventory) {
            return Some(self.inventory);
        }
        if matches!(mode, Probe::Effective) {
            if let Some(v) = self.overlay.get(p) {
                return Some(*v);
            }
            let file = self.root.as_ref()?.join(p);
            return self.observe(&file, Probe::Source);
        }
        if mode == Probe::Kind {
            if let Some(v) = self.kinds.get(p) {
                self.path_hits += 1;
                return *v;
            }
            let v = kind(p).ok().map(key);
            self.kinds.insert(p.to_owned(), v);
            return v;
        }
        if let Some(file) = self.files.get_mut(p) {
            self.path_hits += 1;
            return Some(file.digest(mode));
        }
        let name = key(p);
        self.stamps.open(&self.dir, &name);
        let saved = self.stamps.get(&name);
        let start = crate::thread_cpu();
        // Following the pathname once suffices for a hit. A miss uses fresh
        // descriptor metadata and verifies that the descriptor stayed stable.
        let mut input = None;
        let metadata = if saved.is_some() {
            self.stats += 1;
            fs::metadata(p).ok()?
        } else {
            let f = fs::File::open(p).ok()?;
            let metadata = f.metadata().ok()?;
            input = Some(f);
            metadata
        };
        self.open_cpu += crate::thread_cpu() - start;
        if !metadata.is_file() {
            return None;
        }
        let mut stamp = Stamp::from_metadata(&metadata);
        let trusted = stamp.older_than(self.started);
        let matches = saved.is_some_and(|s| s.stamp == stamp);
        self.young += usize::from(!trusted);
        self.stamp_changed += usize::from(saved.is_some() && !matches);
        let bytes = if let Some(saved) = saved.filter(|_| trusted && matches) {
            self.stamp_hits += 1;
            saved.bytes
        } else if let Some(v) = self.inodes.get(&stamp).filter(|_| trusted) {
            self.inode_hits += 1;
            *v
        } else {
            let start = crate::thread_cpu();
            let mut input = match input {
                Some(f) => f,
                None => {
                    let f = fs::File::open(p).ok()?;
                    let m = f.metadata().ok()?;
                    if !m.is_file() {
                        return None;
                    }
                    stamp = Stamp::from_metadata(&m);
                    f
                }
            };
            let size = usize::try_from(stamp.length).ok()?;
            self.buffer.try_reserve(size.saturating_sub(self.buffer.len())).ok()?;
            self.buffer.resize(size, 0);
            input.read_exact(&mut self.buffer).ok()?;
            if input.read(&mut [0]).ok()? != 0
                || Stamp::from_metadata(&input.metadata().ok()?) != stamp
            {
                return None;
            }
            let digest = *blake3::hash(&self.buffer).as_bytes();
            self.hash_cpu += crate::thread_cpu() - start;
            self.hash_bytes += self.buffer.len();
            self.reads[mode as usize] += 1;
            self.bytes[mode as usize] += self.buffer.len();
            self.inodes.insert(stamp, digest);
            digest
        };
        let executable = stamp.mode & 0o111;
        if let Some(saved) = saved.filter(|s| s.bytes != bytes || s.stamp.mode & 0o111 != executable) {
            self.prior_sources.insert(p.to_owned(), key((saved.bytes, saved.stamp.mode & 0o111)));
        }
        // A young observation can support the current answer by its content
        // hash, but it cannot vouch for a later process. Stale entries cannot
        // match its new stamp and are replaced on the next eligible miss.
        if stamp.older_than(self.started)
            && !saved.is_some_and(|s| s.stamp == stamp && s.bytes == bytes)
        {
            self.stamps.insert(name, SavedStamp { stamp, bytes });
        }
        let mut file = FileObservation {
            bytes,
            executable,
            source: None,
            stamp,
        };
        let digest = file.digest(mode);
        self.files.insert(p.to_owned(), file);
        Some(digest)
    }
    pub fn still_current(&self) -> bool {
        self.files
            .iter()
            .all(|(path, file)| {
                fs::metadata(path).is_ok_and(|m| Stamp::from_metadata(&m) == file.stamp)
                    && (file.stamp.older_than(self.started)
                        || fs::read(path).is_ok_and(|bytes| blake3::hash(&bytes).as_bytes() == &file.bytes))
            })
            && self
                .kinds
                .iter()
                .all(|(path, prior)| kind(path).ok().map(key) == *prior)
    }
    fn leaf(&mut self, i: u32, mode: Probe, paths: &[PathBuf]) -> Option<Key> {
        if let Some(d) = self.values.get(&(i, mode)) {
            return *d;
        }
        let digest = self.observe(paths.get(i as usize)?, mode);
        self.values.insert((i, mode), digest);
        digest
    }

    /// A saved stamp supplies a candidate old leaf. The old Merkle root must
    /// still verify, since an earlier check may have observed another version.
    pub fn has_prior_sources(&self) -> bool {
        !self.prior_sources.is_empty()
    }

    pub fn prior_leaf(
        &mut self, group: &Group, paths: &[PathBuf], expected: Key,
    ) -> Option<(PathBuf, Key)> {
        let mut hash = super::Digest::default();
        group.len().hash(&mut hash);
        let mut changed = None;
        for &(i, mode) in group {
            let mut digest = self.leaf(i, mode, paths)?;
            let path = paths.get(i as usize)?;
            let prior = match mode {
                Probe::Source => self.prior_sources.get_key_value(path),
                Probe::Effective if !self.overlay.contains_key(path) => {
                    let root = self.root.as_ref()?;
                    self.prior_sources.iter().find(|(physical, _)| physical.strip_prefix(root).ok() == Some(path))
                }
                _ => None,
            };
            if let Some((path, &prior)) = prior {
                if changed.is_some() {
                    return None;
                }
                changed = Some((path.clone(), prior));
                digest = prior;
            }
            digest.hash(&mut hash);
        }
        (*hash.0.finalize().as_bytes() == expected).then_some(changed).flatten()
    }

    pub fn group(&mut self, id: Key, paths: &Group, names: &[PathBuf]) -> Option<Key> {
        if let Some(v) = self.groups.get(&id) {
            self.group_hits += 1;
            return *v;
        }
        let mut hash = super::Digest::default();
        paths.len().hash(&mut hash);
        let result = paths
            .iter()
            .try_for_each(|&(i, m)| {
                let d = self.leaf(i, m, names)?;
                d.hash(&mut hash);
                Some(())
            })
            .map(|()| *hash.0.finalize().as_bytes());
        if self.root.is_some() {
            for &(_, mode) in paths {
                match mode {
                    Probe::Inventory => {
                        self.inventory_groups.insert(id);
                    }
                    Probe::Effective => {
                        self.effective_groups.insert(id);
                    }
                    _ => {}
                }
            }
        }
        self.groups.insert(id, result);
        result
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Record {
    pub proof: Vec<Proof>,
    pub packet: Key,
    pub unread: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Schema {
    rule: Box<str>,
    columns: Vec<Box<str>>,
    identity: Vec<usize>,
}
#[derive(Serialize, Deserialize)]
struct PackedRow {
    schema: usize,
    values: Vec<Option<PackedValue>>,
}
#[derive(Serialize, Deserialize)]
enum PackedValue {
    File(u32),
    Text(u32),
    Number(i64),
    Span(u32, u32),
}
#[derive(Serialize, Deserialize)]
struct Rows<Word = Box<str>> {
    words: Vec<Word>,
    schemas: Vec<Schema>,
    rows: Vec<PackedRow>,
}

#[derive(Serialize, Deserialize)]
struct Replacement {
    base: Key,
    copied: Vec<CopiedRange>,
}

#[derive(Serialize, Deserialize)]
struct CopiedRange {
    destination: usize,
    source: usize,
    length: usize,
}

#[derive(Serialize, Deserialize)]
pub(in super::super) struct Packet {
    pub refusal: Option<String>,
    pub digest: String,
    pub unknowns: Vec<(String, String)>,
    #[serde(with = "payload")]
    data: Vec<u8>,
    replacement: Option<Replacement>,
}
impl Packet {
    pub(super) fn declined(reason: &str) -> Self {
        Self {
            refusal: Some(reason.to_owned()),
            digest: String::new(),
            unknowns: vec![],
            data: vec![],
            replacement: None,
        }
    }
    pub(super) fn typed_bytes(&self) -> &[u8] {
        &self.data
    }
    pub(super) fn save(receipt: &Receipt<'_>) -> Self {
        Self {
            refusal: None,
            digest: String::new(),
            unknowns: receipt.unknowns.iter().map(|v| (*v).clone()).collect(),
            data: pack_rows(receipt.rows.iter().copied()),
            replacement: None,
        }
    }
    pub(super) fn base(&self) -> Option<Key> {
        self.replacement.as_ref().map(|part| part.base)
    }

    // Compare against one complete packet. Small insertions can move a long
    // unchanged range, so test for a new alignment before copying literals.
    pub(super) fn replace_from(&mut self, base: Key, prior: &Self) -> usize {
        const ANCHOR: usize = 64;
        if prior.replacement.is_some() || self.data.len() < ANCHOR {
            return 0;
        }
        let old = &prior.data;
        let new = &self.data;
        let (mut source, mut destination) = (0, 0);
        let mut copied = Vec::new();
        while source < old.len() && destination < new.len() && copied.len() < 64 {
            let length = old[source..].iter().zip(&new[destination..])
                .take_while(|(a,b)| a == b).count();
            if length >= ANCHOR {
                copied.push(CopiedRange { destination, source, length });
                source += length;
                destination += length;
                continue;
            }
            let next = [64, 256, 1024, 4096, 16384, 65536].into_iter().find_map(|skip| {
                let at = destination + skip;
                let needle = new.get(at..at + ANCHOR)?;
                let end = (source + skip + 8192).min(old.len());
                let relative = memchr::memmem::find(&old[source..end], needle)?;
                Some((source + relative, at))
            });
            if let Some((s, d)) = next {
                source = s;
                destination = d;
            } else {
                let length = old[source..].iter().rev().zip(new[destination..].iter().rev())
                    .take_while(|(a,b)| a == b).count();
                if length >= ANCHOR {
                    copied.push(CopiedRange { destination: new.len() - length, source: old.len() - length, length });
                }
                break;
            }
        }
        let reused = copied.iter().map(|r| r.length).sum::<usize>();
        if reused <= 64 + 24 * copied.len() {
            return 0;
        }
        let mut literals = Vec::with_capacity(new.len() - reused);
        let mut now = 0;
        for range in &copied {
            literals.extend_from_slice(&new[now..range.destination]);
            now = range.destination + range.length;
        }
        literals.extend_from_slice(&new[now..]);
        self.data = literals;
        self.replacement = Some(Replacement { base, copied });
        reused
    }

    pub(super) fn report(&self, base: Option<&Self>) -> session::Reported {
        let expanded;
        let bytes = if let Some(part) = &self.replacement {
            let old = &base.expect("checked base packet").data;
            let size = self.data.len() + part.copied.iter().map(|r| r.length).sum::<usize>();
            let mut output = Vec::with_capacity(size);
            let mut literal = 0;
            for range in &part.copied {
                let count = range.destination - output.len();
                output.extend_from_slice(&self.data[literal..literal + count]);
                literal += count;
                output.extend_from_slice(&old[range.source..range.source + range.length]);
            }
            output.extend_from_slice(&self.data[literal..]);
            expanded = output;
            expanded.as_slice()
        } else {
            &self.data
        };
        session::Reported { unknowns: self.unknowns.clone(), rows: unpack_rows(bytes) }
    }

}

pub(super) fn pack_rows<'a>(input: impl IntoIterator<Item = &'a finish::answer::Row>) -> Vec<u8> {
    use ryo_values::domain::row_texts::types::Cell;
    use std::sync::Arc;

    struct SharedWord(Arc<str>);
    impl Serialize for SharedWord {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(&self.0)
        }
    }

    fn intern(word: Arc<str>, words: &mut Vec<SharedWord>, dictionary: &mut rustc_hash::FxHashMap<Arc<str>, u32>) -> u32 {
        *dictionary.entry(word).or_insert_with_key(|s| {
            let i = u32::try_from(words.len()).expect("packet dictionary");
            words.push(SharedWord(s.clone()));
            i
        })
    }

    let mut words = Vec::new();
    let mut dictionary = rustc_hash::FxHashMap::default();
    let mut schemas = Vec::new();
    let mut ids = BTreeMap::new();
    let mut previous = None;
    let rows = input.into_iter().map(|r| {
        let schema = if let Some((prior, id)) = previous && Arc::ptr_eq(prior, &r.layout) {
            id
        } else {
            let shape = (&r.layout.rule, &r.layout.names, &r.layout.identity);
            *ids.entry(shape).or_insert_with(|| {
                let i = schemas.len();
                schemas.push(Schema {
                    rule: r.layout.rule.clone(),
                    columns: r.layout.names.iter().map(|n| n.as_str().into()).collect(),
                    identity: r.layout.identity.to_vec(),
                });
                i
            })
        };
        previous = Some((&r.layout, schema));
        let values = r.cells().map(|cell| cell.map(|v| match v {
            Cell::File(s) => PackedValue::File(intern(s, &mut words, &mut dictionary)),
            Cell::Text(s) => PackedValue::Text(intern(s, &mut words, &mut dictionary)),
            Cell::Number(n) => PackedValue::Number(n.0),
            Cell::Span(s) => PackedValue::Span(s.start.0, s.end.0),
        })).collect();
        PackedRow { schema, values }
    }).collect();
    postcard::to_stdvec(&Rows { words, schemas, rows }).expect("typed request rows")
}

pub(super) fn unpack_rows(data: &[u8]) -> Vec<finish::answer::Row> {
    use finish::Ty;
    let Rows { words, schemas, rows } = postcard::from_bytes(data).expect("verified request packet");
    let mut restored = finish::answer::RestoredRows::new(words, schemas.len());
    rows.into_iter().map(|r| {
        let schema = &schemas[r.schema];
        let cells = r.values.into_iter().map(|v| match v {
            None => (Ty::Sym, finish::NULL),
            Some(PackedValue::File(i)) => (Ty::File, u64::from(i)),
            Some(PackedValue::Text(i)) => (Ty::Sym, u64::from(i)),
            Some(PackedValue::Number(n)) => (Ty::Num, n as u64),
            Some(PackedValue::Span(a, b)) => (Ty::Span, (u64::from(a) << 32) | u64::from(b)),
        });
        restored.row(r.schema, &schema.rule, &schema.columns, &schema.identity, cells)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_timestamps_must_precede_the_snapshot_tick() {
        let mut stamp = Stamp {
            device: 1,
            inode: 2,
            length: 3,
            mode: 0o100644,
            modified: (40, 999_999_999),
            changed: (40, 999_999_999),
        };
        assert!(stamp.older_than(41));
        stamp.changed = (41, 0);
        assert!(!stamp.older_than(41));
        stamp.changed = (40, 0);
        stamp.modified = (42, 0);
        assert!(!stamp.older_than(41));
    }

    #[test]
    fn restored_rows_preserve_nulls_negative_one_and_multiplicity() {
        let rows: Rows = Rows {
            words: vec!["same bytes".into()],
            schemas: vec![Schema {
                rule: "query".into(),
                columns: vec!["subject".into(), "value".into()],
                identity: vec![0],
            }],
            rows: vec![
                PackedRow { schema: 0, values: vec![Some(PackedValue::File(0)), Some(PackedValue::Number(-1))] },
                PackedRow { schema: 0, values: vec![Some(PackedValue::Text(0)), None] },
                PackedRow { schema: 0, values: vec![Some(PackedValue::File(0)), Some(PackedValue::Number(-1))] },
            ],
        };
        let encoded = postcard::to_stdvec(&rows).unwrap();
        let restored = unpack_rows(&encoded);
        assert!(std::sync::Arc::ptr_eq(&restored[0].layout, &restored[2].layout));
        assert_eq!(pack_rows(&restored), encoded);
    }
}

mod payload {
    use serde::{Deserialize, Deserializer, Serializer};
    pub(super) fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(bytes)
    }
    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        Ok(<&[u8]>::deserialize(d)?.to_vec())
    }
}
