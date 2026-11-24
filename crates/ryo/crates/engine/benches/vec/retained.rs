//! A single action's completed file findings. All persisted identities are independent of run-local slots.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

type Digest = [u8; 32];
type Key = [u8; 16];
type Records = BTreeMap<Key, Answer>;

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
    unknowns: Vec<(String, Option<String>, String, UnknownOrigin)>,
}

#[derive(Default)]
pub(crate) struct Capture {
    files: BTreeMap<String, Answer>,
    prior_refusal: bool,
    reusable: BTreeMap<String, Answer>,
    hits: usize,
    versions: BTreeMap<String, Digest>,
}

impl Capture {
    #[allow(dead_code, reason = "only hosts that refuse a whole admission use this outcome")]
    pub(crate) fn refused_admission(&mut self, message: &str) {
        self.files = self.versions.keys().map(|origin| {
            let answer = Answer { origin: origin.clone(), admission_refusal: Some(message.into()), ..Default::default() };
            (origin.clone(), answer)
        }).collect();
    }
    pub(crate) fn visited(&mut self, origin: &str, bytes: &[u8]) {
        if !self
            .versions
            .get(origin)
            .is_some_and(|d| d == blake3::hash(bytes).as_bytes())
        {
            self.partial(origin);
            return;
        }
        self.files.entry(origin.into()).or_insert_with(|| Answer {
            origin: origin.into(),
            ..Answer::default()
        });
    }
    pub(crate) fn finding(&mut self, origin: &str, finding: Finding) {
        if let Some(answer) = self.files.get_mut(origin) {
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
        if let Some(answer) = self.files.get_mut(origin) {
            answer.unknowns.push((
                relation.to_owned(),
                kind.map(str::to_owned),
                cause.to_owned(),
                at,
            ));
        }
    }
    pub(crate) fn partial(&mut self, origin: &str) {
        self.files.remove(origin);
    }
    pub(crate) fn refusal(&mut self, origin: &str, emit: &str, reason: &str) {
        if let Some(answer) = self.files.get_mut(origin) {
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
    pub(crate) fn replay(&mut self, origin: &str, output: &mut Vec<String>, failures: &mut std::collections::HashMap<String, String>) -> bool {
        if self.prior_refusal {
            return false;
        }
        let Some(answer) = self.reusable.remove(origin) else {
            return false;
        };
        for finding in &answer.findings {
            if !failures.contains_key(&finding.rule) {
                output.push(serde_json::to_string(finding).expect("finding output"));
            }
        }
        for (emit, reason) in &answer.refusals {
            failures.entry(emit.clone()).or_insert_with(|| format!("{origin}: {reason}"));
        }
        self.files.insert(origin.to_owned(), answer);
        self.hits += 1;
        true
    }
}

#[derive(Deserialize)]
pub(crate) struct Request {
    pub root: PathBuf,
    pub files: Vec<String>,
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "some hosts bind selection in their own request reader"
    )]
    pub selected_rules: Vec<String>,
    #[serde(default)]
    project_search_roots: Vec<PathBuf>,
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
    versions: BTreeMap<String, Digest>,
    observations: Vec<(PathBuf, Stamp)>,
    phase_ms: [f64; 3],
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
    let mut inventory: Vec<_> = req.files.iter().collect();
    inventory.sort_unstable();
    for path in inventory {
        field(&mut h, path.as_bytes());
    }
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
    let mut versions = BTreeMap::new();
    let mut keys = Vec::with_capacity(req.files.len());
    for name in &req.files {
        if versions.contains_key(name) {
            return Err(invalid("repeated input"));
        }
        if !local(Path::new(name)) || Path::new(name).starts_with(cache_name) {
            return Err(invalid("file outside source scope"));
        }
        let p = req.root.join(name);
        let mut file = h.clone();
        field(&mut file, name.as_bytes());
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&p)?;
        field(&mut file, &metadata.permissions().mode().to_le_bytes());
        observations.push((p.clone(), stamp(&metadata)));
        let version = *blake3::hash(&fs::read(p)?).as_bytes();
        field(&mut file, &version);
        versions.insert(name.to_owned(), version);
        keys.push(file.finalize().as_bytes()[..16].try_into().unwrap());
    }
    Ok(Inputs {
        request: req,
        keys,
        versions,
        observations,
        phase_ms: [pack_ms, scope_ms, ms() - pack_ms - scope_ms],
    })
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
    records: &Records,
) -> io::Result<usize> {
    let data = postcard::to_stdvec(&(build, records)).map_err(io::Error::other)?;
    let digest = *blake3::hash(&data).as_bytes();
    if Some(digest) == previous {
        return Ok(0);
    }
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(&digest)?;
    tmp.write_all(&data)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path)?;
    File::open(dir)?.sync_all()?;
    Ok(data.len() + 32)
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
            .zip(&req.files)
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
    for (key, path) in first.keys.iter().zip(&req.files) {
        if let Some(answer) = old.remove(key)
            && answer.origin == *path
        {
            capture.reusable.insert(path.clone(), answer);
        }
    }
    let ok = evaluate(&first.request, Some(&mut capture));
    let evaluated = ms();
    let mut next = Records::new();
    {
        for (p, k) in req.files.iter().zip(&first.keys) {
            if let Some(mut a) = capture.files.remove(p) {
                // A relation's evaluation order may follow run-local symbol numbers.
                // These vectors are multisets. Persist a stable order with duplicates intact.
                a.findings.sort_unstable();
                a.refusals.sort_unstable();
                a.unknowns.sort_unstable();
                next.insert(*k, a);
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
    let mut written = 0;
    let publishing = same_parent && !(next.is_empty() && had_empty);
    if publishing && !observed(&first.observations) {
        drop(lock);
        eprintln!("inputs changed during evaluation; evaluating current contents");
        return Ok(evaluate(
            &serde_json::from_slice(&fs::read(request)?)?,
            None,
        ));
    }
    let checked = ms();
    if publishing {
        written = publish(&dir, &path, build, previous, &next)?;
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
