//! A single action's completed file findings. All persisted identities are independent of run-local slots.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

type Digest = [u8; 32];
type Key = [u8; 16];
type Records = BTreeMap<Key, Answer>;
type Unknown = (String, Option<String>, String, UnknownOrigin);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct Finding {
    pub rule: String,
    pub file: String,
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum UnknownOrigin {
    Partition,
    Field(u32),
    Text(u32),
    Cell(u32, u32),
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Answer {
    admission_refusal: Option<String>,
    origin: String,
    findings: Vec<Finding>,
    refusals: Vec<(String, String)>,
    // A partial provider partition is never promoted to a complete file.
    unknowns: Vec<Unknown>,
}

#[derive(Default)]
pub(crate) struct Capture {
    active: Option<Answer>,
    arena: Vec<u8>,
    prior_refusal: bool,
    reusable: BTreeMap<String, Answer>,
    hits: usize,
    visited_bytes: usize,
    versions: Versions,
}

#[derive(Default)]
struct CapturedFile {
    version: Option<Digest>,
    packed: Option<std::ops::Range<usize>>,
}

#[derive(Default)]
struct Versions {
    names: std::sync::Arc<Vec<String>>,
    sorted: Vec<usize>,
    files: Vec<CapturedFile>,
}
impl Versions {
    fn get_mut(&mut self, name: &str) -> Option<&mut CapturedFile> {
        let i = self.sorted.binary_search_by(|&i| self.names[i].as_str().cmp(name)).ok()?;
        self.files.get_mut(self.sorted[i])
    }
}

fn pack_answer(arena: &mut Vec<u8>, answer: &Answer) -> std::ops::Range<usize> {
    // The admitted inventory already owns the origin. A finding on the same
    // file needs only its rule and span until the public table is written.
    let findings: Vec<_> = answer.findings.iter().map(|f| {
        (&f.rule, (f.file != answer.origin).then_some(&f.file), f.start, f.end)
    }).collect();
    let body = (&answer.admission_refusal, findings, &answer.refusals, &answer.unknowns);
    let offset = arena.len();
    *arena = postcard::to_extend(&body, std::mem::take(arena)).expect("file answer");
    offset..arena.len()
}

fn unpack_answer(origin: &str, bytes: &[u8]) -> Answer {
    #[derive(Deserialize)]
    struct Body {
        admission_refusal: Option<String>,
        findings: Vec<(String, Option<String>, u64, u64)>,
        refusals: Vec<(String, String)>,
        unknowns: Vec<Unknown>,
    }
    let body: Body = postcard::from_bytes(bytes).expect("captured file");
    Answer {
        origin: origin.to_owned(),
        admission_refusal: body.admission_refusal,
        findings: body.findings.into_iter().map(|(rule, file, start, end)| Finding {
            rule, file: file.unwrap_or_else(|| origin.to_owned()), start, end,
        }).collect(),
        refusals: body.refusals,
        unknowns: body.unknowns,
    }
}

impl Capture {
    fn finish_active(&mut self) {
        if let Some(mut answer) = self.active.take() {
            // Canonicalize these multisets once the file stops producing events.
            answer.findings.sort_unstable();
            answer.refusals.sort_unstable();
            answer.unknowns.sort_unstable();
            self.versions.get_mut(&answer.origin).expect("admitted origin").packed =
                Some(pack_answer(&mut self.arena, &answer));
        }
    }

    fn answer(&mut self, origin: &str) -> Option<&mut Answer> {
        if !self.active.as_ref().is_some_and(|a| a.origin == origin) {
            self.finish_active();
            let range = self.versions.get_mut(origin)?.packed.take()?;
            self.active = Some(unpack_answer(origin, &self.arena[range]));
        }
        self.active.as_mut()
    }

    #[allow(dead_code, reason = "only hosts that refuse a whole admission use this outcome")]
    pub(crate) fn refused_admission(&mut self, message: &str) {
        self.active = None;
        self.arena.clear();
        for (origin, file) in self.versions.names.iter().zip(&mut self.versions.files) {
            file.packed = Some(pack_answer(&mut self.arena, &Answer { origin: origin.clone(), admission_refusal: Some(message.into()), ..Default::default() }));
        }
    }
    pub(crate) fn visited(&mut self, origin: &str, bytes: &[u8]) {
        let Some(file) = self.versions.get_mut(origin) else {return};
        self.visited_bytes += bytes.len();
        let digest = *blake3::hash(bytes).as_bytes();
        if file.version.is_some_and(|expected| expected != digest) {
            self.partial(origin);
            return;
        }
        file.version = Some(digest);
        if self.answer(origin).is_none() {
            self.active = Some(Answer { origin: origin.into(), ..Answer::default() });
        }
    }
    pub(crate) fn finding(&mut self, origin: &str, finding: Finding) {
        if let Some(answer) = self.answer(origin) {
            answer.findings.push(finding);
        }
    }
    #[allow(dead_code, reason = "only SDK file readers publish packet unknowns")]
    pub(crate) fn unknown(
        &mut self,
        origin: &str,
        relation: &str,
        kind: Option<&str>,
        cause: &str,
        at: UnknownOrigin,
    ) {
        if let Some(answer) = self.answer(origin) {
            answer.unknowns.push((
                relation.to_owned(),
                kind.map(str::to_owned),
                cause.to_owned(),
                at,
            ));
        }
    }
    pub(crate) fn partial(&mut self, origin: &str) {
        if self.active.as_ref().is_some_and(|a| a.origin == origin) { self.active = None; }
        if let Some(file) = self.versions.get_mut(origin) { file.packed = None; }
    }
    pub(crate) fn refusal(&mut self, origin: &str, emit: &str, reason: &str) {
        if let Some(answer) = self.answer(origin) {
            answer.refusals.push((emit.into(), reason.into()));
        }
    }
    pub(crate) fn failed(&mut self, origin: &str) {
        self.partial(origin);
        self.prior_refusal = true;
    }
    #[allow(
        dead_code,
        reason = "only paired visitors inspect the next file before scheduling it"
    )]
    pub(crate) fn contains(&self, origin: &str) -> bool {
        !self.prior_refusal && self.reusable.contains_key(origin)
    }
    pub(crate) fn replay(&mut self, origin: &str, output: &mut Vec<Finding>, failures: &mut std::collections::HashMap<String, String>) -> bool {
        if self.prior_refusal {
            return false;
        }
        let Some(answer) = self.reusable.remove(origin) else {
            return false;
        };
        for finding in &answer.findings {
            if !failures.contains_key(&finding.rule) {
                output.push(finding.clone());
            }
        }
        for (emit, reason) in &answer.refusals {
            failures.entry(emit.clone()).or_insert_with(|| format!("{origin}: {reason}"));
        }
        self.finish_active();
        self.active = Some(answer);
        self.hits += 1;
        true
    }
}

#[derive(Deserialize)]
pub(crate) struct Request {
    pub root: PathBuf,
    #[serde(deserialize_with = "read_file_list")]
    pub files: std::sync::Arc<Vec<String>>,
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "some hosts bind selection in their own request reader"
    )]
    pub selected_rules: Vec<String>,
    #[serde(default)]
    project_search_roots: Vec<PathBuf>,
}

fn read_file_list<'de, D: serde::Deserializer<'de>>(input: D) -> Result<std::sync::Arc<Vec<String>>, D::Error> {
    Vec::<String>::deserialize(input).map(std::sync::Arc::new)
}

fn field(hash: &mut blake3::Hasher, b: &[u8]) {
    hash.update(&(b.len() as u64).to_le_bytes());
    hash.update(b);
}

fn local(p: &Path) -> bool {
    !p.is_absolute()
        && p.components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

fn invalid(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

// Process-local observations detect replacement while a check is reading its inputs.
// Across processes validity always depends on the content digest.
type Stamp = (u64, u64, i64, i64, u64);
fn stamp(m: &fs::Metadata) -> Stamp {
    use std::os::unix::fs::MetadataExt;
    (m.dev(), m.ino(), m.ctime(), m.ctime_nsec(), m.size())
}
fn observed(obs: &[(PathBuf, Stamp)]) -> bool {
    obs.iter().all(|(p, s)| fs::metadata(p).is_ok_and(|m| stamp(&m) == *s))
}
fn remember(path: &Path, obs: &mut Vec<(PathBuf, Stamp)>) -> io::Result<()> {
    obs.push((path.to_owned(), stamp(&fs::metadata(path)?)));
    Ok(())
}

// Close a directory over the observations its consumer reads. Pack inputs also consume contents.
// A symlink crossing the scope or unsupported file kind has no reusable proof here.
fn tree(
    hash: &mut blake3::Hasher,
    root: &Path,
    dir: &Path,
    cache: &Path,
    contents: bool,
    obs: &mut Vec<(PathBuf, Stamp)>,
) -> io::Result<()> {
    // Searching this directory must succeed for its directory entries to prove
    // the same file/dir predicates that the provider observes through path lookup.
    remember(&dir.join("."), obs)?;
    let mut paths = fs::read_dir(dir)?
        .map(|entry| {
            let entry = entry?;
            Ok((entry.path(), entry.file_type()?))
        })
        .collect::<io::Result<Vec<_>>>()?;
    paths.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    for (path, kind) in paths {
        if path == cache {
            continue;
        }
        field(
            hash,
            path.strip_prefix(root)
                .unwrap()
                .as_os_str()
                .as_encoded_bytes(),
        );
        if kind.is_file() {
            field(hash, &[1]);
            if contents {
                remember(&path, obs)?;
                field(hash, blake3::hash(&fs::read(&path)?).as_bytes());
            }
        } else if kind.is_dir() {
            field(hash, &[2]);
            tree(hash, root, &path, cache, contents, obs)?;
        } else if kind.is_symlink() {
            field(hash, &[3]);
            let target = fs::read_link(&path)?;
            field(hash, target.as_os_str().as_encoded_bytes());
            match fs::canonicalize(&path) {
                Ok(resolved) if resolved.starts_with(root) && !resolved.starts_with(cache) => {
                    field(hash, &[1]);
                    field(
                        hash,
                        resolved
                            .strip_prefix(root)
                            .unwrap()
                            .as_os_str()
                            .as_encoded_bytes(),
                    );
                }
                Err(e)
                    if e.kind() == io::ErrorKind::NotFound
                        && local(&target)
                        && !path.parent().unwrap().join(&target).starts_with(cache) =>
                {
                    // The complete directory walk proves absence inside the declared scope.
                    field(hash, &[0]);
                }
                _ => return Err(invalid("link leaves the declared source scope")),
            }
        } else {
            return Err(invalid("scope includes a special entry"));
        }
    }
    Ok(())
}

/// Match the lowerer's external inputs: every rule name and optional query, all module texts,
/// and the provider's pack inputs. Case fixtures and documentation do not enter the evaluator.
fn pack_inputs(
    hash: &mut blake3::Hasher,
    pack: &Path,
    cache: &Path,
    obs: &mut Vec<(PathBuf, Stamp)>,
) -> io::Result<()> {
    remember(pack, obs)?;
    for (folder, rules) in [("rules", true), ("fragments", false)] {
        let dir = pack.join(folder);
        if !rules && !dir.try_exists()? {
            continue;
        }
        remember(&dir, obs)?;
        let mut entries = fs::read_dir(&dir)?
            .map(|e| e.map(|e| e.path()))
            .collect::<io::Result<Vec<_>>>()?;
        entries.sort_unstable();
        for entry in entries {
            if !rules && entry.extension() != Some(std::ffi::OsStr::new("ryoql")) {
                continue;
            }
            let path = if rules {
                remember(&entry, obs)?;
                entry.join("query.ryoql")
            } else {
                entry
            };
            field(
                hash,
                path.strip_prefix(pack)
                    .unwrap()
                    .as_os_str()
                    .as_encoded_bytes(),
            );
            if path.try_exists()? {
                remember(&path, obs)?;
                field(hash, blake3::hash(&fs::read(&path)?).as_bytes());
            } else {
                field(hash, &[]);
            }
        }
    }
    let input = pack.join("inputs");
    if input.try_exists()? {
        tree(hash, pack, &input, cache, true, obs)?
    }
    Ok(())
}

struct Inputs {
    request: Request,
    keys: Vec<Key>,
    scope: Digest,
    modes: Vec<u32>,
    versions: Versions,
    observations: Vec<(PathBuf, Stamp)>,
    source_observations: Vec<Stamp>,
    seed_proof: Option<Digest>,
    phase_ms: [f64; 3],
}

// A first publication rechecks every admitted input. Keep the ordered digest
// across evaluation and release the path/stamp buffers while the visitor is live.
fn seed_proof(scope: &Digest, observations: &[(PathBuf, Stamp)], sources: &[Stamp]) -> Digest {
    let mut h = blake3::Hasher::new();
    h.update(scope);
    fn add(h: &mut blake3::Hasher, (dev, ino, sec, nsec, size): Stamp) {
        for value in [dev, ino, sec as u64, nsec as u64, size] { h.update(&value.to_le_bytes()); }
    }
    h.update(&(observations.len() as u64).to_le_bytes());
    for (path, st) in observations {
        field(&mut h, path.as_os_str().as_encoded_bytes());
        add(&mut h, *st);
    }
    for st in sources { add(&mut h, *st); }
    *h.finalize().as_bytes()
}

fn inputs(
    pack: &Path,
    request: &Path,
    cache_name: &str,
    project_entries: bool,
) -> io::Result<Inputs> {
    let start = cpu_time::ThreadTime::now();
    let ms = || start.elapsed().as_secs_f64() * 1000.;
    let mut observations = Vec::new();
    remember(request, &mut observations)?;
    let raw = fs::read(request)?;
    let req: Request = serde_json::from_slice(&raw)?;
    let cache = req.root.join(cache_name);
    fs::create_dir_all(&cache)?;
    let mut h = blake3::Hasher::new();
    // The request order selects the first visible refusal. Each stored answer is
    // local to its origin, so bind membership and apply ordering at replay.
    let external: BTreeMap<String, &serde_json::value::RawValue> = serde_json::from_slice(&raw)?;
    for (name, value) in external {
        if name != "files" {
            field(&mut h, name.as_bytes());
            field(&mut h, value.get().as_bytes());
        }
    }
    let mut sorted: Vec<_> = (0..req.files.len()).collect();
    sorted.sort_unstable_by(|&a, &b| req.files[a].cmp(&req.files[b]));
    if sorted.windows(2).any(|w| req.files[w[0]] == req.files[w[1]]) {
        return Err(invalid("repeated input"));
    }
    for &i in &sorted { field(&mut h, req.files[i].as_bytes()); }
    field(
        &mut h,
        fs::canonicalize(pack)?.as_os_str().as_encoded_bytes(),
    );
    // Diagnostic flags select different executor and provider behavior in these harnesses.
    let mut settings: Vec<_> = std::env::vars_os()
        .filter(|(k, _)| k.as_encoded_bytes().starts_with(b"VEC_"))
        .collect();
    settings.sort_unstable();
    for (k, v) in settings {
        field(&mut h, k.as_encoded_bytes());
        field(&mut h, v.as_encoded_bytes());
    }
    pack_inputs(&mut h, pack, &cache, &mut observations)?;
    let pack_ms = ms();
    if project_entries {
        if !req.project_search_roots.iter().all(|p| local(p)) {
            return Err(invalid("open search root"));
        }
        // This host declares that its cross-file reads inspect path existence and kind.
        // Effective bytes and permissions of every admitted file are bound below.
        tree(
            &mut h,
            &req.root,
            &req.root,
            &cache,
            false,
            &mut observations,
        )?;
    }
    let scope_ms = ms() - pack_ms;
    let scope = *h.finalize().as_bytes();
    let seeding = !cache.join("findings.postcard").try_exists()?;
    let mut modes = Vec::with_capacity(req.files.len());
    let mut versions = Versions { names: req.files.clone(), sorted, files: Vec::with_capacity(req.files.len()) };
    let mut keys = Vec::with_capacity(req.files.len());
    let mut source_observations = Vec::with_capacity(req.files.len());
    for name in req.files.iter() {
        if !local(Path::new(name)) || Path::new(name).starts_with(cache_name) {
            return Err(invalid("file outside source scope"));
        }
        let p = req.root.join(name);
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&p)?;
        let mode = metadata.permissions().mode();
        modes.push(mode);
        let version = if seeding { None } else {
            let digest = *blake3::hash(&fs::read(&p)?).as_bytes();
            keys.push(file_key(&scope, name, mode, &digest));
            Some(digest)
        };
        versions.files.push(CapturedFile { version, packed: None });
        source_observations.push(stamp(&metadata));
    }
    let seed_proof = seeding.then(|| {
        let proof = seed_proof(&scope, &observations, &source_observations);
        observations = Vec::new();
        source_observations = Vec::new();
        proof
    });
    Ok(Inputs {
        seed_proof,
        request: req,
        keys,
        scope,
        modes,
        versions,
        observations,
        source_observations,
        phase_ms: [pack_ms, scope_ms, ms() - pack_ms - scope_ms],
    })
}

fn file_key(scope: &Digest, name: &str, mode: u32, version: &Digest) -> Key {
    let mut h = blake3::Hasher::new();
    h.update(scope);
    field(&mut h, name.as_bytes());
    h.update(&mode.to_le_bytes());
    h.update(version);
    h.finalize().as_bytes()[..16].try_into().unwrap()
}

fn build_identity() -> io::Result<Digest> {
    use object::Object;
    let exe = if cfg!(target_os = "linux") {
        PathBuf::from("/proc/self/exe")
    } else {
        std::env::current_exe()?
    };
    let reader = object::read::ReadCache::new(File::open(&exe)?);
    if let Ok(file) = object::File::parse(&reader)
        && let Ok(Some(id)) = file.build_id()
    {
        return Ok(*blake3::hash(id).as_bytes());
    }
    Ok(*blake3::hash(&fs::read(exe)?).as_bytes())
}

fn load(path: &Path, build: &Digest) -> Option<(Digest, Records)> {
    let bytes = fs::read(path).ok()?;
    let (checksum, data) = bytes.split_at_checked(32)?;
    if blake3::hash(data).as_bytes() != checksum {
        return None;
    }
    let (id, records): (Digest, Records) = postcard::from_bytes(data).ok()?;
    (id == *build).then(|| (checksum.try_into().unwrap(), records))
}

fn publish(
    dir: &Path,
    path: &Path,
    build: Digest,
    previous: Option<Digest>,
    records: &BTreeMap<Key, (usize, std::ops::Range<usize>)>,
    names: &[String],
    arena: &[u8],
) -> io::Result<usize> {
    let header = postcard::to_stdvec(&(build, records.len())).map_err(io::Error::other)?;
    let mut buffer = Vec::new();
    let mut body = |write: &mut dyn FnMut(&[u8]) -> io::Result<()>| -> io::Result<()> {
        write(&header)?;
        for (key, (i, range)) in records {
            let answer = unpack_answer(&names[*i], &arena[range.clone()]);
            buffer.clear();
            buffer = postcard::to_extend(&answer, std::mem::take(&mut buffer)).map_err(io::Error::other)?;
            write(key)?;
            write(&buffer)?;
        }
        Ok(())
    };
    let mut hash = blake3::Hasher::new();
    let mut written = 32;
    body(&mut |b| { hash.update(b); written += b.len(); Ok(()) })?;
    let digest = *hash.finalize().as_bytes();
    if Some(digest) == previous { return Ok(0); }
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    {
        let mut out = io::BufWriter::new(&mut tmp);
        out.write_all(&digest)?;
        body(&mut |b| out.write_all(b))?;
        out.flush()?;
    }
    tmp.as_file().sync_all()?;
    tmp.persist(path)?;
    File::open(dir)?.sync_all()?;
    Ok(written)
}

fn report(rows: &Records, keys: &[Key], out: &Path, dedup: bool) -> io::Result<bool> {
    if let Some(reason) = keys.first().and_then(|k|rows[k].admission_refusal.as_ref()) {
        eprintln!("{reason}");
        return Ok(false);
    }
    let mut failures = BTreeMap::new();
    let mut lines = Vec::new();
    for k in keys {
        for row in &rows[k].findings {
            if !failures.contains_key(&row.rule) {
                lines.push(serde_json::to_string(row)?);
            }
        }
        let answer = &rows[k];
        for (emit, reason) in &answer.refusals {
            failures.entry(emit.clone()).or_insert_with(|| format!("{}: {reason}", answer.origin));
        }
    }
    lines.sort_unstable();
    if dedup {
        lines.dedup();
    }
    let mut writer = io::BufWriter::new(File::create(out)?);
    if lines.is_empty() {
        writer.write_all(b"\n")?;
    }
    for line in lines {
        writeln!(writer, "{line}")?;
    }
    writer.flush()?;
    for (emit, reason) in &failures {
        println!("FAILED {emit}: {reason}");
    }
    Ok(failures.is_empty())
}

/// Invoke the normal evaluator on misses. The reader host may replay individual files only when
/// its file packet is the whole provider input. The project-reading host closes over its tree.
pub(crate) fn check(
    pack: &Path,
    request: &Path,
    output: &Path,
    cache_name: &str,
    project_entries: bool,
    dedup: bool,
    mut evaluate: impl FnMut(&Request, Option<&mut Capture>) -> bool,
) -> io::Result<bool> {
    let now = cpu_time::ThreadTime::now();
    let ms = || now.elapsed().as_secs_f64() * 1000.;
    let mut first = match inputs(pack, request, cache_name, project_entries) {
        Ok(input) => input,
        Err(reason) => {
            eprintln!("retention unavailable: {reason}");
            return Ok(evaluate(
                &serde_json::from_slice(&fs::read(request)?)?,
                None,
            ));
        }
    };
    let req = &first.request;
    let dir = req.root.join(cache_name);
    let path = dir.join("findings.postcard");
    let build = build_identity()?;
    let before = fs::metadata(&path).ok();
    let (previous, rows) = load(&path, &build)
        .map(|(d, r)| (Some(d), Some(r)))
        .unwrap_or_default();
    let validation = ms();
    let [pack_ms, scope_ms, files_ms] = first.phase_ms;
    if let Some(rows) = &rows
        && !first.keys.is_empty()
        && rows.len() == first.keys.len()
        && first
            .keys
            .iter()
            .zip(req.files.iter())
            .all(|(k, name)| rows.get(k).is_some_and(|r| r.origin == *name))
    {
        let checked = ms();
        let ok = report(rows, &first.keys, output, dedup)?;
        let rendered = ms();
        eprintln!(
            "retained_cpu_ms pack={pack_ms:.3} scope={scope_ms:.3} files={files_ms:.3} load={:.3} check={:.3} report={:.3}",
            validation - pack_ms - scope_ms - files_ms,
            checked - validation,
            rendered - checked
        );
        eprintln!(
            "retained hits={} rows={} written=0 validation={validation:?}",
            first.keys.len(),
            rows.len()
        );
        return Ok(ok);
    }
    let had_empty = rows.as_ref().is_some_and(Records::is_empty);
    let mut old = rows.unwrap_or_default();
    let mut capture = Capture {
        versions: std::mem::take(&mut first.versions),
        ..Capture::default()
    };
    let mut changed_sources = Vec::new();
    for (i, (path, obs)) in req.files.iter().zip(first.source_observations).enumerate() {
        if let Some(answer) = first.keys.get(i).and_then(|key| old.remove(key))
            && answer.origin == *path
        {
            capture.reusable.insert(path.clone(), answer);
        } else {
            // A prior immutable row already proves its key. Only freshly evaluated
            // bytes and the shared scope need another observation before publication.
            changed_sources.push((i, obs));
        }
    }
    let ok = evaluate(&first.request, Some(&mut capture));
    let evaluated = ms();
    capture.finish_active();
    let mut next = BTreeMap::new();
    {
        for (i, p) in req.files.iter().enumerate() {
            if let CapturedFile { version, packed: Some(answer) } = std::mem::take(&mut capture.versions.files[i]) {
                let k = if let Some(k) = first.keys.get(i) {
                    *k
                } else {
                    // A provider may refuse the whole admission before visiting a
                    // source. That answer still binds the contents of the file.
                    let version = match version {
                        Some(v) => v,
                        None => {
                            let Ok(bytes) = fs::read(req.root.join(p)) else {continue};
                            *blake3::hash(&bytes).as_bytes()
                        }
                    };
                    file_key(&first.scope, p, first.modes[i], &version)
                };
                next.insert(k, (i, answer));
            }
        }
    }
    let lock = File::open(&dir)?;
    lock.lock()?;
    // A concurrent writer has precedence; its complete snapshot remains available on the next call.
    use std::os::unix::fs::MetadataExt;
    let same_parent = before
        .as_ref()
        .map(|m| (m.dev(), m.ino(), m.ctime(), m.ctime_nsec()))
        == fs::metadata(&path)
            .ok()
            .as_ref()
            .map(|m| (m.dev(), m.ino(), m.ctime(), m.ctime_nsec()));
    eprintln!("retained capture sources={} visitor_bytes={} upfront_versions={} arena_bytes={}", next.len(), capture.visited_bytes, first.keys.len(), capture.arena.len());
    let mut written = 0;
    let publishing = same_parent && !(next.is_empty() && had_empty);
    let inputs_current = || match first.seed_proof {
        Some(proof) => inputs(pack, request, cache_name, project_entries)
            .is_ok_and(|rechecked| rechecked.seed_proof == Some(proof)),
        None => observed(&first.observations) && changed_sources.iter().all(|(i, prior)|
            fs::metadata(req.root.join(&req.files[*i])).is_ok_and(|m| stamp(&m) == *prior)),
    };
    if publishing && !inputs_current() {
        drop(lock);
        eprintln!("inputs changed during evaluation; evaluating current contents");
        return Ok(evaluate(
            &serde_json::from_slice(&fs::read(request)?)?,
            None,
        ));
    }
    let checked = ms();
    if publishing {
        written = publish(&dir, &path, build, previous, &next, &req.files, &capture.arena)?;
    }
    eprintln!(
        "retained_cpu_ms pack={pack_ms:.3} scope={scope_ms:.3} files={files_ms:.3} load={:.3} evaluate={:.3} revalidate={:.3} publish={:.3}",
        validation - pack_ms - scope_ms - files_ms,
        evaluated - validation,
        checked - evaluated,
        ms() - checked
    );
    eprintln!(
        "retained hits={} rows={} written={written} validation={validation:?}",
        capture.hits,
        next.len()
    );
    Ok(ok)
}
