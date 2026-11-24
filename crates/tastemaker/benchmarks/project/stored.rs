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

type Key = [u8; 32];
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
fn key(value: impl Hash) -> Key {
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
                    .map(|(n, v)| {
                        (
                            n.clone(),
                            v.as_ref().map(|c| match c {
                                Cell::Text(s) => Value::Text(s.clone()),
                                Cell::File(s) => Value::File(s.clone()),
                                Cell::Number(i) => Value::Number(i.0),
                                Cell::Span(s) => Value::Span(s.start.0, s.end.0),
                            }),
                        )
                    })
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
                    .map(|(n, v)| {
                        (
                            n.clone(),
                            v.as_ref().map(|v| match v {
                                Value::Text(s) => Cell::Text(s.clone()),
                                Value::File(s) => Cell::File(s.clone()),
                                Value::Number(n) => Cell::Number(Integer(*n)),
                                Value::Span(s, e) => Cell::Span(Span {
                                    start: Offset(*s),
                                    end: Offset(*e),
                                }),
                            }),
                        )
                    })
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
    path: String,
    pairs: BTreeMap<Key, Local>,
    notes: Option<Notes>,
    blocks: Vec<EncodedBlock>,
    #[serde(skip)]
    known_blocks: HashMap<Key, usize>,
}
#[derive(Serialize, Deserialize)]
struct Closed {
    report: Report,
    // Additional observations made by a disk-authorized provider.
    checks: Vec<(PathBuf, Evidence)>,
    unread: Vec<String>,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
enum Evidence {
    Bytes(Key),
    Kind(u8),
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
fn valid(checks: &[(PathBuf, Evidence)]) -> bool {
    checks.iter().all(|(p, x)| match x {
        Evidence::Bytes(d) => fs::read(p).is_ok_and(|s| *blake3::hash(&s).as_bytes() == *d),
        Evidence::Kind(k) => kind(p).is_ok_and(|i| i == *k),
    })
}

type Files = BTreeMap<Key, FileAnswers>;
type Scopes = BTreeMap<Key, Closed>;
#[derive(Serialize, Deserialize)]
struct Snapshot {
    build: Key,
    files: Key,
    scopes: Key,
}
pub(super) struct Store {
    dir: PathBuf,
    previous: Vec<u8>,
    build: Key,
    files: Files,
    scopes: Scopes,
    dirty: [bool; 2],
    observed: HashMap<PathBuf, (u64, u64, i64, i64)>,
    versions: Vec<Option<Key>>,
    pairs: Vec<Option<PairIndex>>,
    closed: Vec<Option<Key>>,
    restored_unread: BTreeSet<String>,
    pub file_hits: usize,
    pub closed_hits: usize,
    pub evaluated_pairs: usize,
    pub evaluated_closed: usize,
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
        let loaded = postcard::from_bytes::<Snapshot>(&previous)
            .ok()
            .and_then(|s| {
                if s.build != build {
                    return None;
                }
                Some((
                    load(&table(&dir, "pairs", s.files), s.files)?,
                    load(&table(&dir, "closed", s.scopes), s.scopes)?,
                ))
            });
        let dirty = [loaded.is_none(); 2];
        let (files, scopes) = loaded.unwrap_or_default();
        Ok(Self {
            dir,
            previous,
            build,
            files,
            scopes,
            dirty,
            observed: HashMap::new(),
            versions: Vec::new(),
            pairs: Vec::new(),
            closed: Vec::new(),
            restored_unread: BTreeSet::new(),
            file_hits: 0,
            closed_hits: 0,
            evaluated_pairs: 0,
            evaluated_closed: 0,
        })
    }
    pub(super) fn publish(&mut self) -> Res<usize> {
        if !self.dirty.contains(&true) {
            return Ok(0);
        }
        let guard = fs::File::open(&self.dir)?;
        guard.lock()?;
        use std::os::unix::fs::MetadataExt;
        for (p, stamp) in &self.observed {
            if !fs::metadata(p)
                .is_ok_and(|m| (m.dev(), m.ino(), m.ctime(), m.ctime_nsec()) == *stamp)
            {
                return Ok(0);
            }
        }
        let head = self.dir.join("session.postcard");
        if fs::read(&head).unwrap_or_default() != self.previous {
            return Ok(0);
        }
        let prior = postcard::from_bytes::<Snapshot>(&self.previous).ok();
        let mut digests = [
            prior.as_ref().map_or([0; 32], |s| s.files),
            prior.as_ref().map_or([0; 32], |s| s.scopes),
        ];
        let mut written = 0;
        for (i, name) in ["pairs", "closed"].iter().enumerate() {
            if !self.dirty[i] {
                continue;
            }
            let b = if i == 0 {
                postcard::to_stdvec(&self.files)
            } else {
                postcard::to_stdvec(&self.scopes)
            }
            .map_err(io::Error::other)?;
            let d = *blake3::hash(&b).as_bytes();
            let path = table(&self.dir, name, d);
            if !fs::read(&path).is_ok_and(|old| old == b) {
                atomic(&self.dir, &path, &b)?;
                written += b.len();
            }
            digests[i] = d;
        }
        let bytes = postcard::to_stdvec(&Snapshot {
            build: self.build,
            files: digests[0],
            scopes: digests[1],
        })
        .map_err(io::Error::other)?;
        if self.previous != bytes {
            guard.sync_all()?;
            atomic(&self.dir, &head, &bytes)?;
            guard.sync_all()?;
            written += bytes.len();
        }
        if let Some(old) = prior {
            for (name, d, new) in [
                ("pairs", old.files, digests[0]),
                ("closed", old.scopes, digests[1]),
            ] {
                if d != new {
                    let _ = fs::remove_file(table(&self.dir, name, d));
                }
            }
        }
        self.previous = bytes;
        self.dirty = [false; 2];
        Ok(written)
    }

    pub(super) fn admit(
        &mut self,
        b: &mut Batch,
        world: &mut session::World,
        answers: &mut [Answer],
        contributions: &mut [Option<Rc<project::Contribution>>],
    ) {
        use std::os::unix::fs::MetadataExt;
        let mut inodes = HashMap::new();
        let prior = self.versions.len();
        self.versions
            .extend(b.schedule.sources.iter().skip(prior).map(|source| {
                let digest = if let Some(data) = &source.provided {
                    (*blake3::hash(data).as_bytes(), None)
                } else {
                    let p = Path::new(&*source.root).join(&source.path);
                    let m = fs::metadata(&p).ok()?;
                    let id = (m.dev(), m.ino(), m.ctime(), m.ctime_nsec());
                    self.observed.entry(p.clone()).or_insert(id);
                    let d = if let Some(d) = inodes.get(&id) {
                        *d
                    } else {
                        let d = *blake3::hash(&fs::read(&p).ok()?).as_bytes();
                        inodes.insert(id, d);
                        d
                    };
                    (d, Some(m.mode() & 0o111))
                };
                Some(key((&source.path, digest)))
            }));
        let mut rules = vec![[0; 32]; b.schedule.programs.len()];
        for ((text, params), &idx) in &b.schedule.prepared {
            rules[idx] = key((text, params));
        }
        let bindings: Vec<_> = b
            .bindings
            .iter()
            .map(|(r, host)| key((rules[*r], host)))
            .collect();
        self.pairs.resize(b.schedule.pairs, None);
        for (i, src) in b.schedule.sources.iter().enumerate() {
            for (&binding, &slot) in &src.bindings {
                self.pairs[slot] = self.versions[i].map(|d| (d, bindings[binding]));
            }
        }
        let project_keys: Vec<_> = b
            .projects
            .iter()
            .map(|p| {
                let versions = p
                    .slots
                    .iter()
                    .map(|&i| self.versions[i])
                    .collect::<Option<Vec<_>>>()?;
                Some(key((&p.authority, &p.paths, versions)))
            })
            .collect();
        let mut inventories = HashMap::new();
        let mut needed = vec![false; self.pairs.len()];
        let mut frontier = vec![false; self.pairs.len()];
        let mut projects = BTreeSet::new();
        self.closed = vec![None; b.work.len()];
        self.restored_unread.clear();
        for (i, work) in b.work.iter_mut().enumerate() {
            match work {
                Work::Pairs(slots) => {
                    for &slot in slots.iter() {
                        needed[slot] = true;
                    }
                }
                Work::Closed {
                    project,
                    pairs,
                    binding,
                    inventory,
                    files,
                } => {
                    let inv = *inventories
                        .entry(Rc::as_ptr(inventory))
                        .or_insert_with(|| key(inventory));
                    let prereqs = pairs
                        .iter()
                        .map(|&slot| self.pairs[slot])
                        .collect::<Option<Vec<_>>>();
                    let pk = match project {
                        Some(id) => project_keys[*id],
                        None => Some([0; 32]),
                    };
                    let bound = prereqs
                        .zip(pk)
                        .map(|(ps, p)| key((bindings[*binding], files.as_slice(), inv, p, ps)));
                    self.closed[i] = bound;
                    if let Some(row) = bound
                        .and_then(|k| self.scopes.get(&k))
                        .filter(|r| valid(&r.checks))
                    {
                        self.restored_unread.extend(row.unread.iter().cloned());
                        *work = Work::Project(Rc::new(Ok(row.report.read())));
                        self.closed_hits += 1;
                    } else {
                        self.evaluated_closed += 1;
                        for &slot in pairs.iter() {
                            needed[slot] = true;
                            frontier[slot] = true;
                        }
                        if let Some(p) = project {
                            projects.extend(b.projects[*p].slots.iter().copied());
                        }
                    }
                }
                _ => {}
            }
        }
        let mut restored = HashMap::new();
        // Later live requests can consume a file answer delivered in an earlier epoch.
        // Its source no longer has a pending binding, but its stable pair key remains.
        for (slot, requested) in frontier.into_iter().enumerate() {
            if !requested {
                continue;
            }
            if world.frontiers.contains_key(&slot) {
                continue;
            }
            let Some((version, binding)) = self.pairs[slot] else {
                continue;
            };
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
            world.frontiers.insert(slot, parts);
            if !pair.unavailable.is_empty() {
                let missing = pair
                    .unavailable
                    .iter()
                    .map(|(name, path)| (name.clone(), world.file(path)))
                    .collect();
                world.unavailable.insert(slot, missing);
            }
        }
        for (i, src) in b.schedule.sources.iter_mut().enumerate() {
            if !projects.contains(&i) {
                src.project = false;
            }
            let held = self.versions[i].and_then(|v| self.files.get(&v));
            if let Some(notes) = held.and_then(|f| f.notes.as_ref()).filter(|_| src.project) {
                contributions[i] = Some(Rc::new(project::Contribution {
                    reader: notes.reader,
                    notes: l::Bytes(notes.notes.clone()),
                    manifest: notes.manifest.clone().map(l::ManifestText),
                }));
                src.project = false;
            }
            src.bindings.retain(|_, slot| {
                if !needed[*slot] {
                    return false;
                }
                let Some((_, bind)) = self.pairs[*slot] else {
                    return true;
                };
                let Some(file) = held else { return true };
                let Some(pair) = file.pairs.get(&bind) else {
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
            let file = self.files.entry(version).or_insert_with(|| FileAnswers {
                path: src.path.clone(),
                ..Default::default()
            });
            if let Some(notes) = &contributions[i]
                && file.notes.is_none()
            {
                file.notes = Some(Notes {
                    reader: notes.reader,
                    notes: notes.notes.0.clone(),
                    manifest: notes.manifest.as_ref().map(|m| m.0.clone()),
                });
                self.dirty[0] = true;
            }
            for &slot in src.bindings.values() {
                let Some((_, binding)) = self.pairs[slot] else {
                    continue;
                };
                let a = aliases[slot];
                let Ok(report) = &answers[a] else { continue };
                if file.pairs.contains_key(&binding) {
                    continue;
                }
                let mut frontier = Vec::new();
                if let Some(parts) = world.frontiers.get(&a) {
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
                self.dirty[0] = true;
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
        let Some(key) = self.closed[idx] else { return };
        let Ok(report) = value else { return };
        let mut checks = Vec::new();
        let mut unread = Vec::new();
        if let Some(p) = pop {
            unread.extend(p.refused.iter().map(|r| r.path.0.to_string()));
            if let Some(Authority::Disk(root)) = authority {
                // Record the state seen by the provider, including absent paths.
                checks.push((PathBuf::from(root), Evidence::Kind(2)));
                for entry in &p.observations {
                    use l::ObservedEntryState::*;
                    let k = match entry.state {
                        Missing => 0,
                        File => 1,
                        Directory => 2,
                        Other => 3,
                        Unknown => return,
                    };
                    checks.push((Path::new(root).join(&*entry.path.0), Evidence::Kind(k)));
                }
                for name in &p.consulted {
                    let path = Path::new(root).join(&*name.0);
                    let evidence = match fs::read(&path) {
                        Ok(bytes) => Evidence::Bytes(*blake3::hash(&bytes).as_bytes()),
                        Err(e) if e.kind() == io::ErrorKind::NotFound => Evidence::Kind(0),
                        _ => return,
                    };
                    checks.push((path, evidence));
                }
            }
        }
        self.scopes.insert(
            key,
            Closed {
                report: Report::save(report),
                checks,
                unread,
            },
        );
        self.dirty[1] = true;
    }
    pub(super) fn prune(&mut self) {
        let mut needed = HashMap::<Key, BTreeSet<Key>>::new();
        for &version in self.versions.iter().flatten() {
            needed.entry(version).or_default();
        }
        for &(version, binding) in self.pairs.iter().flatten() {
            needed.entry(version).or_default().insert(binding);
        }
        self.files.retain(|k, f| {
            let Some(bindings) = needed.get(k) else {
                self.dirty[0] = true;
                return false;
            };
            f.pairs.retain(|b, _| {
                let keep = bindings.contains(b);
                self.dirty[0] |= !keep;
                keep
            });
            true
        });
        let scopes: BTreeSet<_> = self.closed.iter().flatten().copied().collect();
        self.scopes.retain(|k, _| {
            let keep = scopes.contains(k);
            self.dirty[1] |= !keep;
            keep
        });
    }
    pub(super) fn unread(&self) -> impl Iterator<Item = l::FilePath> + '_ {
        self.restored_unread
            .iter()
            .map(|p| l::FilePath(p.clone().into()))
    }
}
