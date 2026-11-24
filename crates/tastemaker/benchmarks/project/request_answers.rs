//! A packet preserves caller order, repeated rows and typed nulls. The capture
//! boundary also consumes the canonical findings digest; live calls use the rows.
use super::super::Receipt;
use super::{Key, finish, key, kind, session};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
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

type Stamp = (u64, u64, i64, i64);
fn stamp(m: &fs::Metadata) -> Stamp {
    (m.dev(), m.ino(), m.ctime(), m.ctime_nsec())
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

#[derive(Default)]
pub(super) struct Observations {
    pub hash_cpu: f64,
    pub hash_bytes: usize,
    pub open_cpu: f64,
    files: HashMap<PathBuf, FileObservation>,
    kinds: HashMap<PathBuf, Option<Key>>,
    buffer: Vec<u8>,
    pub path_hits: usize,
    pub inode_hits: usize,
    pub reads: [usize; 3],
    pub bytes: [usize; 3],
    pub group_hits: usize,
    inodes: HashMap<Stamp, Key>,
    values: HashMap<(u32, Probe), Option<Key>>,
    pub root: Option<PathBuf>,
    pub inventory: Key,
    pub overlay: HashMap<PathBuf, Key>,
    pub groups: HashMap<Key, Option<Key>>,
    effective_groups: HashSet<Key>,
    inventory_groups: HashSet<Key>,
}
impl Observations {
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
        let start = crate::thread_cpu();
        let mut input = fs::File::open(p).ok()?;
        let metadata = input.metadata().ok()?;
        self.open_cpu += crate::thread_cpu() - start;
        let stamp = stamp(&metadata);
        let bytes = if let Some(v) = self.inodes.get(&stamp) {
            self.inode_hits += 1;
            *v
        } else {
            let start = crate::thread_cpu();
            self.buffer.clear();
            self.buffer
                .try_reserve(usize::try_from(metadata.len()).ok()?)
                .ok()?;
            input.read_to_end(&mut self.buffer).ok()?;
            let digest = *blake3::hash(&self.buffer).as_bytes();
            self.hash_cpu += crate::thread_cpu() - start;
            self.hash_bytes += self.buffer.len();
            self.reads[mode as usize] += 1;
            self.bytes[mode as usize] += self.buffer.len();
            self.inodes.insert(stamp, digest);
            digest
        };
        let mut file = FileObservation {
            bytes,
            executable: metadata.mode() & 0o111,
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
            .all(|(path, file)| fs::metadata(path).is_ok_and(|m| stamp(&m) == file.stamp))
            && self
                .kinds
                .iter()
                .all(|(path, prior)| kind(path).ok().map(key) == *prior)
    }
    pub fn group(&mut self, id: Key, paths: &Group, names: &[PathBuf]) -> Option<Key> {
        if let Some(v) = self.groups.get(&id) {
            self.group_hits += 1;
            return *v;
        }
        let result = paths
            .iter()
            .map(|&(i, m)| {
                if let Some(d) = self.values.get(&(i, m)) {
                    return *d;
                }
                let d = self.observe(names.get(i as usize)?, m);
                self.values.insert((i, m), d);
                d
            })
            .collect::<Option<Vec<_>>>()
            .map(key);
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
struct Rows {
    words: Vec<Box<str>>,
    schemas: Vec<Schema>,
    rows: Vec<PackedRow>,
}

#[derive(Serialize, Deserialize)]
pub(in super::super) struct Packet {
    pub refusal: Option<String>,
    pub digest: String,
    pub unknowns: Vec<(String, String)>,
    #[serde(with = "payload")]
    data: Vec<u8>,
}
impl Packet {
    pub(super) fn declined(reason: &str) -> Self {
        Self {
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
            unknowns: receipt.unknowns.iter().map(|v| (*v).clone()).collect(),
            data: pack_rows(receipt.rows.iter().copied()),
        }
    }
    pub(in super::super) fn report(&self) -> session::Reported {
        session::Reported {
            unknowns: self.unknowns.clone(),
            rows: unpack_rows(&self.data),
        }
    }
}

pub(super) fn pack_rows<'a>(input: impl IntoIterator<Item = &'a finish::answer::Row>) -> Vec<u8> {
    use ryo_values::domain::row_texts::types::Cell;
    use std::sync::Arc;

    fn intern(word: Arc<str>, words: &mut Vec<Box<str>>, dictionary: &mut HashMap<Arc<str>, u32>) -> u32 {
        *dictionary.entry(word).or_insert_with_key(|s| {
            let i = u32::try_from(words.len()).expect("packet dictionary");
            words.push(s.as_ref().into());
            i
        })
    }

    let mut words = Vec::new();
    let mut dictionary = HashMap::new();
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

mod payload {
    use serde::{Deserialize, Deserializer, Serializer};
    pub(super) fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(bytes)
    }
    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        Ok(<&[u8]>::deserialize(d)?.to_vec())
    }
}
