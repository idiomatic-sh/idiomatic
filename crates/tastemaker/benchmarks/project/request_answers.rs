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
}
pub(super) type Group = Vec<(PathBuf, Probe)>;
pub(super) type Proof = (Key, Key);

#[derive(Default)]
pub(super) struct Observations {
    pub hash_cpu: f64,
    pub hash_bytes: usize,
    paths: HashMap<(PathBuf, Probe), Option<Key>>,
    inodes: HashMap<(u64, u64, i64, i64), Key>,
    pub stamps: HashMap<PathBuf, (u64, u64, i64, i64)>,
    pub groups: HashMap<Key, Option<Key>>,
}
impl Observations {
    pub fn observe(&mut self, p: &Path, mode: Probe) -> Option<Key> {
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
    pub fn group(&mut self, id: Key, paths: &Group) -> Option<Key> {
        if let Some(v) = self.groups.get(&id) {
            return *v;
        }
        let result = paths
            .iter()
            .map(|(p, m)| self.observe(p, *m))
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
    values: Vec<Option<Value>>,
}
#[derive(Serialize, Deserialize)]
struct Rows {
    schemas: Vec<Schema>,
    rows: Vec<PackedRow>,
}

#[derive(Serialize, Deserialize)]
pub(in super::super) struct Packet {
    pub digest: String,
    pub unknowns: Vec<(String, String)>,
    data: Vec<u8>,
}
impl Packet {
    pub(super) fn typed_bytes(&self) -> &[u8] {
        &self.data
    }
    pub(super) fn save(receipt: &Receipt<'_>) -> Self {
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
                        .map(|(_, c)| c.as_ref().map(Value::from_cell))
                        .collect(),
                }
            })
            .collect();
        Self {
            digest: super::super::digest(&receipt.rows),
            unknowns: receipt.unknowns.iter().map(|v| (*v).clone()).collect(),
            data: postcard::to_stdvec(&Rows { schemas, rows }).expect("typed request rows"),
        }
    }
    pub(in super::super) fn report(&self) -> session::Reported {
        let Rows { schemas, rows } =
            postcard::from_bytes(&self.data).expect("verified request packet");
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
                            .zip(r.values.into_iter().map(|c| c.map(|v| v.cell())))
                            .collect(),
                    }
                })
                .collect(),
        }
    }
}
