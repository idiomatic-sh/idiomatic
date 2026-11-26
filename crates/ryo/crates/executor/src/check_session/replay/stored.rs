// Durable values at the typed session boundary. Run-local symbol and file numbers
// are translated before persistence. A single atomic manifest selects both tables.
use super::{
    Answer, Authority, Batch, Path, PathBuf, Population, Rc, Work, finish, l, project, session,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs,
    hash::{Hash, Hasher},
    io::{self, Write},
    sync::Arc,
};

pub type Key = [u8; 32];
pub type Res<T> = io::Result<T>;
/// Collects what a value hashes to and digests it at once: blake3 hashes a short input in one
/// block, where hashing each field of the value as it is written costs several.
#[derive(Default)]
pub struct Digest(Vec<u8>);
impl Digest {
    pub fn digest(&self) -> Key {
        *blake3::hash(&self.0).as_bytes()
    }
}
impl Hasher for Digest {
    fn write(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }
    fn finish(&self) -> u64 {
        u64::from_le_bytes(self.digest()[..8].try_into().unwrap())
    }
}
pub fn key(value: impl Hash) -> Key {
    let mut h = Digest::default();
    value.hash(&mut h);
    h.digest()
}

impl session::Source {
    /// Attach the source's stable contextual inputs to its effective byte and
    /// permission key. A file record also needs the provider's observed-entry
    /// proof before its rows can be restored. Reader settings scope the store.
    pub fn version_key(&self, effective: Key) -> Option<Key> {
        match self.environment.as_deref() {
            Some(environment) => Some(key((environment.version_scope?, effective))),
            None => Some(effective),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Report {
    // None means only the closed operator consumed this contribution.
    // Some(empty) proves a local request completed with no rows.
    pub rows: Option<Vec<u8>>,
    pub issues: Option<Box<ReportIssues>>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportIssues {
    pub rejected_syntax: Vec<String>,
    pub unknowns: Vec<(String, String)>,
}

impl Report {
    pub fn save(answer: &session::Reported, keep_rows: bool) -> Self {
        Self {
            rows: keep_rows.then(|| {
                if answer.rows.is_empty() {
                    Vec::new()
                } else {
                    request_answers::pack_rows(&answer.rows)
                }
            }),
            issues: (!answer.rejected_syntax.is_empty() || !answer.unknowns.is_empty())
                .then(|| Box::new(ReportIssues {
                    rejected_syntax: answer.rejected_syntax.clone(),
                    unknowns: answer.unknowns.clone(),
                })),
        }
    }
    pub fn read(&self, need_rows: bool) -> Option<session::Reported> {
        let rows = if need_rows {
            let data = self.rows.as_ref()?;
            if data.is_empty() {
                vec![]
            } else {
                request_answers::unpack_rows(data)
            }
        } else {
            vec![]
        };
        let (unknowns, rejected_syntax) = self.issues.as_ref()
            .map(|issues| (issues.unknowns.clone(), issues.rejected_syntax.clone()))
            .unwrap_or_default();
        Some(session::Reported::new(rows, unknowns, rejected_syntax))
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct Notes {
    pub context: Option<Proof>,
    pub reader: Option<usize>,
    pub notes: Vec<u8>,
    pub manifest: Option<Box<str>>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Local {
    pub report: Report,
    pub frontier: Vec<(usize, usize)>,
    pub unavailable: Vec<(String, String)>,
}
// A mapped segment backs retained columns. Only new columns own an allocation.
#[derive(Default)]
pub struct EncodedBlock(Option<finish::population::BlockBytes>);
impl Serialize for EncodedBlock {
    fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        out.serialize_bytes(self.0.as_ref().expect("complete block").as_ref().as_ref())
    }
}
impl EncodedBlock {
    pub fn mapped(record: shards::MappedRecord) -> Option<Self> {
        Some(Self(Some(Arc::new(record.postcard_bytes()?))))
    }

    pub fn restore(&self, world: &mut session::World) -> Option<Rc<finish::population::Block>> {
        session::stored_block::restore(self.0.clone()?, world)
    }

    pub fn save(data: Vec<u8>) -> Self {
        Self(Some(Arc::new(data)))
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockId {
    Empty(u32),
    Local(Key),
}
#[derive(Deserialize)]
pub struct FileWire<T> {
    pub context: Option<Proof>,
    pub pairs: BTreeMap<Key, T>,
    pub blocks: Vec<BlockId>,
    pub complete: Vec<u8>,
    pub closed: Vec<u8>,
}

/// A pair's stable identity and its number in the store's binding table. A file record
/// names its trivially empty pairs by number only, so an unchanged file costs a few bytes
/// however many programs were checked against it.
#[derive(Clone, Copy)]
pub struct Binding {
    pub key: Key,
    pub id: u32,
}

pub fn has_bit(bits: &[u8], id: u32) -> bool {
    bits.get(id as usize / 8).is_some_and(|byte| byte & (1 << (id % 8)) != 0)
}

pub fn put_bit(bits: &mut Vec<u8>, id: u32, on: bool) {
    let at = id as usize / 8;
    if on {
        if bits.len() <= at {
            bits.resize(at + 1, 0);
        }
        bits[at] |= 1 << (id % 8);
    } else if let Some(byte) = bits.get_mut(at) {
        *byte &= !(1 << (id % 8));
    }
}

pub struct RowBytes<T>(T);
impl<T: AsRef<[u8]>> Serialize for RowBytes<T> {
    fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        out.serialize_bytes(self.0.as_ref())
    }
}

#[derive(Default)]
pub struct FileAnswers {
    pub context: Option<Proof>,
    pub pairs: BTreeMap<Key, Local>,
    pub locations: BTreeMap<Key, std::ops::Range<usize>>,
    pub mapped: Option<shards::MappedRecord>,
    pub blocks: Vec<BlockId>,
    pub known_blocks: HashMap<BlockId, usize>,
    // Pairs that completed with nothing to report: with rows kept, or only consumed by a closed operator.
    pub complete: Vec<u8>,
    pub closed: Vec<u8>,
}

impl Local {
    pub fn empty(rows: bool) -> Self {
        Self {
            report: Report { rows: rows.then(Vec::new), issues: None },
            frontier: Vec::new(),
            unavailable: Vec::new(),
        }
    }

    pub fn trivial(&self) -> Option<bool> {
        (self.frontier.is_empty() && self.unavailable.is_empty() && self.report.issues.is_none())
            .then(|| match &self.report.rows {
                None => Some(false),
                Some(rows) if rows.is_empty() => Some(true),
                Some(_) => None,
            })
            .flatten()
    }
}

impl FileAnswers {
    pub fn mapped(record: shards::MappedRecord) -> Option<Self> {
        let parsed = {
            let bytes = record.as_ref();
            let FileWire { context, pairs, blocks, complete, closed }: FileWire<&[u8]> = postcard::from_bytes(bytes).ok()?;
            let locations = pairs.into_iter().map(|(key, body)| {
                let start = body.as_ptr() as usize - bytes.as_ptr() as usize;
                (key, start..start + body.len())
            }).collect();
            (locations, blocks, context, complete, closed)
        };
        let (locations, blocks, context, complete, closed) = parsed;
        Some(Self { locations, mapped: Some(record), blocks, context, complete, closed, ..Self::default() })
    }

    pub fn decode(&self, key: &Key) -> Option<Local> {
        let range = self.locations.get(key)?;
        postcard::from_bytes(&self.mapped.as_ref()?.as_ref()[range.clone()]).ok()
    }

    pub fn read(&self, binding: Binding) -> Option<Cow<'_, Local>> {
        if let Some(local) = self.pairs.get(&binding.key) {
            return Some(Cow::Borrowed(local));
        }
        if self.locations.contains_key(&binding.key) {
            return self.decode(&binding.key).map(Cow::Owned);
        }
        if has_bit(&self.complete, binding.id) {
            Some(Cow::Owned(Local::empty(true)))
        } else if has_bit(&self.closed, binding.id) {
            Some(Cow::Owned(Local::empty(false)))
        } else {
            None
        }
    }

    /// The pair finished with no rows, issues or frontier, and kept its (empty) rows.
    pub fn complete_empty(&self, binding: Binding) -> bool {
        has_bit(&self.complete, binding.id)
    }

    pub fn remove(&mut self, binding: Binding) {
        self.locations.remove(&binding.key);
        self.pairs.remove(&binding.key);
        put_bit(&mut self.complete, binding.id, false);
        put_bit(&mut self.closed, binding.id, false);
    }

    pub fn insert(&mut self, binding: Binding, local: Local) {
        self.remove(binding);
        match local.trivial() {
            Some(true) => put_bit(&mut self.complete, binding.id, true),
            Some(false) => put_bit(&mut self.closed, binding.id, true),
            None => {
                self.pairs.insert(binding.key, local);
            }
        }
    }
}

pub struct LocalWire<'a>(&'a FileAnswers);
impl Serialize for LocalWire<'_> {
    fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{Error as _, SerializeMap as _};
        let file = self.0;
        let mut map = out.serialize_map(Some(file.locations.len() + file.pairs.len()))?;
        if let Some(payload) = &file.mapped {
            for (key, range) in &file.locations {
                map.serialize_entry(key, &RowBytes(&payload.as_ref()[range.clone()]))?;
            }
        }
        let mut buffer = Vec::new();
        for (key, pair) in &file.pairs {
            buffer.clear();
            buffer = postcard::to_extend(pair, buffer).map_err(S::Error::custom)?;
            map.serialize_entry(key, &RowBytes(&buffer))?;
        }
        map.end()
    }
}

impl Serialize for FileAnswers {
    fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            context: &'a Option<Proof>,
            pairs: LocalWire<'a>,
            blocks: &'a [BlockId],
            complete: &'a [u8],
            closed: &'a [u8],
        }
        Wire {
            context: &self.context,
            pairs: LocalWire(self),
            blocks: &self.blocks,
            complete: &self.complete,
            closed: &self.closed,
        }
        .serialize(out)
    }
}
pub fn kind(p: &Path) -> Res<u8> {
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
pub mod request_answers;
pub mod shards;
pub use request_answers::{InputScope, Packet};
use request_answers::{Group, Observations, Probe, Proof, Record};
use shards::{Index, Packets, Records, Segments, Shards};

#[derive(Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub names: Option<Key>,
    pub build: Key,
    pub files: Segments,
    pub notes: Index,
    pub requests: Index,
    pub groups: Index,
    pub packets: Index,
    pub packet_layers: Vec<Key>,
    pub blocks: Segments,
    pub stamps: Index,
    pub empty_schemas: Vec<Key>,
    pub request_layers: Vec<Key>,
    pub program_inputs: BTreeMap<Key, Vec<String>>,
    pub bindings: Vec<Key>,
}
pub struct PriorClosed {
    pub work: usize,
    pub source: usize,
    pub version: Key,
    pub record: Record,
}

pub struct RequestPlan {
    pub proof: Vec<Proof>,
    pub unread: Vec<(String, String)>,
    pub environments: BTreeSet<Arc<str>>,
}

pub struct Store {
    pub dir: PathBuf,
    pub names: Vec<PathBuf>,
    pub name_ids: HashMap<PathBuf, u32>,
    pub names_digest: Option<Key>,
    pub saved_names: usize,
    pub previous: Vec<u8>,
    pub build: Key,
    pub files: Records<FileAnswers>,
    pub blocks: Records<EncodedBlock>,
    pub empty_schemas: Vec<Key>,
    pub schema_ids: HashMap<Key, u32>,
    pub notes: Shards<Notes>,
    pub requests: Packets<Record>,
    pub groups: Shards<Group>,
    pub packets: Packets<Packet>,
    pub observed: Observations,
    pub work_inputs: HashMap<usize, InputScope>,
    pub pending_requests: Vec<(usize, Key)>,
    pub updated_proofs: HashMap<usize, Record>,
    pub prior_groups: HashMap<(Key, Key, Option<InputScope>), Option<(PathBuf, Key)>>,
    pub supplied_versions: HashMap<usize, Key>,
    pub versions: Vec<Option<Key>>,
    pub files_valid: Vec<bool>,
    pub notes_valid: Vec<bool>,
    pub pairs: Vec<Option<(usize, usize)>>,
    pub program_keys: Vec<Key>,
    pub program_inputs: BTreeMap<Key, Vec<String>>,
    pub bindings: Vec<Binding>,
    pub binding_table: Vec<Key>,
    pub binding_numbers: HashMap<Key, u32>,
    pub plans: Vec<Option<RequestPlan>>,
    pub observed_entries: HashMap<Arc<str>, BTreeMap<PathBuf, Option<u8>>>,
    pub environment_proofs: HashMap<Arc<str>, Option<Proof>>,
    pub reused_context: HashSet<(Arc<str>, Proof)>,
    pub keep_pairs: Vec<bool>,
    pub local_rows: Vec<bool>,
    pub project_proofs: HashMap<usize, Option<Proof>>,
    pub file_hits: usize,
    pub closed_hits: usize,
    pub evaluated_pairs: usize,
    pub evaluated_closed: usize,
    pub request_hits: usize,
    pub absent_file: usize,
    pub absent_local: usize,
    pub absent_frontier: usize,
    pub restored_blocks: usize,
    pub frontier_cpu: f64,
    pub packets_packed: usize,
    pub empty_packet: Option<Key>,
    pub empty_packet_reuses: usize,
    pub packed_rows: usize,
    pub rendered_packets: usize,
    pub pack_cpu: f64,
}
pub fn table(dir: &Path, name: &str, digest: Key) -> PathBuf {
    dir.join(format!(
        "{name}-{}.postcard",
        blake3::Hash::from_bytes(digest).to_hex()
    ))
}
pub fn load<T: DeserializeOwned>(p: &Path, digest: Key) -> Option<T> {
    let b = fs::read(p).ok()?;
    (*blake3::hash(&b).as_bytes() == digest)
        .then(|| postcard::from_bytes(&b).ok())
        .flatten()
}
pub fn atomic(dir: &Path, p: &Path, b: &[u8]) -> Res<()> {
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(b)?;
    tmp.as_file().sync_all()?;
    tmp.persist(p)?;
    Ok(())
}
impl Store {
    pub fn open(root: &Path, readers: &[impl session::source::Source]) -> Res<Self> {
        Self::directory(root.join(".ryo_cache"), readers)
    }
    pub fn directory(dir: PathBuf, readers: &[impl session::source::Source]) -> Res<Self> {
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

        let providers: Vec<_> = readers.iter().map(|r| (r.name(), r.identity())).collect();
        let build = key((build, providers));

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
            dir: dir.clone(),
            previous,
            build,
            files: Records::new("pairs", snapshot.files),
            blocks: Records::new("blocks", snapshot.blocks),
            schema_ids: snapshot
                .empty_schemas
                .iter()
                .enumerate()
                .map(|(i, &k)| (k, i as u32))
                .collect(),
            empty_schemas: snapshot.empty_schemas,
            notes: Shards::new("project", snapshot.notes),
            requests: Packets::new("requests", snapshot.requests, snapshot.request_layers).coarse(6),
            groups: Shards::new("inputs", snapshot.groups).coarse(4),
            packets: Packets::new("answers", snapshot.packets, snapshot.packet_layers),
            observed: Observations::new(dir, snapshot.stamps),
            work_inputs: HashMap::new(),
            pending_requests: Vec::new(),
            updated_proofs: HashMap::new(),
            prior_groups: HashMap::new(),
            supplied_versions: HashMap::new(),
            versions: vec![],
            files_valid: vec![],
            notes_valid: vec![],
            pairs: vec![],
            program_keys: vec![],
            program_inputs: snapshot.program_inputs,
            bindings: vec![],
            binding_numbers: snapshot
                .bindings
                .iter()
                .enumerate()
                .map(|(i, &k)| (k, i as u32))
                .collect(),
            binding_table: snapshot.bindings,
            plans: vec![],
            observed_entries: HashMap::new(),
            environment_proofs: HashMap::new(),
            reused_context: HashSet::new(),
            keep_pairs: vec![],
            local_rows: vec![],
            project_proofs: HashMap::new(),
            file_hits: 0,
            closed_hits: 0,
            evaluated_pairs: 0,
            evaluated_closed: 0,
            request_hits: 0,
            absent_file: 0,
            absent_local: 0,
            absent_frontier: 0,
            restored_blocks: 0,
            frontier_cpu: 0.0,
            packets_packed: 0,
            empty_packet: None,
            empty_packet_reuses: 0,
            packed_rows: 0,
            rendered_packets: 0,
            pack_cpu: 0.0,
        })
    }
    pub fn publish(&mut self) -> Res<usize> {
        if !(self.files.dirty()
            || self.blocks.dirty()
            || self.notes.dirty()
            || self.requests.dirty()
            || self.groups.dirty()
            || self.packets.dirty())
        {
            return Ok(0);
        }
        let guard = fs::File::open(&self.dir)?;
        guard.lock()?;
        let head = self.dir.join("session.postcard");
        if fs::read(&head).unwrap_or_default() != self.previous {
            return Ok(0);
        }
        if !self.observed.still_current() {
            return Ok(0);
        }
        let mut old = vec![];
        let t0 = crate::thread_cpu();
        let mut written = self.files.flush(&self.dir, &mut old)?;
        let block_start = crate::thread_cpu();
        written += self.blocks.flush(&self.dir, &mut old)?;
        eprintln!(
            "blocks_publish_cpu={:.6}",
            crate::thread_cpu() - block_start
        );
        let t1 = crate::thread_cpu();
        written += self.notes.flush(&self.dir, &mut old)?;
        let t2 = crate::thread_cpu();
        written += self.requests.flush(&self.dir, &mut old)?;
        let t3 = crate::thread_cpu();
        written += self.groups.flush(&self.dir, &mut old)?;
        let t4 = crate::thread_cpu();
        written += self.packets.flush(&self.dir, &mut old)?;
        let t5 = crate::thread_cpu();
        written += self.observed.stamps.flush(&self.dir, &mut old)?;
        let stamp_end = crate::thread_cpu();
        eprintln!("stamps_publish_cpu={:.6}", stamp_end - t5);
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
            requests: self.requests.index().clone(),
            groups: self.groups.index.clone(),
            packets: self.packets.index().clone(),
            packet_layers: self.packets.layers.clone(),
            blocks: self.blocks.index.clone(),
            stamps: self.observed.stamps.index.clone(),
            empty_schemas: self.empty_schemas.clone(),
            request_layers: self.requests.layers.clone(),
            program_inputs: self.program_inputs.clone(),
            bindings: self.binding_table.clone(),
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
        eprintln!(
            "publish_cpu pairs={:.6} project={:.6} requests={:.6} inputs={:.6} answers={:.6} paths_and_manifest={:.6}",
            t1 - t0,
            t2 - t1,
            t3 - t2,
            t4 - t3,
            t5 - t4,
            crate::thread_cpu() - stamp_end
        );
        Ok(written)
    }
    pub fn costs(&self) {
        eprintln!(
            "table_read_cpu pairs={:.6} project={:.6} requests={:.6} inputs={:.6} answers={:.6}",
            self.files.read_cpu,
            self.notes.read_cpu,
            self.requests.read_cpu,
            self.groups.read_cpu,
            self.packets.read_cpu
        );
        eprintln!(
            "packets_packed={} empty_packet_reuses={} packed_rows={} packet_build_cpu={:.6}",
            self.packets_packed, self.empty_packet_reuses, self.packed_rows, self.pack_cpu
        );
        eprintln!("rendered_packets={}", self.rendered_packets);
        eprintln!(
            "block_table read={} written={} read_cpu={:.6} empty_schemas={}",
            self.blocks.read_bytes,
            self.blocks.write_bytes,
            self.blocks.read_cpu,
            self.empty_schemas.len()
        );
        let o = &self.observed;
        eprintln!("observation_open_cpu={:.6}", o.open_cpu);
        eprintln!(
            "stamps stats={} hits={} young={} changed={} read={} written={} read_cpu={:.6}",
            o.stats, o.stamp_hits, o.young, o.stamp_changed,
            o.stamps.read_bytes, o.stamps.write_bytes, o.stamps.read_cpu
        );
        eprintln!(
            "observations source_reads={} contract_reads={} source_bytes={} contract_bytes={} path_hits={} inode_hits={} groups={} group_hits={}",
            o.reads[0],
            o.reads[1],
            o.bytes[0],
            o.bytes[1],
            o.path_hits,
            o.inode_hits,
            o.checked_groups(),
            o.group_hits
        );
        eprintln!(
            "contributions restored_blocks={} frontier_cpu={:.6} missing_file_slots={} missing_local_bindings={} missing_frontier_bindings={}",
            self.restored_blocks, self.frontier_cpu, self.absent_file, self.absent_local, self.absent_frontier
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
    /// Recover which inputs a previously prepared program reads. This is
    /// independent of the values the current request binds to those inputs.
    pub fn program_inputs(&self, program: Key) -> Option<&[String]> {
        self.program_inputs.get(&program).map(Vec::as_slice)
    }

    pub fn remember_program_inputs(&mut self, program: Key, names: &[String]) {
        self.program_inputs.entry(program).or_insert_with(|| names.to_vec());
    }

    pub fn request_inputs(
        &mut self,
        root: &Path,
        inventory: Key,
        peers: Key,
        supplied: BTreeMap<String, Key>,
    ) -> InputScope {
        self.observed.request_inputs(root, inventory, peers, supplied)
    }

    pub fn bind_work(&mut self, work: usize, scope: InputScope) {
        self.work_inputs.insert(work, scope);
    }

    pub fn offer(&mut self, work: usize, id: Key) {
        self.pending_requests.push((work, id));
    }

    pub fn truncate_work(&mut self, len: usize) {
        self.pending_requests.retain(|&(work, _)| work < len);
        self.work_inputs.retain(|&work, _| work < len);
    }

    /// Admission supplies the durable identity once the source has been resolved.
    pub fn bind_version(&mut self, source: usize, version: Key) {
        self.supplied_versions.insert(source, version);
    }
    pub fn inventory(&mut self, i: usize, path: PathBuf, expected: Option<Key>) {
        let evidence = self.group(vec![(path, Probe::Bytes)], None);
        if expected.is_some()
            && let Some(g) = evidence
            && let Some(plan) = &mut self.plans[i]
        {
            plan.proof.push(g);
        } else {
            self.plans[i] = None;
        }
    }
    pub fn contract_file(&mut self, p: &Path) -> Option<Key> {
        self.observed.observe(p, Probe::Bytes)
    }
    pub fn lookup(&mut self, id: Key) -> Option<(Key, &[(String, String)])> {
        self.lookup_for(None, id)
    }

    pub fn lookup_in(&mut self, scope: InputScope, id: Key) -> Option<(Key, &[(String, String)])> {
        self.lookup_for(Some(scope), id)
    }

    pub fn lookup_for(&mut self, scope: Option<InputScope>, id: Key) -> Option<(Key, &[(String, String)])> {
        self.requests.open(&self.dir, &id);
        let r = self.requests.get(&id)?;
        for &(group, expected) in &r.proof {
            self.groups.open(&self.dir, &group);
            if self.observed.group(group, self.groups.get(&group)?, &self.names, scope) != Some(expected) {
                return None;
            }
        }
        self.packets.open(&self.dir, &r.packet);
        self.packets.get(&r.packet)?;
        self.request_hits += 1;
        Some((r.packet, &r.unread))
    }
    pub fn packet(&self, id: &Key) -> &Packet {
        self.packets.get(id).expect("verified packet")
    }
    pub fn save(&mut self, id: Key, idx: usize, r: &super::Receipt<'_>) -> Option<Key> {
        if let Some(record) = self.updated_proofs.remove(&idx) {
            let packet = record.packet;
            self.requests.insert(id, record);
            return Some(packet);
        }
        let RequestPlan { mut proof, unread, environments } = self.plans.get_mut(idx)?.take()?;
        for environment in environments {
            proof.push(self.environment_proof(environment)?);
        }
        let start = crate::thread_cpu();
        let empty = r.rows.is_empty() && r.unknowns.is_empty() && r.rejected_syntax.is_empty();
        let k = if let Some(packet) = self.empty_packet.filter(|_| empty) {
            self.record(id, packet, proof, unread);
            self.empty_packet_reuses += 1;
            packet
        } else {
            let k = self.put(id, Packet::save(r), proof, unread, || super::digest(&r.rows));
            if empty {
                self.empty_packet = Some(k);
            }
            self.packets_packed += 1;
            self.packed_rows += r.rows.len();
            k
        };
        self.pack_cpu += crate::thread_cpu() - start;
        Some(k)
    }
    pub fn decline(&mut self, id: Key, reason: &crate::check_session::refusal::Refusal) {
        self.put(id, Packet::declined(reason), vec![], vec![], String::new);
    }
    pub fn put(
        &mut self,
        id: Key,
        mut packet: Packet,
        proof: Vec<Proof>,
        unread: Vec<(String, String)>,
        render: impl FnOnce() -> String,
    ) -> Key {
        let k = key((&packet.refusal, &packet.unknowns, &packet.rejected_syntax, packet.typed_bytes()));
        self.packets.open(&self.dir, &k);
        if self.packets.get(&k).is_none() {
            packet.digest = render();
            self.rendered_packets += 1;
            self.packets.insert(k, packet);
        }
        self.record(id, k, proof, unread);
        k
    }
    pub fn record(&mut self, id: Key, packet: Key, proof: Vec<Proof>, unread: Vec<(String, String)>) {
        self.requests.open(&self.dir, &id);
        self.requests.insert(
            id,
            Record {
                packet,
                proof,
                unread,
            },
        );
    }
    pub fn group(&mut self, mut paths: Vec<(PathBuf, Probe)>, scope: Option<InputScope>) -> Option<Proof> {
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
        let expected = self.observed.group(id, &g, &self.names, scope)?;
        if self.groups.get(&id).is_none() {
            self.groups.insert(id, g);
        }
        Some((id, expected))
    }
    pub fn prepare(&mut self, b: &Batch) {
        self.project_proofs.clear();
        self.environment_proofs.clear();
        self.reused_context.clear();
        let prior = self.versions.len();
        self.versions
            .extend(b.schedule.sources.iter().enumerate().skip(prior).map(|(index, source)| {
                if let Some(&version) = self.supplied_versions.get(&index) {
                    return source.version_key(version);
                }
                let d = if let Some(v) = &source.provided {
                    *blake3::hash(v).as_bytes()
                } else {
                    self.observed
                        .observe(&Path::new(&*source.root).join(&source.path), Probe::Source)?
                };
                source.version_key(key((&source.path, &source.reader, source.permission, d)))
            }));
        // Programs and bindings are appended to the schedule. Their effective
        // inputs stay the same when earlier answers are delivered.
        let prior_programs = self.program_keys.len();
        if prior_programs < b.schedule.programs.len() {
            self.program_keys.resize(b.schedule.programs.len(), [0; 32]);
            for ((text, params), &idx) in &b.schedule.prepared {
                if idx >= prior_programs {
                    self.program_keys[idx] = key((text, params));
                }
            }
        }
        for (binding, host) in b.bindings.iter().enumerate().skip(self.bindings.len()) {
            let rule = self.program_keys[b.program_of[binding]];
            // The projected work before a closed join uses the hosted
            // schema. The closed request key binds the consumed rows.
            let identity = if b.cut_of[binding].is_some() {
                let shape = host
                    .iter()
                    .map(|h| (&h.name, &h.columns))
                    .collect::<Vec<_>>();
                key((0u8, rule, shape))
            } else {
                key((1u8, rule, host))
            };
            let table = &mut self.binding_table;
            let id = *self.binding_numbers.entry(identity).or_insert_with(|| {
                table.push(identity);
                u32::try_from(table.len() - 1).expect("binding table fits a number")
            });
            self.bindings.push(Binding { key: identity, id });
        }
        self.pairs.resize(b.schedule.pairs, None);
        for (i, src) in b.schedule.sources.iter().enumerate() {
            for (binding, slot) in src.bindings.iter() {
                self.pairs[slot] = Some((i, binding));
            }
        }
    }

    pub fn changed_record(&mut self, id: Key, scope: Option<InputScope>) -> Option<((PathBuf, Key), Record)> {
        self.requests.open(&self.dir, &id);
        let mut record = self.requests.get(&id)?.clone();
        // The previous result must not hide a changed or transient read failure.
        if !record.unread.is_empty() {
            return None;
        }
        let mut changed = None;
        for (group, expected) in &mut record.proof {
            self.groups.open(&self.dir, group);
            let paths = self.groups.get(group)?;
            let current = self.observed.group(*group, paths, &self.names, scope)?;
            if current == *expected {
                continue;
            }
            let old = self.prior_groups.entry((*group, *expected, scope)).or_insert_with(|| {
                self.observed.prior_leaf(paths, &self.names, scope, *expected)
            }).as_ref()?;
            if changed.as_ref().is_some_and(|other| other != old) {
                return None;
            }
            changed = Some(old.clone());
            *expected = current;
        }
        Some((changed?, record))
    }

    /// Prove the unchanged inputs before opening their saved contributions.
    pub fn probe(&mut self, b: &Batch) -> (Vec<usize>, Vec<PriorClosed>) {
        self.updated_proofs.clear();
        self.prior_groups.clear();
        let pending = std::mem::take(&mut self.pending_requests);
        if pending.is_empty() || !self.observed.has_prior_sources() {
            return (vec![], vec![]);
        }
        let mut sources = HashMap::<_, Option<usize>>::new();
        for &i in &b.schedule.pending {
            let source = &b.schedule.sources[i];
            if source.provided.is_none() {
                sources.entry(Path::new(&*source.root).join(&source.path))
                    .and_modify(|prior| *prior = None).or_insert(Some(i));
            }
        }
        let mut files = BTreeSet::new();
        let mut candidates = Vec::new();
        for (work, id) in pending {
            let Work::Closed { pairs, project, .. } = &b.work[work] else { continue };
            let scope = self.work_inputs.get(&work).copied();
            let Some(((path, digest), record)) = self.changed_record(id, scope) else { continue };
            let Some(source) = sources.get(&path).copied().flatten() else { continue };
            if !(pairs.iter().any(|&slot| self.pairs[slot].is_some_and(|(i, _)| i == source))
                || project.is_some_and(|p| b.projects[p].slots.contains(&source))) {
                continue;
            }
            let src = &b.schedule.sources[source];
            let Some(version) = src.version_key(key((&src.path, &src.reader, src.permission, digest))) else { continue };
            files.insert(source);
            candidates.push(PriorClosed { work, source, version, record });
        }
        if !files.is_empty() {
            self.keep_pairs = vec![false; self.pairs.len()];
            self.local_rows = vec![false; self.pairs.len()];
            for work in &b.work {
                let closed = matches!(work, Work::Closed { .. });
                let slots = work.pair_slots();
                let shared = slots.clone().count() > 1;
                for slot in slots {
                    self.keep_pairs[slot] |= closed || shared;
                    self.local_rows[slot] |= !closed;
                }
            }
            self.files_valid.resize(self.versions.len(), false);
            self.notes_valid.resize(self.versions.len(), false);
            for &i in &files {
                self.files_valid[i] = false;
                self.notes_valid[i] = false;
                self.evaluated_pairs += b.schedule.sources[i].bindings.len();
            }
        }
        (files.into_iter().collect(), candidates)
    }

    pub fn reuse_closed(&mut self, b: &mut Batch, candidates: Vec<PriorClosed>) {
        let count = candidates.len();
        let already_reused = self.closed_hits;
        let started = crate::thread_cpu();
        for prior in candidates {
            let Work::Closed { pairs, project, .. } = &b.work[prior.work] else { continue };
            let Some(current) = self.versions[prior.source] else { continue };
            let source = &b.schedule.sources[prior.source];
            if project.is_some_and(|p| b.projects[p].slots.contains(&prior.source)) {
                self.notes.open(&self.dir, &prior.version);
                if self.notes.get(&prior.version).is_none()
                    || self.notes.get(&prior.version) != self.notes.get(&current) {
                    continue;
                }
            }
            let pairs_match = pairs.iter().all(|&slot| {
                let Some((index, binding)) = self.pairs[slot] else { return false };
                index != prior.source || self.same_frontier(prior.version, current, self.bindings[binding])
            });
            if !pairs_match {
                continue;
            }
            self.packets.open(&self.dir, &prior.record.packet);
            let Some(packet) = self.packets.get(&prior.record.packet) else { continue };
            if packet.refusal.is_some() {
                continue;
            }
            let report = packet.report();
            // The closed delivery locates spans against source bytes. A matching
            // relational projection alone cannot prove a changed line layout.
            if report.rows.iter().any(|row| row.span_file().is_some_and(|path| path.as_ref() == source.path)) {
                continue;
            }
            b.work[prior.work] = Work::Project(Rc::new(Ok(report)));
            self.updated_proofs.insert(prior.work, prior.record);
            self.closed_hits += 1;
        }
        eprintln!(
            "retained_closed_proof candidates={count} reused={} cpu={:.6}",
            self.closed_hits - already_reused,
            crate::thread_cpu() - started,
        );
    }

    pub fn same_frontier(&mut self, before: Key, after: Key, binding: Binding) -> bool {
        self.files.open_mapped(&self.dir, &before, FileAnswers::mapped);
        let (Some(old), Some(new)) = (self.files.get(&before), self.files.get(&after)) else {
            return false;
        };
        let (Some(left), Some(right)) = (old.read(binding), new.read(binding)) else {
            return false;
        };
        if old.context != new.context || left.report.issues != right.report.issues
            || left.unavailable != right.unavailable || left.frontier.len() != right.frontier.len()
        {
            return false;
        }
        let fingerprint = |file: &FileAnswers, index: usize| match *file.blocks.get(index)? {
            BlockId::Empty(i) => Some((false, *self.empty_schemas.get(i as usize)?)),
            BlockId::Local(mut key) => {
                // The first byte locates its segment. The rest identify the
                // typed column bytes independently of the file version.
                key[0] = 0;
                Some((true, key))
            }
        };
        left.frontier.iter().zip(&right.frontier).all(|(&(node, a), &(other, b))| {
            node == other && fingerprint(old, a).is_some_and(|k| Some(k) == fingerprint(new, b))
        })
    }

    pub fn admit(
        &mut self,
        b: &mut Batch,
        world: &mut session::World,
        answers: &mut super::ReportSlots,
        contributions: &mut [Option<Rc<project::Contribution>>],
        pre_read: &[usize],
    ) {
        // The probe may have already recorded evidence for the edited file.
        // Later requests can add observations when their contributions replay.
        self.environment_proofs.clear();
        let mut needed = vec![false; self.pairs.len()];
        let mut frontier = vec![false; self.pairs.len()];
        let mut frontier_sources = vec![false; b.schedule.sources.len()];
        self.keep_pairs = vec![false; self.pairs.len()];
        self.local_rows = vec![false; self.pairs.len()];
        let mut projects = BTreeSet::new();
        let mut groups = rustc_hash::FxHashMap::<(Option<InputScope>, Vec<usize>), Option<(Proof, BTreeSet<Arc<str>>)>>::default();
        let work_inputs = std::mem::take(&mut self.work_inputs);
        self.plans.clear();
        for (index, work) in b.work.iter().enumerate() {
            let scope = work_inputs.get(&index).copied();
            let (project, closed) = match work {
                Work::Pairs(_) | Work::Project(_) => (None, false),
                Work::Closed { project, .. } => (*project, true),
                Work::Refused(_) | Work::Invalid(_) => {
                    self.plans.push(None);
                    continue;
                }
            };
            if closed {
                self.evaluated_closed += 1;
            }
            let pairs = work.pair_slots();
            let shared = pairs.clone().count() > 1;
            for slot in pairs.clone() {
                needed[slot] = true;
                // A closed input consumes its frontier and issues. Only a
                // local receipt reads the emitted rows directly.
                self.local_rows[slot] |= !closed;
                frontier[slot] |= closed;
                if closed && let Some((source, _)) = self.pairs[slot] {
                    frontier_sources[source] = true;
                }
                self.keep_pairs[slot] |= closed || shared;
            }
            let local = pairs
                .map(|slot| self.pairs[slot].map(|(source, _)| source))
                .collect::<Option<Vec<_>>>();
            let mut proof = Some(Vec::new());
            let mut environments = BTreeSet::new();
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
                    let g = groups.entry((scope, ids.to_vec())).or_insert_with(|| {
                        let mut paths = Vec::new();
                        let mut environment_ids = BTreeSet::new();
                        for &i in ids {
                            let src = &b.schedule.sources[i];
                            if let Some(environment) = &src.environment {
                                scope?;
                                environment_ids.insert(environment.id.clone());
                            } else {
                                self.versions[i]?;
                            }
                            if scope.is_some() {
                                paths.push((src.path.as_str().into(), Probe::Effective));
                            } else if src.provided.is_none() {
                                paths.push((Path::new(&*src.root).join(&src.path), Probe::Source));
                            }
                        }
                        Some((self.group(paths, scope)?, environment_ids))
                    });
                    if let (Some(ps), Some((group, ids))) = (&mut proof, g) {
                        ps.push(*group);
                        environments.extend(ids.iter().cloned());
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
            if scope.is_some()
                && (closed || !environments.is_empty() || matches!(work, Work::Project(_)))
                && let Some(ps) = &mut proof
            {
                ps.push(
                    self.group(vec![(PathBuf::new(), Probe::Inventory)], scope)
                        .expect("declared inventory"),
                );
            }
            self.plans.push(proof.map(|proof| RequestPlan { proof, unread: Vec::new(), environments }));
        }
        self.files_valid.resize(self.versions.len(), false);
        self.notes_valid.resize(self.versions.len(), false);
        self.files_valid.fill(false);
        self.notes_valid.fill(false);
        for (i, src) in b.schedule.sources.iter().enumerate() {
            let Some(version) = self.versions[i] else { continue };
            if projects.contains(&i) {
                self.notes.open(&self.dir, &version);
                let context = self.notes.get(&version).map(|notes| notes.context);
                self.notes_valid[i] = context.is_some_and(|proof| {
                    self.reuse_context(src.environment.as_deref(), proof)
                });
            }
            if frontier_sources[i] || src.bindings.iter().any(|(_, slot)| self.keep_pairs[slot]) {
                self.files.open_mapped(&self.dir, &version, FileAnswers::mapped);
                let context = self.files.get(&version).map(|file| file.context);
                self.files_valid[i] = context.is_some_and(|proof| {
                    self.reuse_context(src.environment.as_deref(), proof)
                });
            }
        }
        let restore_start = crate::thread_cpu();
        let mut restored = HashMap::new();
        let mut damaged = Vec::new();
        // Later live requests can consume a file answer delivered in an earlier epoch.
        // Its source no longer has a pending binding, but its stable pair key remains.
        for (slot, requested) in frontier.into_iter().enumerate() {
            if !requested {
                continue;
            }
            if world.frontier_parts(slot).is_some() {
                continue;
            }
            let Some((source, binding)) = self.pairs[slot] else {
                continue;
            };
            let Some(version) = self.versions[source] else {
                continue;
            };
            if !self.files_valid[source] {
                continue;
            }
            let binding = self.bindings[binding];
            let Some(file) = self.files.get(&version) else {
                continue;
            };
            let Some(pair) = file.read(binding) else {
                continue;
            };
            let parts = pair
                .frontier
                .iter()
                .map(|&(node, id)| {
                    let k = match *file.blocks.get(id)? {
                        BlockId::Empty(i) => *self.empty_schemas.get(i as usize)?,
                        BlockId::Local(k) => k,
                    };
                    let block = if let Some(block) = restored.get(&k) {
                        Rc::clone(block)
                    } else {
                        self.blocks.open_mapped(&self.dir, &k, EncodedBlock::mapped);
                        let block = self.blocks.get(&k)?.restore(world)?;
                        restored.insert(k, block.clone());
                        block
                    };
                    Some((node, block))
                })
                .collect::<Option<Vec<_>>>();
            let Some(parts) = parts else {
                damaged.push((version, binding));
                continue;
            };
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
        for (version, binding) in damaged {
            self.files.change(&version).remove(binding);
        }
        self.restored_blocks += restored.len();
        self.frontier_cpu += crate::thread_cpu() - restore_start;
        for (i, src) in b.schedule.sources.iter_mut().enumerate() {
            if pre_read.binary_search(&i).is_ok() {
                continue;
            }
            if !projects.contains(&i) {
                src.project = false;
            }
            let held = self.versions[i].and_then(|v| self.files.get(&v))
                .filter(|_| self.files_valid[i]);
            if let Some(notes) = self.versions[i]
                .and_then(|v| self.notes.get(&v))
                .filter(|_| src.project && self.notes_valid[i])
            {
                contributions[i] = Some(Rc::new(project::Contribution {
                    reader: notes.reader,
                    notes: l::Bytes(notes.notes.clone()),
                    manifest: notes.manifest.clone().map(l::ManifestText),
                }));
                src.project = false;
            }
            src.bindings.keep(|(binding, slot)| {
                if !needed[slot] {
                    return false;
                }
                let Some((_, bind)) = self.pairs[slot] else {
                    return true;
                };
                let Some(file) = held else {
                    self.absent_file += 1;
                    return true;
                };
                let bound = self.bindings[bind];
                if file.complete_empty(bound) {
                    answers.set(slot, Ok(session::Reported::default()));
                    self.file_hits += 1;
                    return false;
                }
                let Some(pair) = file.read(bound) else {
                    if b.cut_of[binding].is_some() {
                        self.absent_frontier += 1
                    } else {
                        self.absent_local += 1
                    }
                    return true;
                };
                let Some(report) = pair.report.read(self.local_rows[slot]) else {
                    return true;
                };
                answers.set(slot, Ok(report));
                self.file_hits += 1;
                false
            });
        }
        b.schedule.pending.retain(|i| {
            if pre_read.binary_search(i).is_ok() {
                return false;
            }
            let src = &b.schedule.sources[*i];
            self.evaluated_pairs += src.bindings.len();
            src.project || !src.bindings.is_empty()
        });
    }

    pub fn local(
        &mut self,
        b: &Batch,
        world: &session::World,
        answers: &super::ReportSlots,
        aliases: &session::worker::PairAliases,
        contributions: &[Option<Rc<project::Contribution>>],
        pending: &[usize],
    ) {
        let mut encoded = HashMap::new();
        let mut empty_blocks = HashMap::new();
        let mut observed_environments = BTreeSet::new();
        for &i in pending {
            let src = &b.schedule.sources[i];
            if let Some(environment) = &src.environment
                && observed_environments.insert(&environment.id)
            {
                self.remember_entries(environment);
            }
            let Some(version) = self.versions[i] else {
                continue;
            };
            if src.bindings.is_empty() && !src.project {
                continue;
            }
            let context = match &src.environment {
                None => None,
                Some(environment) => match self.environment_proof(environment.id.clone()) {
                    Some(proof) => Some(proof),
                    None => continue,
                },
            };
            if src.project
                && let Some(notes) = &contributions[i]
                && !self.notes_valid[i]
            {
                // The changed-file probe can finish before regular admission
                // has opened this version's shard.
                self.notes.open(&self.dir, &version);
                self.notes.insert(
                    version,
                    Notes {
                        context,
                        reader: notes.reader,
                        notes: notes.notes.0.clone(),
                        manifest: notes.manifest.as_ref().map(|m| m.0.clone()),
                    },
                );
            }
            if !src.bindings.iter().any(|(_, s)| self.keep_pairs[s]) {
                continue;
            }
            let file = self.files.change(&version);
            if !self.files_valid[i] {
                *file = FileAnswers::default();
                self.files_valid[i] = true;
            }
            file.context = context;
            for (_, slot) in src.bindings.iter() {
                if !self.keep_pairs[slot] {
                    continue;
                }
                let Some((_, binding)) = self.pairs[slot] else {
                    continue;
                };
                let binding = self.bindings[binding];
                let a = aliases.lookup(slot);
                let Ok(report) = &answers[a] else { continue };
                if file.read(binding)
                    .is_some_and(|p| !self.local_rows[slot] || p.report.rows.is_some())
                {
                    continue;
                }
                let mut frontier = Vec::new();
                if let Some(parts) = world.frontier_parts(a) {
                    for (node, block) in parts {
                        let addr = (version, Rc::as_ptr(block));
                        // The encoded identity survives a reset of the file
                        // record. Its row number belongs to the current record.
                        let reference = *encoded.entry(addr).or_insert_with(|| {
                            if block.n == 0 {
                                *empty_blocks.entry(Rc::as_ptr(block)).or_insert_with(|| {
                                    let bytes = session::stored_block::encode(block, world);
                                    let k = *blake3::hash(&bytes).as_bytes();
                                    let i = *self.schema_ids.entry(k).or_insert_with(|| {
                                        self.empty_schemas.push(k);
                                        u32::try_from(self.empty_schemas.len() - 1)
                                            .expect("schema number")
                                    });
                                    self.blocks.open_mapped(&self.dir, &k, EncodedBlock::mapped);
                                    if self.blocks.get(&k).is_none() {
                                        *self.blocks.change(&k) = EncodedBlock::save(bytes);
                                    }
                                    BlockId::Empty(i)
                                })
                            } else {
                                let bytes = session::stored_block::encode(block, world);
                                let mut k = *blake3::hash(&bytes).as_bytes();
                                // Keep replacement columns for one file version in its own prefix.
                                k[0] = version[0];
                                self.blocks.open_mapped(&self.dir, &k, EncodedBlock::mapped);
                                if self.blocks.get(&k).is_none() {
                                    *self.blocks.change(&k) = EncodedBlock::save(bytes);
                                }
                                BlockId::Local(k)
                            }
                        });
                        let index = *file.known_blocks.entry(reference).or_insert_with(|| {
                            file.blocks.push(reference);
                            file.blocks.len() - 1
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
                file.insert(
                    binding,
                    Local {
                        report: Report::save(report, self.local_rows[slot]),
                        frontier,
                        unavailable,
                    },
                );
            }
        }
    }

    pub fn closed(
        &mut self,
        idx: usize,
        value: &Answer,
        pop: Option<&Population>,
        project: Option<(usize, &Authority)>,
    ) {
        if value.is_err() {
            self.plans[idx] = None;
            return;
        }
        let Some(p) = pop else { return };
        let unread = p.refused.iter()
            .map(|r| (r.path.0.to_string(), r.reason.0.to_string()))
            .collect();
        let Some((project, root)) = project.and_then(|(id, authority)| authority.disk().map(|root| (id, root))) else {
            if let Some(plan) = &mut self.plans[idx] {
                plan.unread = unread;
            }
            return;
        };
        let evidence = if let Some(g) = self.project_proofs.get(&project) {
            *g
        } else {
            let g = self.population_proof(p, root);
            self.project_proofs.insert(project, g);
            g
        };
        if let Some(g) = evidence
            && let Some(plan) = &mut self.plans[idx]
        {
            plan.proof.push(g);
            plan.unread = unread;
        } else {
            self.plans[idx] = None
        }
    }

    pub fn reuse_context(&mut self, environment: Option<&project::Environment>, proof: Option<Proof>) -> bool {
        let Some(environment) = environment else {
            return proof.is_none();
        };
        let Some((group, expected)) = proof else {
            return false;
        };
        let reuse = (environment.id.clone(), (group, expected));
        if self.reused_context.contains(&reuse) {
            return true;
        }
        self.groups.open(&self.dir, &group);
        let Some(paths) = self.groups.get(&group) else {
            return false;
        };
        if paths.iter().any(|&(_, mode)| mode != Probe::Kind)
            || self.observed.group(group, paths, &self.names, None) != Some(expected)
        {
            return false;
        }
        // A new request may combine a saved contribution with freshly evaluated
        // files. Its proof must include observations of every saved contribution.
        let entries = self.observed_entries.entry(environment.id.clone()).or_default();
        for &(name, _) in paths {
            let path = &self.names[name as usize];
            merge_entry(entries, path.clone(), self.observed.observe_kind(path));
        }
        self.reused_context.insert(reuse);
        true
    }

    pub fn remember_entries(&mut self, environment: &project::Environment) {
        let entries = self.observed_entries.entry(environment.id.clone()).or_default();
        environment.observed(|relative, observed| {
            let state = environment.authority.disk().and_then(|_| entry_kind(observed));
            let path = environment.authority.disk().map_or_else(
                || PathBuf::from(relative),
                |root| Path::new(root).join(relative),
            );
            // Later work can borrow rows from an earlier round in this overlay.
            // Include its observations and decline if a probe gave conflicting evidence.
            merge_entry(entries, path, state);
        });
    }

    pub fn environment_proof(&mut self, environment: Arc<str>) -> Option<Proof> {
        if let Some(&proof) = self.environment_proofs.get(&environment) {
            return proof;
        }
        let mut paths = Vec::new();
        let proof = self.observed_entries.get(&environment).and_then(|entries| {
            for (path, &state) in entries {
                if self.observed.observe_kind(path) != Some(state?) {
                    return None;
                }
                paths.push((path.clone(), Probe::Kind));
            }
            Some(())
        }).and_then(|()| self.group(paths, None));
        self.environment_proofs.insert(environment, proof);
        proof
    }

    pub fn population_proof(&mut self, p: &Population, root: &str) -> Option<Proof> {
        let mut paths = vec![(PathBuf::from(root), Probe::Kind)];
        for entry in &p.observations {
            let k = entry_kind(entry.state)?;
            let path = Path::new(root).join(&*entry.path.0);
            if self.observed.observe(&path, Probe::Kind) != Some(key(k)) {
                return None;
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
        self.group(paths, None)
    }
}

pub fn merge_entry(entries: &mut BTreeMap<PathBuf, Option<u8>>, path: PathBuf, state: Option<u8>) {
    let prior = entries.entry(path).or_insert(state);
    if *prior != state {
        *prior = None;
    }
}

pub fn entry_kind(state: l::ObservedEntryState) -> Option<u8> {
    use l::ObservedEntryState::*;
    Some(match state {
        Missing => 0,
        File => 1,
        Directory => 2,
        Other => 3,
        Unknown => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use session::source::Fake;

    #[test]
    fn mapped_replay_copies_untouched_rows_into_a_changed_file_record() {
        let dir = tempfile::tempdir().unwrap();
        let version = key("version");
        let numbered = |name: &str, id| Binding { key: key(name), id };
        let replaced = numbered("replaced", 0);
        let untouched = numbered("untouched", 1);
        let removed = numbered("removed", 2);
        let evaluated = numbered("evaluated", 3);
        let origin = Some((key("entries"), key("content")));
        let local = |rows| Local {
            report: Report { rows, issues: None },
            frontier: vec![],
            unavailable: vec![],
        };
        let mut records = Records::<FileAnswers>::new("pairs", Default::default());
        let row = records.change(&version);
        row.context = origin;
        for binding in [replaced, untouched, removed] {
            row.insert(binding, local(Some(vec![])));
        }
        row.insert(evaluated, local(Some(vec![1, 2, 3])));
        let mut obsolete = vec![];
        records.flush(dir.path(), &mut obsolete).unwrap();

        let mut reopened = Records::new("pairs", records.index);
        reopened.open_mapped(dir.path(), &version, FileAnswers::mapped);
        let row = reopened.change(&version);
        assert!(matches!(row.read(untouched), Some(Cow::Owned(_))));
        row.insert(replaced, local(None));
        row.remove(removed);
        reopened.flush(dir.path(), &mut obsolete).unwrap();

        let mut published = Records::new("pairs", reopened.index);
        published.open_mapped(dir.path(), &version, FileAnswers::mapped);
        let row = published.get(&version).unwrap();
        assert_eq!(row.context, origin);
        assert_eq!(row.read(untouched).unwrap().report.rows, Some(vec![]));
        assert_eq!(row.read(replaced).unwrap().report.rows, None);
        assert_eq!(row.read(evaluated).unwrap().report.rows, Some(vec![1, 2, 3]));
        assert!(row.read(removed).is_none());
    }

    #[test]
    fn contextual_proofs_recheck_absence_and_reject_unknown_entries() {
        let root = tempfile::tempdir().unwrap();
        let open = || Store::open(root.path(), &[] as &[Fake]).unwrap();
        let epoch = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let environment = project::Environment::new(
            "observed".into(),
            Authority::Disk(root.path().to_str().unwrap().to_owned()),
            BTreeSet::new(),
            epoch.clone(),
        );
        let marker = Path::new("marker");
        assert!(matches!(l::FileProject::entry(&environment, marker), l::ObservedEntryState::Missing));
        let mut store = open();
        store.remember_entries(&environment);
        let proof = store.environment_proof(environment.id.clone()).unwrap();
        let request = key("observed request");
        store.put(request, Packet::declined(&"from proved absence".into()), vec![proof], vec![], String::new);
        assert!(store.publish().unwrap() > 0);
        assert!(open().lookup(request).is_some());
        let mut reused = open();
        assert!(!reused.reuse_context(Some(&environment), None));
        assert!(reused.reuse_context(Some(&environment), Some(proof)));
        assert_eq!(reused.environment_proof(environment.id.clone()), Some(proof));
        fs::write(root.path().join(marker), b"now present").unwrap();
        let mut edited = open();
        assert!(!edited.reuse_context(Some(&environment), Some(proof)));
        assert!(edited.lookup(request).is_none());

        let unknown = project::Environment::new("unknown".into(), Authority::Given, BTreeSet::new(), epoch);
        assert!(matches!(l::FileProject::entry(&unknown, marker), l::ObservedEntryState::Unknown));
        let mut store = open();
        store.remember_entries(&unknown);
        assert!(store.environment_proof(unknown.id.clone()).is_none());
    }

    #[test]
    fn answers_replay_after_publish_with_each_requests_inputs() {
        let root = tempfile::tempdir().unwrap();
        let open = || Store::open(root.path(), &[] as &[Fake]).unwrap();
        let inventory = key("one closed inventory");
        let peers = key("same bound peers");
        let supplied = |body: &str| BTreeMap::from([("input".to_owned(), key(body))]);
        let scope = |store: &mut Store, body| {
            store.request_inputs(root.path(), inventory, peers, supplied(body))
        };
        let mut store = open();
        let program = key("bound program");
        let inputs = vec!["left".to_owned(), "right".to_owned()];
        store.remember_program_inputs(program, &inputs);
        for (request, body) in [("first", "old bytes"), ("second", "new bytes")] {
            let input = scope(&mut store, body);
            let evidence = store.group(
                vec![("input".into(), Probe::Effective), (PathBuf::new(), Probe::Inventory)],
                Some(input),
            ).unwrap();
            store.put(key(request), Packet::declined(&request.into()), vec![evidence], vec![], String::new);
        }
        assert!(store.publish().unwrap() > 0);

        let mut store = open();
        assert_eq!(store.program_inputs(program), Some(inputs.as_slice()));
        assert!(store.program_inputs(key("different program")).is_none());
        let new = scope(&mut store, "new bytes");
        let old = scope(&mut store, "old bytes");
        for (input, request) in [(new, "second"), (old, "first"), (new, "second")] {
            let (packet, _) = store.lookup_in(input, key(request)).unwrap();
            assert_eq!(store.packet(&packet).refusal, Some(request.into()));
        }
        assert!(store.lookup_in(new, key("first")).is_none());
        assert!(store.lookup_in(old, key("second")).is_none());
        assert!(store.lookup(key("first")).is_none());
        assert_eq!(store.publish().unwrap(), 0);

        let evidence = store.group(
            vec![("input".into(), Probe::Effective)],
            Some(old),
        ).unwrap();
        store.put(
            key("first"),
            Packet::declined(&"replacement".into()),
            vec![evidence],
            vec![("input".into(), "unreadable project input".into()); 2],
            String::new,
        );
        let shards = store.requests.index().clone();
        assert!(store.publish().unwrap() > 0);
        assert_eq!(store.requests.index(), &shards);
        assert_eq!(store.requests.layers.len(), 1);

        let mut store = open();
        let old = scope(&mut store, "old bytes");
        let new = scope(&mut store, "new bytes");
        for (input, key_name, text) in [(old, "first", "replacement"), (new, "second", "second")] {
            let (packet, unread) = store.lookup_in(input, key(key_name)).unwrap();
            if key_name == "first" {
                assert_eq!(unread, vec![("input".to_owned(), "unreadable project input".to_owned()); 2].as_slice());
            } else {
                assert!(unread.is_empty());
            }
            assert_eq!(store.packet(&packet).refusal, Some(text.into()));
        }
        assert!(store.lookup_in(new, key("first")).is_none());
    }
}
