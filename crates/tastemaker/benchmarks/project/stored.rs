//! Durable values at the typed session boundary. Run-local symbol and file numbers
//! are translated before persistence. A single atomic manifest selects both tables.
use super::{
    Answer, Authority, Batch, Path, PathBuf, Population, Rc, Work, finish, l, project, session,
};
use ryo_relations::domain::relations::types::{Integer, Offset, Span};
use ryo_values::domain::row_texts::types::Cell;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    hash::{Hash, Hasher},
    io::{self, Write},
};

pub(super) type Key = [u8; 32];
type PairIndex = (Key, Key);
type Res<T> = io::Result<T>;
#[derive(Default)]
struct Digest(blake3::Hasher);
impl Hasher for Digest {
    fn write(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }
    fn finish(&self) -> u64 {
        u64::from_le_bytes(self.0.finalize().as_bytes()[..8].try_into().unwrap())
    }
}
pub(super) fn key(value: impl Hash) -> Key {
    let mut h = Digest::default();
    value.hash(&mut h);
    *h.0.finalize().as_bytes()
}

#[derive(Serialize, Deserialize)]
enum Value {
    File(Box<str>),
    Text(Box<str>),
    Number(i64),
    Span(u32, u32),
}

impl Value {
    fn from_cell(c: &Cell) -> Self {
        match c {
            Cell::File(s) => Self::File(s.clone()),
            Cell::Text(s) => Self::Text(s.clone()),
            Cell::Number(n) => Self::Number(n.0),
            Cell::Span(s) => Self::Span(s.start.0, s.end.0),
        }
    }
    fn cell(&self) -> Cell {
        match self {
            Self::File(s) => Cell::File(s.clone()),
            Self::Text(s) => Cell::Text(s.clone()),
            Self::Number(n) => Cell::Number(Integer(*n)),
            Self::Span(a, b) => Cell::Span(Span {
                start: Offset(*a),
                end: Offset(*b),
            }),
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Row {
    rule: Box<str>,
    columns: Vec<(Box<str>, Option<Value>)>,
    identity: Vec<usize>,
}
#[derive(Serialize, Deserialize)]
struct Report {
    rows: Vec<Row>,
    unknowns: Vec<(String, String)>,
}
impl Report {
    fn save(answer: &session::Reported) -> Self {
        let rows = answer
            .rows
            .iter()
            .map(|r| Row {
                rule: r.rule.clone(),
                identity: r.identity.clone(),
                columns: r
                    .columns
                    .iter()
                    .map(|(n, v)| (n.clone(), v.as_ref().map(Value::from_cell)))
                    .collect(),
            })
            .collect();
        Self {
            rows,
            unknowns: answer.unknowns.clone(),
        }
    }
    fn read(&self) -> session::Reported {
        let rows = self
            .rows
            .iter()
            .map(|r| finish::answer::Row {
                rule: r.rule.clone(),
                identity: r.identity.clone(),
                columns: r
                    .columns
                    .iter()
                    .map(|(n, v)| (n.clone(), v.as_ref().map(Value::cell)))
                    .collect(),
            })
            .collect();
        session::Reported {
            rows,
            unknowns: self.unknowns.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Notes {
    reader: Option<usize>,
    notes: Vec<u8>,
    manifest: Option<Box<str>>,
}
#[derive(Serialize, Deserialize)]
struct Local {
    report: Report,
    frontier: Vec<(usize, usize)>,
    unavailable: Vec<(String, String)>,
}
// This length-prefixed postcard payload stays packed until a changed closed
// answer actually requests a column from it.
struct EncodedBlock(Vec<u8>);
impl<'de> Deserialize<'de> for EncodedBlock {
    fn deserialize<D: serde::Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        Ok(Self(<&[u8]>::deserialize(input)?.to_vec()))
    }
}
impl Serialize for EncodedBlock {
    fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        out.serialize_bytes(&self.0)
    }
}
impl EncodedBlock {
    fn restore(&self, world: &mut session::World) -> Rc<finish::population::Block> {
        let block: session::stored_block::StoredBlock =
            postcard::from_bytes(&self.0).expect("block in checksum-verified table for this build");
        block.restore(world)
    }
}
#[derive(Default, Serialize, Deserialize)]
struct FileAnswers {
    pairs: BTreeMap<Key, Local>,
    blocks: Vec<EncodedBlock>,
    #[serde(skip)]
    known_blocks: HashMap<Key, usize>,
}
fn kind(p: &Path) -> Res<u8> {
    match fs::metadata(p) {
        Ok(m) => Ok(if m.is_file() {
            1
        } else if m.is_dir() {
            2
        } else {
            3
        }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(e),
    }
}
#[path = "request_answers.rs"]
mod request_answers;
#[path = "shards.rs"]
mod shards;
pub(super) use request_answers::Packet;
use request_answers::{Group, Observations, Probe, Proof, Record};
use shards::{Index, Records, Segments, Shards};

#[derive(Default, Serialize, Deserialize)]
struct Snapshot {
    names: Option<Key>,
    build: Key,
    files: Segments,
    notes: Index,
    requests: Index,
    groups: Index,
    packets: Index,
}
pub(super) struct Store {
    dir: PathBuf,
    names: Vec<PathBuf>,
    name_ids: HashMap<PathBuf, u32>,
    names_digest: Option<Key>,
    saved_names: usize,
    previous: Vec<u8>,
    build: Key,
    files: Records<FileAnswers>,
    notes: Shards<Notes>,
    requests: Shards<Record>,
    groups: Shards<Group>,
    packets: Shards<Packet>,
    observed: Observations,
    versions: Vec<Option<Key>>,
    pairs: Vec<Option<PairIndex>>,
    sources: Vec<Option<usize>>,
    plans: Vec<Option<(Vec<Proof>, Vec<String>)>>,
    keep_pairs: Vec<bool>,
    pub file_hits: usize,
    pub closed_hits: usize,
    pub evaluated_pairs: usize,
    pub evaluated_closed: usize,
    pub request_hits: usize,
    absent_file: usize,
    absent_local: usize,
    absent_frontier: usize,
    restored_blocks: usize,
}
fn table(dir: &Path, name: &str, digest: Key) -> PathBuf {
    dir.join(format!(
        "{name}-{}.postcard",
        blake3::Hash::from_bytes(digest).to_hex()
    ))
}
fn load<T: DeserializeOwned>(p: &Path, digest: Key) -> Option<T> {
    let b = fs::read(p).ok()?;
    (*blake3::hash(&b).as_bytes() == digest)
        .then(|| postcard::from_bytes(&b).ok())
        .flatten()
}
fn atomic(dir: &Path, p: &Path, b: &[u8]) -> Res<()> {
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(b)?;
    tmp.as_file().sync_all()?;
    tmp.persist(p)?;
    Ok(())
}
impl Store {
    pub(super) fn open(root: &Path) -> Res<Self> {
        Self::directory(root.join(".ryo_cache"))
    }
    pub(super) fn directory(dir: PathBuf) -> Res<Self> {
        use object::Object;
        let executable = std::env::current_exe()?;
        let image = object::read::ReadCache::new(fs::File::open(&executable)?);
        let build = if let Ok(file) = object::File::parse(&image)
            && let Ok(Some(id)) = file.build_id()
        {
            *blake3::hash(id).as_bytes()
        } else {
            *blake3::hash(&fs::read(executable)?).as_bytes()
        };

        fs::create_dir_all(&dir)?;
        let guard = fs::File::open(&dir)?;
        guard.lock_shared()?;
        let previous = fs::read(dir.join("session.postcard")).unwrap_or_default();
        let snapshot = postcard::from_bytes::<Snapshot>(&previous)
            .ok()
            .filter(|s| s.build == build)
            .unwrap_or_default();
        let (snapshot, names) = match snapshot.names {
            Some(k) => match load::<Vec<PathBuf>>(&table(&dir, "paths", k), k) {
                Some(n) => (snapshot, n),
                None => (Snapshot::default(), vec![]),
            },
            None => (snapshot, vec![]),
        };
        Ok(Self {
            names_digest: snapshot.names,
            saved_names: names.len(),
            names,
            name_ids: HashMap::new(),
            dir,
            previous,
            build,
            files: Records::new(snapshot.files),
            notes: Shards::new("project", snapshot.notes),
            requests: Shards::new("requests", snapshot.requests),
            groups: Shards::new("inputs", snapshot.groups),
            packets: Shards::new("answers", snapshot.packets),
            observed: Default::default(),
            versions: vec![],
            pairs: vec![],
            sources: vec![],
            plans: vec![],
            keep_pairs: vec![],
            file_hits: 0,
            closed_hits: 0,
            evaluated_pairs: 0,
            evaluated_closed: 0,
            request_hits: 0,
            absent_file: 0,
            absent_local: 0,
            absent_frontier: 0,
            restored_blocks: 0,
        })
    }
    pub(super) fn publish(&mut self) -> Res<usize> {
        if !(self.files.dirty()
            || self.notes.dirty()
            || self.requests.dirty()
            || self.groups.dirty()
            || self.packets.dirty())
        {
            return Ok(0);
        }
        use std::os::unix::fs::MetadataExt;
        let guard = fs::File::open(&self.dir)?;
        guard.lock()?;
        let head = self.dir.join("session.postcard");
        if fs::read(&head).unwrap_or_default() != self.previous {
            return Ok(0);
        }
        if self.observed.stamps.iter().any(|(p, id)| {
            !fs::metadata(p).is_ok_and(|m| (m.dev(), m.ino(), m.ctime(), m.ctime_nsec()) == *id)
        }) {
            return Ok(0);
        }
        let mut old = vec![];
        let mut written = self.files.flush(&self.dir, &mut old)?
            + self.notes.flush(&self.dir, &mut old)?
            + self.requests.flush(&self.dir, &mut old)?
            + self.groups.flush(&self.dir, &mut old)?
            + self.packets.flush(&self.dir, &mut old)?;
        if self.names.len() != self.saved_names {
            let b = postcard::to_stdvec(&self.names).map_err(io::Error::other)?;
            let k = *blake3::hash(&b).as_bytes();
            atomic(&self.dir, &table(&self.dir, "paths", k), &b)?;
            written += b.len();
            if let Some(old_key) = self.names_digest.replace(k)
                && old_key != k
            {
                old.push(table(&self.dir, "paths", old_key));
            }
            self.saved_names = self.names.len();
        }
        let bytes = postcard::to_stdvec(&Snapshot {
            names: self.names_digest,
            build: self.build,
            files: self.files.index.clone(),
            notes: self.notes.index.clone(),
            requests: self.requests.index.clone(),
            groups: self.groups.index.clone(),
            packets: self.packets.index.clone(),
        })
        .map_err(io::Error::other)?;
        if bytes != self.previous {
            guard.sync_all()?;
            atomic(&self.dir, &head, &bytes)?;
            guard.sync_all()?;
            written += bytes.len();
            self.previous = bytes;
        }
        for p in old {
            let _ = fs::remove_file(p);
        }
        Ok(written)
    }
    pub(super) fn costs(&self) {
        eprintln!(
            "contributions restored_blocks={} missing_file_slots={} missing_local_bindings={} missing_frontier_bindings={}",
            self.restored_blocks, self.absent_file, self.absent_local, self.absent_frontier
        );
        eprintln!(
            "request_store hash_cpu={:.6} hash_bytes={} pairs_read={} project_read={} requests_read={} inputs_read={} answers_read={} pairs_written={} project_written={} requests_written={} inputs_written={} answers_written={}",
            self.observed.hash_cpu,
            self.observed.hash_bytes,
            self.files.read_bytes,
            self.notes.read_bytes,
            self.requests.read_bytes,
            self.groups.read_bytes,
            self.packets.read_bytes,
            self.files.write_bytes,
            self.notes.write_bytes,
            self.requests.write_bytes,
            self.groups.write_bytes,
            self.packets.write_bytes
        );
    }
    pub(super) fn overlay(
        &mut self,
        root: &Path,
        inventory: &[String],
        peers: Key,
        v: &BTreeMap<String, std::sync::Arc<[u8]>>,
    ) {
        self.observed.overlay(root, inventory, peers, v);
    }
    pub(super) fn inventory(&mut self, i: usize, path: PathBuf, expected: Option<Key>) {
        let evidence = self.group(vec![(path, Probe::Bytes)]);
        if expected.is_some()
            && let Some(g) = evidence
            && let Some((proof, _)) = &mut self.plans[i]
        {
            proof.push(g);
        } else {
            self.plans[i] = None;
        }
    }
    pub(super) fn contract_file(&mut self, p: &Path) -> Option<Key> {
        self.observed.observe(p, Probe::Bytes)
    }
    pub(super) fn lookup(&mut self, id: Key) -> Option<(Key, Vec<String>)> {
        self.requests.open(&self.dir, &id);
        let r = self.requests.get(&id)?;
        for &(g, expected) in &r.proof {
            self.groups.open(&self.dir, &g);
            if self.observed.group(g, self.groups.get(&g)?, &self.names) != Some(expected) {
                return None;
            }
        }
        self.packets.open(&self.dir, &r.packet);
        self.packets.get(&r.packet)?;
        self.request_hits += 1;
        Some((r.packet, r.unread.clone()))
    }
    pub(super) fn packet(&self, id: &Key) -> &Packet {
        self.packets.get(id).expect("verified packet")
    }
    pub(super) fn save(&mut self, id: Key, idx: usize, r: &super::Receipt<'_>) -> Option<Key> {
        let (proof, unread) = self.plans.get_mut(idx)?.take()?;
        Some(self.put(id, Packet::save(r), proof, unread))
    }
    pub(super) fn decline(&mut self, id: Key, reason: &str) {
        self.put(id, Packet::declined(reason), vec![], vec![]);
    }
    fn put(&mut self, id: Key, packet: Packet, proof: Vec<Proof>, unread: Vec<String>) -> Key {
        let k = key((
            &packet.refusal,
            &packet.digest,
            &packet.unknowns,
            packet.typed_bytes(),
        ));
        self.packets.open(&self.dir, &k);
        if self.packets.get(&k).is_none() {
            self.packets.insert(k, packet);
        }
        self.requests.open(&self.dir, &id);
        self.requests.insert(
            id,
            Record {
                packet: k,
                proof,
                unread,
            },
        );
        k
    }
    fn group(&mut self, mut paths: Vec<(PathBuf, Probe)>) -> Option<Proof> {
        if self.name_ids.is_empty() && !self.names.is_empty() {
            self.name_ids.extend(
                self.names
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (p.clone(), i as u32)),
            );
        }
        paths.sort_unstable();
        paths.dedup();
        let g: Group = paths
            .into_iter()
            .map(|(p, mode)| {
                let i = *self.name_ids.entry(p.clone()).or_insert_with(|| {
                    let i = u32::try_from(self.names.len())
                        .expect("source inventory fits in a process");
                    self.names.push(p);
                    i
                });
                (i, mode)
            })
            .collect();
        let id = key(&g);
        self.groups.open(&self.dir, &id);
        let expected = self.observed.group(id, &g, &self.names)?;
        if self.groups.get(&id).is_none() {
            self.groups.insert(id, g);
        }
        Some((id, expected))
    }
    pub(super) fn admit(
        &mut self,
        b: &mut Batch,
        world: &mut session::World,
        answers: &mut [Answer],
        contributions: &mut [Option<Rc<project::Contribution>>],
    ) {
        let prior = self.versions.len();
        self.versions
            .extend(b.schedule.sources.iter().skip(prior).map(|source| {
                let d = if let Some(v) = &source.provided {
                    *blake3::hash(v).as_bytes()
                } else {
                    self.observed
                        .observe(&Path::new(&*source.root).join(&source.path), Probe::Source)?
                };
                Some(key((&source.path, d)))
            }));
        let mut rules = vec![[0; 32]; b.schedule.programs.len()];
        for ((text, params), &idx) in &b.schedule.prepared {
            rules[idx] = key((text, params));
        }
        let bindings: Vec<_> = b
            .bindings
            .iter()
            .enumerate()
            .map(|(binding, host)| {
                let r = b.program_of[binding];
                // The engine shares this pre-join work for every binding with the
                // same declared host shape. Which caller first supplied the rows
                // can change when all but one request is restored.
                if b.cut_of[binding].is_some() {
                    let shape = host
                        .iter()
                        .map(|h| (&h.name, &h.columns))
                        .collect::<Vec<_>>();
                    key((0u8, rules[r], shape))
                } else {
                    key((1u8, rules[r], host))
                }
            })
            .collect();
        self.pairs.resize(b.schedule.pairs, None);
        self.sources.resize(b.schedule.pairs, None);
        for (i, src) in b.schedule.sources.iter().enumerate() {
            for (&binding, &slot) in &src.bindings {
                self.sources[slot] = Some(i);
                self.pairs[slot] = self.versions[i].map(|d| (d, bindings[binding]));
            }
        }
        let mut needed = vec![false; self.pairs.len()];
        let mut frontier = vec![false; self.pairs.len()];
        self.keep_pairs = vec![false; self.pairs.len()];
        let mut projects = BTreeSet::new();
        let mut groups = HashMap::<Vec<usize>, Option<Proof>>::new();
        self.plans.clear();
        for work in &b.work {
            let (pairs, project, closed) = match work {
                Work::Pairs(p) => (p.as_slice(), None, false),
                Work::Closed { pairs, project, .. } => (pairs.as_slice(), *project, true),
                Work::Project(_) => ([].as_slice(), None, false),
                Work::Refused(_) | Work::Invalid(_) => {
                    self.plans.push(None);
                    continue;
                }
            };
            if closed {
                self.evaluated_closed += 1;
            }
            for &slot in pairs {
                needed[slot] = true;
                frontier[slot] |= closed;
                self.keep_pairs[slot] |= closed || pairs.len() > 1;
            }
            let local = pairs
                .iter()
                .map(|&slot| self.sources[slot])
                .collect::<Option<Vec<_>>>();
            let mut proof = Some(Vec::new());
            if let Some(local) = local {
                for ids in [
                    Some(local.as_slice()),
                    project.map(|i| b.projects[i].slots.as_slice()),
                ]
                .into_iter()
                .flatten()
                {
                    if ids.is_empty() {
                        continue;
                    }
                    let g = *groups.entry(ids.to_vec()).or_insert_with(|| {
                        let mut paths = Vec::new();
                        for &i in ids {
                            self.versions[i]?;
                            let src = &b.schedule.sources[i];
                            if self.observed.root.is_some() {
                                paths.push((src.path.as_str().into(), Probe::Effective));
                            } else if src.provided.is_none() {
                                paths.push((Path::new(&*src.root).join(&src.path), Probe::Source));
                            }
                        }
                        self.group(paths)
                    });
                    if let (Some(ps), Some(g)) = (&mut proof, g) {
                        ps.push(g)
                    } else {
                        proof = None;
                        break;
                    }
                }
            } else {
                proof = None
            }
            if let Some(i) = project {
                projects.extend(b.projects[i].slots.iter().copied());
            }
            if self.observed.root.is_some()
                && (closed || matches!(work, Work::Project(_)))
                && let Some(ps) = &mut proof
            {
                ps.push(
                    self.group(vec![(PathBuf::new(), Probe::Inventory)])
                        .expect("declared inventory"),
                );
            }
            self.plans.push(proof.map(|p| (p, Vec::new())));
        }
        for (i, src) in b.schedule.sources.iter().enumerate() {
            if let Some(v) = self.versions[i] {
                if projects.contains(&i) {
                    self.notes.open(&self.dir, &v);
                }
                if src.bindings.values().any(|&slot| self.keep_pairs[slot]) {
                    self.files.open(&self.dir, &v);
                }
            }
        }
        let mut restored = HashMap::new();
        // Later live requests can consume a file answer delivered in an earlier epoch.
        // Its source no longer has a pending binding, but its stable pair key remains.
        for (slot, requested) in frontier.into_iter().enumerate() {
            if !requested {
                continue;
            }
            if world.frontier_parts(slot).is_some() {
                continue;
            }
            let Some((version, binding)) = self.pairs[slot] else {
                continue;
            };
            self.files.open(&self.dir, &version);
            let Some(file) = self.files.get(&version) else {
                continue;
            };
            let Some(pair) = file.pairs.get(&binding) else {
                continue;
            };
            let parts = pair
                .frontier
                .iter()
                .map(|&(n, id)| {
                    let block = restored
                        .entry((version, id))
                        .or_insert_with(|| file.blocks[id].restore(world))
                        .clone();
                    (n, block)
                })
                .collect();
            world.restore_frontier(slot, parts);
            if !pair.unavailable.is_empty() {
                let missing = pair
                    .unavailable
                    .iter()
                    .map(|(name, path)| (name.clone(), world.file(path)))
                    .collect();
                world.unavailable.insert(slot, missing);
            }
        }
        self.restored_blocks += restored.len();
        for (i, src) in b.schedule.sources.iter_mut().enumerate() {
            if !projects.contains(&i) {
                src.project = false;
            }
            let held = self.versions[i].and_then(|v| self.files.get(&v));
            if let Some(notes) = self.versions[i]
                .and_then(|v| self.notes.get(&v))
                .filter(|_| src.project)
            {
                contributions[i] = Some(Rc::new(project::Contribution {
                    reader: notes.reader,
                    notes: l::Bytes(notes.notes.clone()),
                    manifest: notes.manifest.clone().map(l::ManifestText),
                }));
                src.project = false;
            }
            src.bindings.retain(|&binding, slot| {
                if !needed[*slot] {
                    return false;
                }
                let Some((_, bind)) = self.pairs[*slot] else {
                    return true;
                };
                let Some(file) = held else {
                    self.absent_file += 1;
                    return true;
                };
                let Some(pair) = file.pairs.get(&bind) else {
                    if b.cut_of[binding].is_some() {
                        self.absent_frontier += 1
                    } else {
                        self.absent_local += 1
                    }
                    return true;
                };
                answers[*slot] = Ok(pair.report.read());
                self.file_hits += 1;
                false
            });
        }
        b.schedule.pending.retain(|i| {
            let src = &b.schedule.sources[*i];
            self.evaluated_pairs += src.bindings.len();
            src.project || !src.bindings.is_empty()
        });
    }

    pub(super) fn local(
        &mut self,
        b: &Batch,
        world: &session::World,
        answers: &[Answer],
        aliases: &[usize],
        contributions: &[Option<Rc<project::Contribution>>],
    ) {
        let mut encoded = HashMap::new();
        for (i, src) in b.schedule.sources.iter().enumerate() {
            let Some(version) = self.versions[i] else {
                continue;
            };
            if src.bindings.is_empty() && !src.project {
                continue;
            }
            if src.project
                && let Some(notes) = &contributions[i]
                && self.notes.get(&version).is_none()
            {
                self.notes.insert(
                    version,
                    Notes {
                        reader: notes.reader,
                        notes: notes.notes.0.clone(),
                        manifest: notes.manifest.as_ref().map(|m| m.0.clone()),
                    },
                );
            }
            if !src.bindings.values().any(|&s| self.keep_pairs[s]) {
                continue;
            }
            self.files.open(&self.dir, &version);
            let file = self.files.change(&version);
            for &slot in src.bindings.values() {
                if !self.keep_pairs[slot] {
                    continue;
                }
                let Some((_, binding)) = self.pairs[slot] else {
                    continue;
                };
                let a = aliases[slot];
                let Ok(report) = &answers[a] else { continue };
                if file.pairs.contains_key(&binding) {
                    continue;
                }
                let mut frontier = Vec::new();
                if let Some(parts) = world.frontier_parts(a) {
                    for (node, block) in parts {
                        let addr = (version, Rc::as_ptr(block));
                        let index = *encoded.entry(addr).or_insert_with(|| {
                            let stored = session::stored_block::StoredBlock::save(block, world);
                            let bytes = postcard::to_stdvec(&stored).expect("typed block");
                            let hash = *blake3::hash(&bytes).as_bytes();
                            *file.known_blocks.entry(hash).or_insert_with(|| {
                                file.blocks.push(EncodedBlock(bytes));
                                file.blocks.len() - 1
                            })
                        });
                        frontier.push((*node, index));
                    }
                }
                let unavailable = world
                    .unavailable
                    .get(&a)
                    .into_iter()
                    .flatten()
                    .map(|(n, f)| (n.clone(), world.run.paths[*f as usize].clone()))
                    .collect();
                file.pairs.insert(
                    binding,
                    Local {
                        report: Report::save(report),
                        frontier,
                        unavailable,
                    },
                );
            }
        }
    }

    pub(super) fn closed(
        &mut self,
        idx: usize,
        value: &Answer,
        pop: Option<&Population>,
        authority: Option<&Authority>,
    ) {
        if value.is_err() {
            self.plans[idx] = None;
            return;
        }
        let Some(p) = pop else { return };
        let unread = p.refused.iter().map(|r| r.path.0.to_string()).collect();
        let Some(Authority::Disk(root)) = authority else {
            if let Some((_, u)) = &mut self.plans[idx] {
                *u = unread;
            }
            return;
        };
        let mut paths = vec![(PathBuf::from(root), Probe::Kind)];
        for entry in &p.observations {
            use l::ObservedEntryState::*;
            let k = match entry.state {
                Missing => 0,
                File => 1,
                Directory => 2,
                Other => 3,
                Unknown => {
                    self.plans[idx] = None;
                    return;
                }
            };
            let path = Path::new(root).join(&*entry.path.0);
            if self.observed.observe(&path, Probe::Kind) != Some(key(k as u8)) {
                self.plans[idx] = None;
                return;
            }
            paths.push((path, Probe::Kind));
        }
        for name in &p.consulted {
            let path = Path::new(root).join(&*name.0);
            let probe = if kind(&path).ok() == Some(0) {
                Probe::Kind
            } else {
                Probe::Bytes
            };
            paths.push((path, probe));
        }
        if let Some(g) = self.group(paths)
            && let Some((proof, u)) = &mut self.plans[idx]
        {
            proof.push(g);
            *u = unread
        } else {
            self.plans[idx] = None
        }
    }
}
