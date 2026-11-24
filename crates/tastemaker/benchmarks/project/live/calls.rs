//! Typed calls from the pinned check. A refusal returns to the host as an error.
use super::super::HostCell;
use super::{Active, Authority, Batch, Hosted, Input};
use crate::thread_cpu;
#[path = "pinned.rs"]
mod pinned;
#[path = "call_cache.rs"]
mod call_cache;
#[path = "allocated.rs"]
mod allocated;
use call_cache::{CallCache, PathFilter};
use pinned::{Dictionary, Pinned};
use release_sdk::linter as sdk;
use ryo_values::domain::finish_plans::expressions::types::{
    ConstantEntry, ConstantEntryKey, PlanConstant, PlanNumber,
};
use ryo_values::domain::plan_parts::types::{LiteralText, WholeNumber};
use ryo_values::domain::row_texts::types::Cell;
use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::{Arc, atomic::{AtomicU64, Ordering}},
};

type Reply = Result<sdk::LintReport, sdk::LintError>;

thread_local! {
    static SESSIONS: std::cell::RefCell<HashMap<usize, Live>> = Default::default();
}

pub fn link_executor() {
    release_sdk::linked_sdk::link(execute, |id| {
        measure_worker(|| SESSIONS.with(|sessions| drop(sessions.borrow_mut().remove(&id))));
    });
}

fn execution(reason: String) -> sdk::LintError {
    sdk::LintError::Execution { reason }
}

fn execute(
    id: usize,
    mut requests: Vec<sdk::LintRequest>,
    languages: &sdk::Languages,
    cache: &sdk::Cache,
) -> Vec<Reply> {
    let keys = crate::proof_enabled().then(|| {
        requests.iter_mut().map(|request| {
            // Different runs allocate different names for the same in-memory fixture.
            let root = matches!(request.corpus, sdk::Corpus::Given(_))
                .then(|| std::mem::take(&mut request.root));
            let key = digest(request);
            if let Some(root) = root {
                request.root = root;
            }
            key
        }).collect::<Vec<_>>()
    });
    let answers = measure_worker(|| SESSIONS.with(|sessions| {
        let mut sessions = sessions.borrow_mut();
        let session = sessions.entry(id).or_default();
        let directory = match cache {
            sdk::Cache::At(path) => Some(path.as_path()),
            _ => None,
        };
        if let Err(reason) = session.use_cache(directory) {
            return requests.into_iter().map(|_| Err(execution(reason.clone()))).collect();
        }
        session.lint(requests, languages)
    }));
    if let Some(keys) = keys {
        for (key, answer) in keys.into_iter().zip(&answers) {
            eprintln!("TM_ROUND_DIGEST\t{key:x}\t{:x}", digest(answer));
        }
    }
    answers
}

fn digest(value: &impl std::fmt::Debug) -> sha2::digest::Output<sha2::Sha256> {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;

    struct Stream(Sha256);
    impl Write for Stream {
        fn write_str(&mut self, text: &str) -> std::fmt::Result {
            self.0.update(text.as_bytes());
            Ok(())
        }
    }

    let mut stream = Stream(Sha256::new());
    write!(stream, "{value:?}").expect("the digest writer accepts all text");
    stream.0.finalize()
}

type WorkEntry = (
    sdk::RuleId,
    Result<super::stored::Key, (usize, Option<super::stored::Key>)>,
);

struct Pending {
    ids: Vec<WorkEntry>,
    paths: Rc<Vec<String>>,
    projects: std::collections::BTreeSet<usize>,
    restored_unread: BTreeMap<String, usize>,
}

struct Live {
    batch: Batch,
    store:Option<super::stored::Store>,
    directory:Option<std::path::PathBuf>,
    state: Active<pinned::File, Dictionary>,
    languages: Vec<Pinned>,
    filters: HashMap<Vec<sdk::Glob>, (usize, Rc<PathFilter>)>,
    cache: CallCache,
    versions: HashMap<(Authority, BTreeMap<String, Arc<[u8]>>), Arc<str>>,
}

impl Drop for Live {
    fn drop(&mut self){
        if let Some(store)=&mut self.store && let Err(e)=store.publish(){eprintln!("publish completed session: {e}");}
    }
}
impl Default for Live {
    fn default() -> Self {
        MEASURED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .sessions += 1;
        let mut state: Active<pinned::File, Dictionary> = Active {
            keep: true,
            ..Default::default()
        };
        state.retention.configure(Some(retention_limit()), allocated::bytes);
        Self {
            batch: Batch::default(),
            store:None, directory:None,
            state,
            languages: Vec::new(),
            filters: HashMap::new(),
            cache: CallCache::default(),
            versions: HashMap::new(),
        }
    }
}

// A test override for the session's own projection retention. The public SDK
// does not take a budget and the host's memory setting does not reach here.
fn retention_limit() -> u64 {
    const RETAINED_FILE_BYTES: u64 = 1024 * 1024 * 1024;
    static LIMIT: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *LIMIT.get_or_init(|| {
        std::env::var_os("TM_BENCH_MEMORY_BYTES").map_or(RETAINED_FILE_BYTES, |value| {
            value.to_str().and_then(|s| s.parse::<std::num::NonZeroU64>().ok())
                .expect("TM_BENCH_MEMORY_BYTES must be a positive byte count").get()
        })
    })
}

fn rule_error(
    id: sdk::RuleId,
    query: release_sdk::query::RyoQlQuery,
    linked: &sdk::Languages,
    hosted: &[Hosted],
    reason: String,
) -> sdk::LintError {
    use release_sdk::query::{ValidateError, ValidateRequest, validate};

    let hosted = hosted.iter().map(|table| sdk::HostDeclaration {
        name: sdk::HostRelationName(table.name.as_str().into()),
        columns: table.columns.iter().map(|(name, kind)| sdk::HostColumn {
            name: sdk::HostColumnName(name.as_str().into()),
            kind: match kind.as_str() {
                "File" => sdk::HostKind::File,
                "Text" => sdk::HostKind::Text,
                "Number" => sdk::HostKind::Number,
                _ => unreachable!("host table admission only accepts these column kinds"),
            },
        }).collect(),
    }).collect();
    match validate(linked, ValidateRequest { query, hosted, observations: Vec::new() }) {
        Ok(_) | Err(ValidateError::Catalog { .. }) => sdk::LintError::Unrunnable { id, reason },
        Err(error) => sdk::LintError::Rule { id, error },
    }
}

fn parameter(v: sdk::ParameterValue) -> Result<PlanConstant, String> {
    use sdk::ParameterValue::*;
    let number = |n| PlanConstant::Number(PlanNumber::Whole(WholeNumber(n)));
    let text = |s| PlanConstant::Text(LiteralText(s));
    let texts =
        |v: Vec<sdk::ParameterText>| PlanConstant::List(v.into_iter().map(|s| text(s.0)).collect());
    Ok(match v {
        Number(n) => number(n.0),
        Text(s) => text(s.0),
        Flag(f) => text((f == sdk::FlagState::On).to_string().into()),
        Numbers(ns) => PlanConstant::List(ns.0.into_iter().map(number).collect()),
        Texts(ss) => texts(ss),
        TextSets(sets) => PlanConstant::List(sets.into_iter().map(texts).collect()),
        Tables(rows) => PlanConstant::List(
            rows.into_iter()
                .map(|row| {
                    let mut fields: Vec<_> = row
                        .0
                        .into_iter()
                        .map(|c| ConstantEntry {
                            key: ConstantEntryKey(c.key.0),
                            value: text(c.value.0),
                        })
                        .collect();
                    fields.sort_by(|a, b| a.key.0.cmp(&b.key.0));
                    PlanConstant::Map(fields)
                })
                .collect(),
        ),
        _ => return Err("unsupported SDK parameter type".into()),
    })
}

fn slot(v: &Option<Cell<std::sync::Arc<str>>>) -> sdk::Slot {
    let Some(c) = v else {
        return sdk::Slot::Nothing;
    };
    use release_sdk::languages::{Offset, Span};
    sdk::Slot::Is(match c {
        Cell::File(p) => sdk::Cell::File(sdk::RelativePath(p.as_ref().into())),
        Cell::Text(s) => sdk::Cell::Text(sdk::HostText(s.as_ref().into())),
        Cell::Number(n) => sdk::Cell::Number(sdk::HostNumber(n.0)),
        Cell::Span(s) => sdk::Cell::Span(Span {
            start: Offset(s.start.0),
            end: Offset(s.end.0),
        }),
    })
}

impl Live {
    fn use_cache(&mut self,directory:Option<&std::path::Path>)->Result<(),String>{
        // A cache location is available on every real check. Retention is a caller mode.
        let directory = directory.filter(|_| {
            std::env::var_os("TM_CHECK_SESSION").as_deref() == Some(std::ffi::OsStr::new("retained"))
        });
        if self.directory.as_deref()!=directory {
            if let Some(old)=&mut self.store {old.publish().map_err(|e|e.to_string())?;}
            self.store=directory.map(|p|super::stored::Store::directory(p.to_owned())).transpose().map_err(|e|e.to_string())?;
            self.directory=directory.map(std::path::Path::to_owned);
        }
        Ok(())
    }
    fn globs(&mut self, names: &[sdk::Glob]) -> Result<(usize, Rc<PathFilter>), String> {
        if let Some(found) = self.filters.get(names) {
            return Ok(found.clone());
        }
        let matcher = PathFilter::compile(names)?;
        let set = (self.filters.len(), Rc::new(matcher));
        self.filters.insert(names.to_vec(), set.clone());
        Ok(set)
    }

    fn phase_totals(&self) -> [f64; 6] {
        let c = &self.batch.clocks;
        [c.prepare, c.read_parse, c.screen, c.evaluate, c.project, c.assemble]
    }

    fn lint(
        &mut self,
        requests: Vec<sdk::LintRequest>,
        linked: &sdk::Languages,
        ) -> Vec<Reply> {
        // The opt-in retained store's overlay is local to a request.
        if self.store.is_some() && requests.len() > 1 {
            return requests.into_iter()
                .flat_map(|request| self.lint(vec![request], linked)).collect();
        }
        let previous = self.phase_totals();
        if self.languages.is_empty() {
            self.languages = linked.0.iter().cloned().map(Pinned::from).collect();
        }
        let count = requests.iter().map(|request| request.rules.len()).sum::<usize>();
        let pending = requests.into_iter().map(|request| {
            let start = self.batch.work.len();
            let result = self.admit(request, linked);
            if result.is_err() {
                self.batch.work.truncate(start);
            }
            result
        }).collect::<Vec<_>>();
        if !self.batch.work.is_empty() {
            if let Some(store) = &mut self.store {
                self.batch.epoch_with_store(&self.languages, &mut self.state, Some(store));
            } else {
                self.batch.epoch(&self.languages, &mut self.state);
            }
        }
        let results = pending.into_iter().map(|request| {
            request.and_then(|request| self.deliver(request, linked))
        }).collect::<Vec<_>>();
        self.batch.work.clear();
        let mut last = MEASURED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let c = &self.batch.clocks;
        last.calls += results.len();
        last.rounds.push(results.len());
        last.empty_reader_epochs = self.batch.empty_reader_epochs;
        last.bulk_screen_files = c.bulk_screen_files;
        last.requests += count;
        for reason in results.iter().filter_map(|result| result.as_ref().err()) {
            last.refused += 1;
            *last.reasons.entry(reason.to_string()).or_default() += 1;
        }
        for (total, (before, after)) in last.parts.iter_mut().zip(previous.into_iter().zip(self.phase_totals())) {
            *total += after - before;
        }
        last.retained = self.state.retention.counts();
        last.project_counts = [
            self.batch.projects.len(),
            self.batch.populations_built,
            self.batch.relations_built,
            self.batch.relations_rebuilt,
        ];
        last.counts = [
            self.batch.schedule.programs.len(),
            self.batch.bindings.len(),
            self.batch.schedule.sources.len(),
            c.reads,
            c.files,
            c.unique_pairs,
            c.skipped,
            c.evaluated,
        ];
        results
    }

    fn admit(&mut self, req: sdk::LintRequest, linked: &sdk::Languages) -> Result<Pending, sdk::LintError> {
        if req.source_language != sdk::SourceLanguage::Recognized
            || req.project_search_scope != sdk::ProjectSearchScope::Undeclared
            || req.given_project_inventory != Default::default()
        {
            return Err(execution("unsupported source or project authority".into()));
        }
        let physical: Arc<str> = Arc::from(req.root.to_str().ok_or_else(|| sdk::LintError::PathNotText { path: req.root.clone() })?);
        let (authority, listed, given) = match req.corpus {
            sdk::Corpus::Given(files) => (Authority::Given, Vec::new(), files),
            sdk::Corpus::Listed(paths) => {
                (Authority::Disk(physical.to_string()), paths, Vec::new())
            }
            _ => return Err(execution("a closed source inventory is required".into())),
        };
        let mut overlay = BTreeMap::new();
        for source in given.into_iter().chain(req.beside) {
            if source.executable != sdk::ExecutablePermission::Unknown {
                return Err(execution("unhandled supplied executable permission".into()));
            }
            overlay.insert(
                source.path.0.into_string(),
                Arc::<[u8]>::from(source.text.0.as_bytes()),
            );
        }
        let (inventory, paths) = self.cache.inventory(listed.iter().map(|p| p.0.as_ref())
            .chain(overlay.keys().map(String::as_str)).collect());
        let key = (authority.clone(), overlay);
        let mut prepared_root=None;
        let filter = self.globs(&req.files).map_err(execution)?;
        for (index, relation) in req.hosted.iter().enumerate() {
            if req.hosted[..index].iter().any(|earlier| earlier.name == relation.name) {
                return Err(sdk::LintError::DuplicateHostRelation { name: relation.name.clone() });
            }
        }
        let hosted = req.hosted.into_iter().map(|t| self.cache.table(t))
            .collect::<Result<Vec<_>, _>>()?;
        let mut ids = Vec::new();
        let scope=if let Some(s)=&mut self.store{
            // The unread list belongs to the union of requested project relations.
            // File-only requests have no dependency on this peer inventory.
            let peers=req.rules.iter().map(|(id,rule)|{
                let parameters=rule.parameters.iter().map(|(k,v)|Ok((&k.0,parameter(v.clone())?))).collect::<Result<Vec<_>,String>>()?;
                Ok(super::stored::key((id.as_ref(),rule.query.as_ref(),&rule.include,&rule.exclude,parameters)))
            }).collect::<Result<Vec<_>,String>>().map_err(execution)?;
            let shape=hosted.iter().map(|h|(&h.name,&h.columns)).collect::<Vec<_>>();
            s.overlay(std::path::Path::new(&*physical),&paths,super::stored::key((peers,shape)),&key.1);
            Some(super::stored::key((&physical,&authority)))
        }else{None};
        let host_keys=self.store.as_ref().map(|_|hosted.iter().map(super::stored::key).collect::<Vec<_>>());
        let mut restored_unread=BTreeMap::<String,usize>::new();
        let mut projects = std::collections::BTreeSet::new();
        for (id, rule) in req.rules {
            if rule.applicability != sdk::RuleApplicability::Always || !rule.observations.is_empty()
            {
                return Err(sdk::LintError::Unrunnable { id, reason: "unsupported observation or syntax requirement".into() });
            }
            let inc = self.globs(&rule.include).map_err(execution)?;
            let exc = self.globs(&rule.exclude).map_err(execution)?;
            let files = self.cache.files(
                (inventory, filter.0, inc.0, exc.0),
                &filter.1,
                &inc.1,
                &exc.1,
            );
            let parameters = rule
                .parameters
                .into_iter()
                .map(|(k, v)| Ok((k.0, parameter(v)?)))
                .collect::<Result<_, String>>().map_err(execution)?;
            let query=self.cache.query(rule.query.as_ref());
            let contract=scope.map(|scope|{
                let rule=self.batch.prepared_rule(query.clone(),&parameters);
                let plan=self.batch.schedule.programs[rule].as_ref().ok();
                let consumed:Vec<_>=hosted.iter().enumerate().filter(|(_,t)|plan.is_some_and(|p|p.bases.contains(&t.name)))
                    .map(|(i,_)|host_keys.as_ref().unwrap()[i]).collect();
                super::stored::key((scope,&query,&parameters,&files,consumed))
            });
            if let (Some(store),Some(k))=(&mut self.store,contract)
                && let Some((packet,unread))=store.lookup(k){
                if let Some(reason)=&store.packet(&packet).refusal{return Err(rule_error(id, rule.query, linked, &hosted, reason.clone()))}
                let mut counts=BTreeMap::<String,usize>::new();
                for p in unread{*counts.entry(p).or_default()+=1;}
                for (p,n) in counts{let total=restored_unread.entry(p).or_default();*total=(*total).max(n);}
                ids.push((id,Ok(packet)));
                continue
            }
            if prepared_root.is_none(){
        let root = if let Some(id) = self.versions.get(&key) {
            id.clone()
        } else {
            let id: Arc<str> = Arc::from(format!("check-input:{}", self.versions.len()));
            for (p, v) in &key.1 {
                self.batch.schedule.provide(&id, p.clone(), v.clone()).map_err(execution)?;
            }
            self.versions.insert(key.clone(), id.clone());
            id
        };
        for p in &listed {
            if !key.1.contains_key(p.0.as_ref()) {
                self.batch.schedule.listed(&root, &physical, &p.0).map_err(execution)?;
            }
        }
                prepared_root=Some(root);
            }
            let root=prepared_root.as_ref().unwrap();
            let work = self
                .batch
                .register_query(
                    query,
                    Input {
                        root: root.to_string(),
                        files,
                        inventory: Some(paths.clone()),
                        repository: Some(authority.clone()),
                        hosted: hosted.clone(),
                        parameters,
                    },
                    None,
                )
                .map_err(|reason| rule_error(id.clone(), rule.query.clone(), linked, &hosted, reason))?;
            if let super::Work::Invalid(reason)=&work{
                if let (Some(store),Some(k))=(&mut self.store,contract){store.decline(k,reason);}
                return Err(rule_error(id, rule.query, linked, &hosted, reason.clone()))
            }
            if let super::Work::Closed { project: Some(id), .. } = &work {
                projects.insert(*id);
            }
            ids.push((id,Err((self.batch.work.len(),contract))));
            self.batch.work.push(work);
        }
        Ok(Pending { ids, paths, projects, restored_unread })
    }

    fn deliver(&mut self, request: Pending, linked: &sdk::Languages) -> Result<sdk::LintReport, sdk::LintError> {
        let Pending { ids, paths, projects, restored_unread } = request;
        use release_sdk::linked_sdk::{finding, report, unknown, unread};
        let mut findings = Vec::new();
        let mut unknowns = Vec::new();
        let mut unread_files = Vec::new();
        let start = thread_cpu();
        for (id,entry) in ids {
            let held;
            let receipt=match entry {
                Ok(k)=>{
                    held=self.store.as_ref().unwrap().packet(&k).report();
                    super::Receipt {rows:held.rows.iter().collect(),unknowns:held.unknowns.iter().collect()}
                }
                Err((i,k))=>{
                    let r=Batch::answer(&self.batch.work[i],&self.state.answers).map_err(|reason| sdk::LintError::Unrunnable { id: id.clone(), reason: reason.to_owned() })?;
                    if let (Some(store),Some(k))=(&mut self.store,k){store.save(k,i,&r);}
                    r
                }
            };
            for row in receipt.rows {
                let (&subject, keys) = row.layout.identity.split_first().ok_or_else(|| execution("missing row subject".into()))?;
                let key = sdk::FindingKey(keys.iter().map(|&index| slot(&row.value(index))).collect());
                let columns = row.layout.names.iter().zip(row.cells())
                    .map(|(name, value)| sdk::FindingColumn {
                        name: sdk::ColumnName(name.as_str().into()),
                        value: slot(&value),
                    })
                    .collect();
                // The packed row retains emitted cells but carries no SDK source location.
                findings.push(finding(id.clone(), slot(&row.value(subject)), key, sdk::Place::Missing, columns));
            }
            for (path, message) in receipt.unknowns {
                unknowns.push(unknown(
                    id.clone(), sdk::RelativePath(path.as_str().into()), sdk::Message(message.as_str().into()),
                ));
            }
        }
        for path in Rc::unwrap_or_clone(paths) {
            let p = sdk::RelativePath(path.into());
            if linked.reads(&p) != release_sdk::languages::Recognition::Reads
                && !linked.0.iter().any(|reader| reader.primary_role(release_sdk::languages::SourcePath(&p.0)) == release_language::domain::files::types::PrimaryFileRole::Configuration)
            {
                unread_files.push(unread(p, sdk::Unreadable::NoLanguage));
            }
        }
        let mut manifest = BTreeMap::<Box<str>, Vec<sdk::ReadFailure>>::new();
        for project in projects {
            if let Some((_, Ok(population))) = self.state.populations.get(&project) {
                for refused in &population.refused {
                    manifest.entry(refused.path.0.clone()).or_default()
                        .push(sdk::ReadFailure(refused.reason.0.clone()));
                }
            }
        }
        for (path, count) in restored_unread {
            let reasons = manifest.entry(path.into()).or_default();
            if reasons.len() < count {
                reasons.resize_with(count, || sdk::ReadFailure("retained manifest was unreadable".into()));
            }
        }
        for (path, mut reasons) in manifest {
            reasons.sort_unstable();
            unread_files.extend(reasons.into_iter().map(|reason| {
                unread(sdk::RelativePath(path.clone()), sdk::Unreadable::Failed(reason))
            }));
        }
        unread_files.sort_by(|left, right| left.path.cmp(&right.path));
        self.batch.clocks.assemble += thread_cpu() - start;
        Ok(report(findings, unknowns, unread_files))
    }
}

/// Replay a closed capture with exactly the parsers linked into this release.
pub fn capture(linked: sdk::Languages, manifest: &str, output: &str, workers: Option<std::num::NonZeroUsize>) {
    capture_then(linked, manifest, output, workers, || {});
}

pub fn capture_then(linked: sdk::Languages, manifest: &str, output: &str, workers: Option<std::num::NonZeroUsize>, completed: impl FnOnce()) {
    let readers: Vec<_> = linked.0.into_iter().map(Pinned::from).collect();
    super::batch_mode_with_workers(&readers, manifest, output, None, workers, completed);
}

pub fn capture_retained(linked:sdk::Languages,manifest:&str,output:&str,root:&str){
    let readers:Vec<_>=linked.0.into_iter().map(Pinned::from).collect();
    super::batch_mode(&readers,manifest,output,Some(std::path::Path::new(root)));
}

#[derive(Default)]
struct Measurement {
    sessions: usize,
    rounds: Vec<usize>,
    empty_reader_epochs: usize,
    bulk_screen_files: usize,
    project_counts: [usize; 4],
    calls: usize,
    requests: usize,
    refused: usize,
    reasons: BTreeMap<String, usize>,
    parts: [f64; 6],
    counts: [usize; 8],
    retained: [u64; 4],
}
static MEASURED: std::sync::Mutex<Measurement> = std::sync::Mutex::new(Measurement {
    sessions: 0,
    rounds: Vec::new(),
    empty_reader_epochs: 0,
    bulk_screen_files: 0,
    project_counts: [0; 4],
    calls: 0,
    requests: 0,
    refused: 0,
    reasons: BTreeMap::new(),
    parts: [0.0; 6],
    counts: [0; 8],
    retained: [0; 4],
});

thread_local! {
    static WORK_ON_THIS_THREAD: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
}

static WORK_NANOS: AtomicU64 = AtomicU64::new(0);
static DISPATCH_NANOS: AtomicU64 = AtomicU64::new(0);

fn measure_worker<T>(work: impl FnOnce() -> T) -> T {
    let start = thread_cpu();
    let result = work();
    let elapsed = thread_cpu() - start;
    WORK_ON_THIS_THREAD.set(WORK_ON_THIS_THREAD.get() + elapsed);
    WORK_NANOS.fetch_add((elapsed * 1e9) as u64, Ordering::Relaxed);
    result
}

/// Include scheduling on the caller, including fixture calls made on the same worker.
pub fn measure_dispatch<T>(lint: impl FnOnce() -> T) -> T {
    let prior = WORK_ON_THIS_THREAD.get();
    let start = thread_cpu();
    let result = lint();
    let overlapped = WORK_ON_THIS_THREAD.get() - prior;
    let dispatch = thread_cpu() - start - overlapped;
    DISPATCH_NANOS.fetch_add((dispatch * 1e9) as u64, Ordering::Relaxed);
    result
}

/// Print the one executor's cumulative counters to the benchmark log after the host renders.
pub fn print_measurement() {
    let m = MEASURED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (reason, count) in &m.reasons {
        eprintln!("UNANSWERED\t{count}\t{reason}");
    }
    if m.rounds.iter().any(|&size| size > 1) {
        eprintln!("TM_ROUND_COUNTS epochs={} max_width={} closed_only={} bulk_screen_files={}",
            m.rounds.len(), m.rounds.iter().max().unwrap(), m.empty_reader_epochs, m.bulk_screen_files);
    }
    let [versions,hits,evictions,peak_charge]=m.retained;
    eprintln!("FILE_CACHE versions={versions} hits={hits} evictions={evictions} peak_charge={peak_charge}");
    let [prepare, read, first, second, project, assembly] = m.parts;
    let total = (WORK_NANOS.load(Ordering::Relaxed) + DISPATCH_NANOS.load(Ordering::Relaxed)) as f64 / 1e9;
    let [plans, bindings, sources, reads, parses, pairs, skips, eval] = m.counts;
    let [inventories, builds, relations, repeated] = m.project_counts;
    eprintln!(
        "PROJECT_POPULATIONS sessions={} inventories={inventories} builds={builds} relations={relations} repeated_relations={repeated}",
        m.sessions,
    );
    eprintln!(
        "CHECK_SESSION live calls={} requests={} failed_calls={} programs={plans} bindings={bindings} sources={sources} reads={reads} parses={parses} distinct_pairs={pairs} skipped={skips} evaluated={eval} prep={prepare:.6} read_parse={read:.6} first={first:.6} second={second:.6} project={project:.6} assembly={assembly:.6} admission={:.6}",
        m.calls,
        m.requests,
        m.refused,
        total - m.parts.iter().sum::<f64>()
    );
}

/// Write one fixture's complete rows, through the same reader and executor as the check.
pub fn capture_one(
    linked: sdk::Languages,
    pack: &str,
    input: &str,
    out: &str,
) -> Result<(), String> {
    let readers: Vec<_> = linked.0.into_iter().map(Pinned::from).collect();
    let bytes = std::fs::read(input).map_err(|e| e.to_string())?;
    let req = crate::json_input::decode(&bytes).map_err(|e| e.to_string())?;
    super::one_source(&readers, pack, req, out)
}
