//! A single action's completed file findings. All persisted identities are independent of run-local slots.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

type Digest = [u8; 32];
type Key = [u8; 16];
type Records = BTreeMap<Key, Answer>;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Finding {
    pub rule: String,
    pub file: String,
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) enum UnknownOrigin {
    Partition,
    Field(u32),
    Text(u32),
    Cell(u32, u32),
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Answer {
    origin: String,
    findings: Vec<Finding>,
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
    pub(crate) fn refused(&mut self, origin: &str) {
        self.partial(origin);
        self.prior_refusal = true;
    }
    pub(crate) fn contains(&self, origin: &str) -> bool {
        !self.prior_refusal && self.reusable.contains_key(origin)
    }
    pub(crate) fn replay(&mut self, origin: &str, output: &mut Vec<String>) -> bool {
        if self.prior_refusal {
            return false;
        }
        let Some(answer) = self.reusable.remove(origin) else {
            return false;
        };
        for finding in &answer.findings {
            output.push(serde_json::to_string(finding).expect("finding output"));
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
fn stamp(p: &Path) -> io::Result<Stamp> {
    use std::os::unix::fs::MetadataExt;
    let m = fs::metadata(p)?;
    Ok((m.dev(), m.ino(), m.ctime(), m.ctime_nsec(), m.size()))
}
fn observed(obs: &[(PathBuf, Stamp)]) -> bool {
    obs.iter().all(|(p, s)| stamp(p).is_ok_and(|v| v == *s))
}
fn remember(path: &Path, obs: &mut Vec<(PathBuf, Stamp)>) -> io::Result<()> {
    obs.push((path.to_owned(), stamp(path)?));
    Ok(())
}

// For providers with project reads, include both names and contents over the declared closed scope.
// A symlink crossing the scope or unsupported file kind has no reusable proof here.
fn tree(
    hash: &mut blake3::Hasher,
    root: &Path,
    dir: &Path,
    cache: &Path,
    obs: &mut Vec<(PathBuf, Stamp)>,
) -> io::Result<()> {
    remember(dir, obs)?;
    let mut paths = fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<io::Result<Vec<_>>>()?;
    paths.sort_unstable();
    for path in paths {
        if path == cache {
            continue;
        }
        let meta = fs::symlink_metadata(&path)?;
        field(
            hash,
            path.strip_prefix(root)
                .unwrap()
                .as_os_str()
                .as_encoded_bytes(),
        );
        use std::os::unix::fs::PermissionsExt;
        field(hash, &meta.permissions().mode().to_le_bytes());
        if meta.is_file() {
            remember(&path, obs)?;
            field(hash, blake3::hash(&fs::read(&path)?).as_bytes());
        } else if meta.is_dir() {
            tree(hash, root, &path, cache, obs)?;
        } else if meta.is_symlink() {
            let resolved = fs::canonicalize(&path)?;
            if !resolved.starts_with(root) || resolved.starts_with(cache) {
                return Err(invalid("link leaves the declared source scope"));
            }
            field(hash, fs::read_link(&path)?.as_os_str().as_encoded_bytes());
            field(
                hash,
                resolved
                    .strip_prefix(root)
                    .unwrap()
                    .as_os_str()
                    .as_encoded_bytes(),
            );
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
        tree(hash, pack, &input, &pack.join(".ryo_cache"), obs)?
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

fn inputs(pack: &Path, request: &Path, closed: bool) -> io::Result<Inputs> {
    let start = cpu_time::ThreadTime::now();
    let ms = || start.elapsed().as_secs_f64() * 1000.;
    let mut observations = Vec::new();
    remember(request, &mut observations)?;
    let raw = fs::read(request)?;
    let req: Request = serde_json::from_slice(&raw)?;
    let cache = req.root.join(".ryo_cache");
    fs::create_dir_all(&cache)?;
    let mut h = blake3::Hasher::new();
    field(&mut h, &raw);
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
    pack_inputs(&mut h, pack, &mut observations)?;
    let pack_ms = ms();
    if closed {
        if !req.project_search_roots.iter().all(|p| local(p)) {
            return Err(invalid("open search root"));
        }
        tree(&mut h, &req.root, &req.root, &cache, &mut observations)?;
    }
    let scope_ms = ms() - pack_ms;
    let mut versions = BTreeMap::new();
    let mut keys = Vec::with_capacity(req.files.len());
    for name in &req.files {
        if versions.contains_key(name) {
            return Err(invalid("repeated input"));
        }
        if !local(Path::new(name)) || Path::new(name).starts_with(".ryo_cache") {
            return Err(invalid("file outside source scope"));
        }
        let p = req.root.join(name);
        let mut file = h.clone();
        field(&mut file, name.as_bytes());
        use std::os::unix::fs::PermissionsExt;
        field(
            &mut file,
            &fs::metadata(&p)?.permissions().mode().to_le_bytes(),
        );
        remember(&p, &mut observations)?;
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

fn report(rows: &Records, keys: &[Key], out: &Path, dedup: bool) -> io::Result<()> {
    let mut lines = Vec::new();
    for k in keys {
        for row in &rows[k].findings {
            lines.push(serde_json::to_string(row)?);
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
    writer.flush()
}

/// Invoke the normal evaluator on misses. The reader host may replay individual files only when
/// its file packet is the whole provider input. The project-reading host closes over its tree.
pub(crate) fn check(
    pack: &Path,
    request: &Path,
    output: &Path,
    closed: bool,
    dedup: bool,
    mut evaluate: impl FnMut(&Request, Option<&mut Capture>) -> bool,
) -> io::Result<bool> {
    let now = cpu_time::ThreadTime::now();
    let ms = || now.elapsed().as_secs_f64() * 1000.;
    let mut first = match inputs(pack, request, closed) {
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
    let dir = req.root.join(".ryo_cache");
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
        if !observed(&first.observations) {
            return Ok(evaluate(
                &serde_json::from_slice(&fs::read(request)?)?,
                None,
            ));
        }
        let checked = ms();
        report(rows, &first.keys, output, dedup)?;
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
        return Ok(true);
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
            if let Some(a) = capture.files.remove(p) {
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
    if !observed(&first.observations)
        || !inputs(pack, request, closed).is_ok_and(|after| after.keys == first.keys)
    {
        drop(lock);
        eprintln!("inputs changed during evaluation; evaluating current contents");
        return Ok(evaluate(
            &serde_json::from_slice(&fs::read(request)?)?,
            None,
        ));
    }
    let checked = ms();
    if same_parent && !(next.is_empty() && had_empty) {
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
