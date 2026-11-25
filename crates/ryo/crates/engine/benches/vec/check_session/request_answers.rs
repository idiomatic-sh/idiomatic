//! A packet preserves caller order, repeated rows and typed nulls. The capture
//! boundary also consumes the canonical findings digest; live calls use the rows.
use super::super::Receipt;
use super::{Key, Shards, finish, key, kind, session};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
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

/// Identifies the immutable evidence supplied by one admission in this store.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(in super::super) struct InputScope {
    overlay: usize,
    inventory: usize,
}

struct RequestInputs {
    root: PathBuf,
    supplied: BTreeMap<String, Key>,
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
    kinds: HashMap<PathBuf, Option<u8>>,
    buffer: Vec<u8>,
    pub path_hits: usize,
    pub inode_hits: usize,
    pub reads: [usize; 3],
    pub bytes: [usize; 3],
    pub group_hits: usize,
    inodes: HashMap<Stamp, Key>,
    values: FxHashMap<(u32, Probe, Option<usize>), Option<Key>>,
    groups: FxHashMap<(Key, Option<usize>, Option<usize>), Option<Key>>,
    group_inputs: FxHashMap<Key, (bool, bool)>,
    scopes: Vec<RequestInputs>,
    scope_ids: FxHashMap<Key, usize>,
    inventories: Vec<Key>,
    inventory_ids: FxHashMap<Key, usize>,
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
            groups: FxHashMap::default(),
            group_inputs: FxHashMap::default(),
            scopes: Vec::new(),
            scope_ids: FxHashMap::default(),
            inventories: Vec::new(),
            inventory_ids: FxHashMap::default(),
        }
    }
    pub fn request_inputs(
        &mut self,
        root: &Path,
        inventory: Key,
        peers: Key,
        supplied: BTreeMap<String, Key>,
    ) -> InputScope {
        let id = key((root, &supplied));
        let overlay = *self.scope_ids.entry(id).or_insert_with(|| {
            let index = self.scopes.len();
            self.scopes.push(RequestInputs { root: root.to_owned(), supplied });
            index
        });
        let digest = key((inventory, peers));
        let inventory = *self.inventory_ids.entry(digest).or_insert_with(|| {
            let index = self.inventories.len();
            self.inventories.push(digest);
            index
        });
        InputScope { overlay, inventory }
    }

    fn observe_in(&mut self, p: &Path, mode: Probe, overlay: Option<usize>) -> Option<Key> {
        match mode {
            Probe::Effective => {
                let input = self.scopes.get(overlay?)?;
                if let Some(value) = input.supplied.get(p.to_str()?) {
                    return Some(*value);
                }
                let path = input.root.join(p);
                self.observe(&path, Probe::Source)
            }
            _ => self.observe(p, mode),
        }
    }

    pub(super) fn observe_kind(&mut self, path: &Path) -> Option<u8> {
        if let Some(&state) = self.kinds.get(path) {
            self.path_hits += 1;
            return state;
        }
        let state = kind(path).ok();
        self.kinds.insert(path.to_owned(), state);
        state
    }

    pub fn observe(&mut self, p: &Path, mode: Probe) -> Option<Key> {
        debug_assert!(!matches!(mode, Probe::Effective | Probe::Inventory));
        if mode == Probe::Kind {
            return self.observe_kind(p).map(key);
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
                .all(|(path, prior)| kind(path).ok() == *prior)
    }
    pub fn checked_groups(&self) -> usize {
        self.groups.len()
    }

    fn leaf(&mut self, i: u32, mode: Probe, names: &[PathBuf], scope: Option<InputScope>) -> Option<Key> {
        if mode == Probe::Inventory {
            return scope.map(|scope| self.inventories[scope.inventory]);
        }
        let overlay = if mode == Probe::Effective { Some(scope?.overlay) } else { None };
        let binding = (i, mode, overlay);
        if let Some(digest) = self.values.get(&binding) {
            return *digest;
        }
        let digest = self.observe_in(names.get(i as usize)?, mode, overlay);
        self.values.insert(binding, digest);
        digest
    }

    pub fn has_prior_sources(&self) -> bool {
        !self.prior_sources.is_empty()
    }

    /// A saved stamp is a candidate old leaf. Reconstructing the saved root
    /// proves that the cached receipt differs at precisely this physical input.
    pub fn prior_leaf(
        &mut self, group: &Group, names: &[PathBuf], scope: Option<InputScope>, expected: Key,
    ) -> Option<(PathBuf, Key)> {
        let mut hash = super::Digest::default();
        group.len().hash(&mut hash);
        let mut changed = None;
        for &(i, mode) in group {
            let mut digest = self.leaf(i, mode, names, scope)?;
            let path = names.get(i as usize)?;
            let prior = match mode {
                Probe::Source => self.prior_sources.get_key_value(path),
                Probe::Effective => {
                    let input = self.scopes.get(scope?.overlay)?;
                    if input.supplied.contains_key(path.to_str()?) {
                        None
                    } else {
                        self.prior_sources.get_key_value(&input.root.join(path))
                    }
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

    pub fn group(&mut self, id: Key, paths: &Group, names: &[PathBuf], scope: Option<InputScope>) -> Option<Key> {
        let (effective, closed) = *self.group_inputs.entry(id).or_insert_with(|| {
            (paths.iter().any(|&(_, mode)| mode == Probe::Effective),
             paths.iter().any(|&(_, mode)| mode == Probe::Inventory))
        });
        let overlay = if effective { Some(scope?.overlay) } else { None };
        let inventory = if closed { Some(scope?.inventory) } else { None };
        let group = (id, overlay, inventory);
        if let Some(v) = self.groups.get(&group) {
            self.group_hits += 1;
            return *v;
        }
        let mut hash = super::Digest::default();
        paths.len().hash(&mut hash);
        let result = paths.iter().try_for_each(|&(i, mode)| {
            let digest = self.leaf(i, mode, names, scope)?;
            digest.hash(&mut hash);
            Some(())
        }).map(|()| *hash.0.finalize().as_bytes());
        self.groups.insert(group, result);
        result
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Record {
    pub proof: Vec<Proof>,
    pub packet: Key,
    pub unread: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize)]
struct Schema {
    rule: Box<str>,
    columns: Vec<Box<str>>,
    identity: Vec<usize>,
}
#[derive(Serialize, Deserialize)]
struct PackedRow {
    place: (u32, u32),
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
pub(in super::super) struct Packet {
    pub rejected_syntax: Vec<String>,
    pub refusal: Option<String>,
    pub digest: String,
    pub unknowns: Vec<(String, String)>,
    #[serde(with = "payload")]
    data: Vec<u8>,
}
impl Packet {
    pub(super) fn declined(reason: &str) -> Self {
        Self {
            rejected_syntax: Vec::new(),
            refusal: Some(reason.to_owned()),
            digest: String::new(),
            unknowns: vec![],
            data: vec![],
        }
    }
    pub(super) fn typed_bytes(&self) -> &[u8] {
        &self.data
    }
    pub(super) fn save(receipt: &Receipt<'_>) -> Self {
        Self {
            refusal: None,
            digest: String::new(),
            rejected_syntax: receipt.rejected_syntax.iter().map(|v| (*v).clone()).collect(),
            unknowns: receipt.unknowns.iter().map(|v| (*v).clone()).collect(),
            data: pack_rows(receipt.rows.iter().copied()),
        }
    }
    pub(in super::super) fn report(&self) -> session::Reported {
        session::Reported::new(unpack_rows(&self.data), self.unknowns.clone(), self.rejected_syntax.clone())
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
        PackedRow { schema, values, place: r.place }
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
        let mut row = restored.row(r.schema, &schema.rule, &schema.columns, &schema.identity, cells);
        row.place = r.place;
        row
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interleaved_requests_keep_their_own_effective_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        fs::write(first.join("file"), b"one").unwrap();
        fs::write(second.join("file"), b"two").unwrap();

        let mut observed = Observations::new(dir.path().to_owned(), Default::default());
        let inventory = key("inventory");
        let peers = key("peers");
        let disk = observed.request_inputs(&first, inventory, peers, BTreeMap::new());
        let other_disk = observed.request_inputs(&second, inventory, peers, BTreeMap::new());
        let other_inventory = observed.request_inputs(&first, key("other inventory"), peers, BTreeMap::new());
        let supplied = observed.request_inputs(
            &first, key("other inventory"), peers, BTreeMap::from([("file".into(), key("supplied"))]),
        );
        let names = ["file".into(), PathBuf::new()];
        let paths = vec![(0, Probe::Effective)];
        let id = key(&paths);
        assert_eq!(observed.group(id, &paths, &names, None), None);
        let value = observed.group(id, &paths, &names, Some(disk)).unwrap();
        assert_eq!(observed.group(id, &paths, &names, Some(other_inventory)), Some(value));
        assert_eq!(observed.checked_groups(), 1);
        assert_ne!(observed.group(id, &paths, &names, Some(other_disk)), Some(value));
        assert_ne!(observed.group(id, &paths, &names, Some(supplied)), Some(value));
        assert_eq!(observed.group(id, &paths, &names, Some(disk)), Some(value));

        let paths = vec![(1, Probe::Inventory)];
        let id = key(&paths);
        let value = observed.group(id, &paths, &names, Some(disk));
        assert_eq!(observed.group(id, &paths, &names, Some(other_disk)), value);
        assert_eq!(observed.checked_groups(), 4);
        assert_ne!(observed.group(id, &paths, &names, Some(supplied)), value);
    }

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
                PackedRow { place: (2, 3), schema: 0, values: vec![Some(PackedValue::File(0)), Some(PackedValue::Number(-1))] },
                PackedRow { place: (2, 3), schema: 0, values: vec![Some(PackedValue::Text(0)), None] },
                PackedRow { place: (2, 3), schema: 0, values: vec![Some(PackedValue::File(0)), Some(PackedValue::Number(-1))] },
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
