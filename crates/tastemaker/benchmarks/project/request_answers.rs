//! A packet preserves caller order, repeated rows and typed nulls. The capture
//! boundary also consumes the canonical findings digest; live calls use the rows.
use super::super::Receipt;
use super::{Key, Value, finish, key, kind, session};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
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

#[derive(Default)]
pub(super) struct Observations {
    pub hash_cpu: f64,
    pub hash_bytes: usize,
    paths: HashMap<(PathBuf, Probe), Option<Key>>,
    inodes: HashMap<(u64, u64, i64, i64), Key>,
    pub stamps: HashMap<PathBuf, (u64, u64, i64, i64)>,
    values: HashMap<(u32, Probe), Option<Key>>,
    pub root: Option<PathBuf>,
    pub inventory: Key,
    pub overlay: HashMap<PathBuf, Key>,
    pub groups: HashMap<Key, Option<Key>>,
}
impl Observations {
    pub fn overlay(
        &mut self,
        root: &Path,
        inventory: &[String],
        peers: Key,
        values: &BTreeMap<String, std::rc::Rc<[u8]>>,
    ) {
        self.root = Some(root.to_owned());
        self.inventory = key((inventory, peers));
        self.overlay = values
            .iter()
            .map(|(p, data)| (p.into(), *blake3::hash(data).as_bytes()))
            .collect();
        self.groups.clear();
        self.values
            .retain(|(_, mode), _| !matches!(mode, Probe::Effective | Probe::Inventory));
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
        if let Some(v) = self.paths.get(&(p.to_owned(), mode)) {
            return *v;
        }
        let v = (|| {
            if matches!(mode, Probe::Kind) {
                return kind(p).ok().map(key);
            }
            let m = fs::metadata(p).ok()?;
            let stamp = (m.dev(), m.ino(), m.ctime(), m.ctime_nsec());
            let digest = if let Some(v) = self.inodes.get(&stamp) {
                *v
            } else {
                let start = crate::thread_cpu();
                let bytes = fs::read(p).ok()?;
                let v = *blake3::hash(&bytes).as_bytes();
                self.hash_cpu += crate::thread_cpu() - start;
                self.hash_bytes += bytes.len();
                self.inodes.insert(stamp, v);
                v
            };
            self.stamps.insert(p.to_owned(), stamp);
            Some(if matches!(mode, Probe::Source) {
                key((digest, m.mode() & 0o111))
            } else {
                digest
            })
        })();
        self.paths.insert((p.to_owned(), mode), v);
        v
    }
    pub fn group(&mut self, id: Key, paths: &Group, names: &[PathBuf]) -> Option<Key> {
        if let Some(v) = self.groups.get(&id) {
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
        use ryo_values::domain::row_texts::types::Cell;
        let mut words = Vec::new();
        let mut dictionary = HashMap::new();
        fn intern<'a>(
            s: &'a str,
            words: &mut Vec<Box<str>>,
            dictionary: &mut HashMap<&'a str, u32>,
        ) -> u32 {
            *dictionary.entry(s).or_insert_with(|| {
                let id = u32::try_from(words.len()).expect("packet dictionary");
                words.push(s.into());
                id
            })
        }
        let mut schemas = Vec::new();
        let mut ids = BTreeMap::new();
        let rows = receipt
            .rows
            .iter()
            .map(|r| {
                let shape = (
                    &r.rule,
                    r.columns.iter().map(|(n, _)| n).collect::<Vec<_>>(),
                    &r.identity,
                );
                let schema = *ids.entry(shape).or_insert_with(|| {
                    let i = schemas.len();
                    schemas.push(Schema {
                        rule: r.rule.clone(),
                        columns: r.columns.iter().map(|(n, _)| n.clone()).collect(),
                        identity: r.identity.clone(),
                    });
                    i
                });
                PackedRow {
                    schema,
                    values: r
                        .columns
                        .iter()
                        .map(|(_, c)| {
                            c.as_ref().map(|v| match v {
                                Cell::File(s) => {
                                    PackedValue::File(intern(s, &mut words, &mut dictionary))
                                }
                                Cell::Text(s) => {
                                    PackedValue::Text(intern(s, &mut words, &mut dictionary))
                                }
                                Cell::Number(n) => PackedValue::Number(n.0),
                                Cell::Span(s) => PackedValue::Span(s.start.0, s.end.0),
                            })
                        })
                        .collect(),
                }
            })
            .collect();
        Self {
            refusal: None,
            digest: super::super::digest(&receipt.rows),
            unknowns: receipt.unknowns.iter().map(|v| (*v).clone()).collect(),
            data: postcard::to_stdvec(&Rows {
                words,
                schemas,
                rows,
            })
            .expect("typed request rows"),
        }
    }
    pub(in super::super) fn report(&self) -> session::Reported {
        let Rows {
            words,
            schemas,
            rows,
        } = postcard::from_bytes(&self.data).expect("verified request packet");
        session::Reported {
            unknowns: self.unknowns.clone(),
            rows: rows
                .into_iter()
                .map(|r| {
                    let s = &schemas[r.schema];
                    finish::answer::Row {
                        rule: s.rule.clone(),
                        identity: s.identity.clone(),
                        columns: s
                            .columns
                            .iter()
                            .cloned()
                            .zip(r.values.into_iter().map(|c| {
                                c.map(|v| match v {
                                    PackedValue::File(i) => {
                                        Value::File(words[i as usize].clone()).cell()
                                    }
                                    PackedValue::Text(i) => {
                                        Value::Text(words[i as usize].clone()).cell()
                                    }
                                    PackedValue::Number(n) => Value::Number(n).cell(),
                                    PackedValue::Span(a, b) => Value::Span(a, b).cell(),
                                })
                            }))
                            .collect(),
                    }
                })
                .collect(),
        }
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
