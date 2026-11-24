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
    sync::Arc,
    time::Instant,
};

#[derive(Default)]
pub struct LiveReport {
    pub findings: Vec<(Box<str>, sdk::Slot, Vec<(Box<str>, sdk::Slot)>)>,
    pub unknowns: Vec<(Box<str>, String, String)>,
    pub unread: Vec<Box<str>>,
}

/// Options supplied by the caller of the pinned SDK check.
#[derive(Clone, Copy, Default)]
pub struct LiveOptions {
    pub workers: Option<std::num::NonZeroUsize>,
    pub reverse_files: bool,
}

// The pinned CLI has one check invocation. The SDK receives these values
// explicitly even though its session resides on the host's scheduler thread.
static CHECK_OPTIONS: std::sync::OnceLock<LiveOptions> = std::sync::OnceLock::new();

pub fn configure_bench_check(workers: Option<std::num::NonZeroUsize>, reverse_files: bool) {
    let _ = CHECK_OPTIONS.set(LiveOptions { workers, reverse_files });
}

pub fn check_options() -> LiveOptions {
    CHECK_OPTIONS.get().copied().unwrap_or_default()
}

pub struct Live {
    batch: Batch,
    store:Option<super::stored::Store>,
    directory:Option<std::path::PathBuf>,
    state: Active<pinned::File, Dictionary>,
    languages: Vec<Pinned>,
    filters: HashMap<Vec<sdk::Glob>, (usize, Rc<PathFilter>)>,
    cache: CallCache,
    admission_wall: f64,
    admission_workers_cpu: f64,
    admission_shards: usize,
    delivery_wall: f64,
    file_epoch_wall: f64,
    parallel_rows: usize,
    delivery_dictionaries: usize,
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
        Self {
            batch: Batch::default(),
            admission_wall: 0.0,
            admission_workers_cpu: 0.0,
            admission_shards: 0,
            delivery_wall: 0.0,
            file_epoch_wall: 0.0,
            parallel_rows: 0,
            delivery_dictionaries: 0,
            store:None, directory:None,
            state: Active {
                keep: true,
                ..Default::default()
            },
            languages: Vec::new(),
            filters: HashMap::new(),
            cache: CallCache::default(),
            versions: HashMap::new(),
        }
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

fn parameters(
    input: BTreeMap<sdk::ParameterName, sdk::ParameterValue>,
) -> Result<super::Parameters, String> {
    input.into_iter().map(|(k, v)| Ok((k.0, parameter(v)?))).collect()
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
    pub fn use_cache(&mut self,directory:Option<&std::path::Path>)->Result<(),String>{
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

    pub fn lint(
        &mut self,
        req: sdk::LintRequest,
        linked: &sdk::Languages,
        memory: sdk::MemoryBudget,
        options: LiveOptions,
    ) -> Result<LiveReport, String> {
        let wall = Instant::now();
        let t = thread_cpu();
        let limit = match memory {
            sdk::MemoryBudget::Machine => None,
            sdk::MemoryBudget::Bytes(n) => Some(n.get()),
            _ => return Err("unsupported memory budget".into()),
        };
        self.state.retention.configure(limit, allocated::bytes);
        let count = req.rules.len();
        self.batch.workers = options.workers;
        let result = self.answer(req, linked, options.reverse_files);
        let mut last = MEASURED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let c = &self.batch.clocks;
        last.calls += 1;
        last.requests += count;
        last.refused += usize::from(result.is_err());
        if let Err(reason) = &result {
            *last.reasons.entry(reason.clone()).or_default() += 1;
        }
        last.elapsed += thread_cpu() - t;
        last.worker_cpu = c.delivery_workers_cpu + c.file_workers_cpu + c.project_workers_cpu + self.admission_workers_cpu;
        last.file_shards = c.file_shards;
        last.dispatch_wall = [c.inventory_wall, c.join_wall, c.shard_critical_wall, c.shard_busy_wall, c.largest_file_wall];
        if c.largest_file_wall > 0. {
            last.largest_file.clone_from(&c.largest_file);
        }
        last.parallel_rows = self.parallel_rows;
        last.delivery_dictionaries = self.delivery_dictionaries;
        last.empty_file_epochs = c.empty_file_epochs;
        last.omitted_file_bindings = c.omitted_file_bindings;
        last.project_shards = c.project_shards;
        last.admission_shards = self.admission_shards;
        last.project_prepare_wall = c.project_prepare_wall;
        last.call_wall += wall.elapsed().as_secs_f64();
        last.phase_wall = [
            self.admission_wall,
            self.file_epoch_wall - c.project_wall,
            c.project_wall,
            self.delivery_wall,
        ];
        last.parts = [
            c.prepare,
            c.read_parse,
            c.screen,
            c.evaluate,
            c.project + c.project_workers_cpu,
            c.assemble + c.delivery_workers_cpu,
        ];
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
        result
    }

    fn answer(
        &mut self,
        req: sdk::LintRequest,
        linked: &sdk::Languages,
        reverse_files: bool,
    ) -> Result<LiveReport, String> {
        let admission = Instant::now();
        if self.languages.is_empty() {
            self.languages = linked.0.iter().cloned().map(Pinned::from).collect();
        }
        if req.source_language != sdk::SourceLanguage::Recognized
            || req.project_search_scope != sdk::ProjectSearchScope::Undeclared
            || req.given_project_inventory != Default::default()
        {
            return Err("unsupported source or project authority".into());
        }
        let physical: Arc<str> = Arc::from(req.root.to_str().ok_or("non-UTF8 root")?);
        let (authority, mut listed, mut given) = match req.corpus {
            sdk::Corpus::Given(files) => (Authority::Given, Vec::new(), files),
            sdk::Corpus::Listed(paths) => {
                (Authority::Disk(physical.to_string()), paths, Vec::new())
            }
            _ => return Err("a closed source inventory is required".into()),
        };
        if reverse_files {
            listed.reverse();
            given.reverse();
        }
        let mut overlay = BTreeMap::new();
        for source in given.into_iter().chain(req.beside) {
            if source.executable != sdk::ExecutablePermission::Unknown {
                return Err("unhandled supplied executable permission".into());
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
        let filter = self.globs(&req.files)?;
        let hosted = req
            .hosted
            .into_iter()
            .map(|t| self.cache.table(t))
            .collect::<Result<Vec<_>, _>>()?;
        let mut ids = Vec::new();
        let scope=if let Some(s)=&mut self.store{
            // The unread list belongs to the union of requested project relations.
            // File-only requests have no dependency on this peer inventory.
            let peers=req.rules.iter().map(|(id,rule)|{
                let parameters=rule.parameters.iter().map(|(k,v)|Ok((&k.0,parameter(v.clone())?))).collect::<Result<Vec<_>,String>>()?;
                Ok(super::stored::key((id.as_ref(),rule.query.as_ref(),&rule.include,&rule.exclude,parameters)))
            }).collect::<Result<Vec<_>,String>>()?;
            let shape=hosted.iter().map(|h|(&h.name,&h.columns)).collect::<Vec<_>>();
            s.overlay(std::path::Path::new(&*physical),&paths,super::stored::key((peers,shape)),&key.1);
            Some(super::stored::key((&physical,&authority)))
        }else{None};
        let host_keys=self.store.as_ref().map(|_|hosted.iter().map(super::stored::key).collect::<Vec<_>>());
        let mut restored_unread=BTreeMap::<String,usize>::new();
        self.batch.work.clear();
        let mut rules = req.rules.into_iter();
        let mut prepared = Vec::new();
        let workers = self.batch.workers.map_or(1, std::num::NonZeroUsize::get);
        if self.store.is_none() && workers > 1 && rules.len() >= 128 {
            prepared.extend(rules.by_ref().map(|(id, rule)| (id, rule, None)));
            let chunk_size = prepared.len().div_ceil(workers);
            self.admission_shards += prepared.len().div_ceil(chunk_size);
            self.admission_workers_cpu += std::thread::scope(|scope| {
                let decode = |chunk: &mut [(_, sdk::Rule, Option<_>)]| {
                    let start = thread_cpu();
                    for (_, rule, typed) in chunk {
                        *typed = Some(parameters(std::mem::take(&mut rule.parameters)));
                    }
                    thread_cpu() - start
                };
                let mut chunks = prepared.chunks_mut(chunk_size);
                let first = chunks.next().expect("non-empty rules");
                let children: Vec<_> = chunks.map(|chunk| scope.spawn(move || decode(chunk))).collect();
                decode(first);
                children.into_iter().map(|child| child.join().expect("admission worker panicked")).sum::<f64>()
            });
        }
        let serial = rules.map(|(id, rule)| (id, rule, None));
        for (id, rule, typed) in serial.chain(prepared) {
            if rule.applicability != sdk::RuleApplicability::Always || !rule.observations.is_empty()
            {
                return Err(format!(
                    "undeclared observation or syntax requirement for {id}"
                ));
            }
            let inc = self.globs(&rule.include)?;
            let exc = self.globs(&rule.exclude)?;
            let files = self.cache.files(
                (inventory, filter.0, inc.0, exc.0),
                &filter.1,
                &inc.1,
                &exc.1,
            );
            let parameters = typed.unwrap_or_else(|| parameters(rule.parameters))?;
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
                if let Some(reason)=&store.packet(&packet).refusal{return Err(format!("{id}: {reason}"))}
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
                self.batch.schedule.provide(&id, p.clone(), v.clone())?;
            }
            self.versions.insert(key.clone(), id.clone());
            id
        };
        for p in &listed {
            if !key.1.contains_key(p.0.as_ref()) {
                self.batch.schedule.listed(&root, &physical, &p.0)?;
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
                .map_err(|e| format!("{id}: {e}"))?;
            if let super::Work::Invalid(reason)=&work{
                if let (Some(store),Some(k))=(&mut self.store,contract){store.decline(k,reason);}
                return Err(format!("{id}: {reason}"))
            }
            ids.push((id,Err((self.batch.work.len(),contract))));
            self.batch.work.push(work);
        }
        self.admission_wall += admission.elapsed().as_secs_f64();
        let epoch = Instant::now();
        if let Some(store)=&mut self.store {
            if !self.batch.work.is_empty(){self.batch.epoch_with_store(&self.languages,&mut self.state,Some(store));}else{self.state.manifest_failures.clear();}
        } else {
            self.batch.epoch(&self.languages,&mut self.state);
        }
        self.file_epoch_wall += epoch.elapsed().as_secs_f64();
        let deliver = Instant::now();
        let cpu = thread_cpu();
        let mut out = LiveReport::default();
        let count = self.batch.workers.map_or(1, std::num::NonZeroUsize::get);
        // Validation and fixture calls usually contain a single request.
        // Keep the existing per-request writer and amortize launches over a batch.
        if self.store.is_none() && count > 1 && ids.len() >= 128 {
            let mut receipts = ids.iter().map(|(id, entry)| {
                let (i, _) = entry.as_ref().err().expect("fresh receipt");
                let receipt = Batch::answer(&self.batch.work[*i], &self.state.answers)
                    .map_err(|e| format!("{id}: {e}"))?;
                Ok::<_, String>((id.as_ref(), receipt))
            }).collect::<Result<Vec<_>, _>>()?;
            let chunk_size = receipts.len().div_ceil(count);
            let other_cpu = std::thread::scope(|scope| {
                let mut chunks = receipts.chunks_mut(chunk_size);
                let first = chunks.next().expect("non-empty requests");
                let children: Vec<_> = chunks.map(|chunk| scope.spawn(move || {
                    let start = thread_cpu();
                    let result = render_chunk(chunk);
                    (result, thread_cpu() - start)
                })).collect();
                let (report, mut dictionaries) = render_chunk(first)?;
                out = report;
                let mut cpu = 0.;
                for child in children {
                    let (part, time) = child.join().expect("receipt worker panicked");
                    let (part, count) = part?;
                    dictionaries += count;
                    out.findings.extend(part.findings);
                    out.unknowns.extend(part.unknowns);
                    cpu += time;
                }
                Ok::<_, String>((cpu, dictionaries))
            })?;
            self.parallel_rows += out.findings.len();
            self.delivery_dictionaries += other_cpu.1;
            self.batch.clocks.delivery_workers_cpu += other_cpu.0;
        } else {
            for (id,entry) in ids {
                let held;
                let receipt=match entry {
                    Ok(k)=>{
                        held=self.store.as_ref().unwrap().packet(&k).report();
                        super::Receipt {rows:held.rows.iter().collect(),unknowns:held.unknowns.iter().collect()}
                    }
                    Err((i,k))=>{
                        let r=Batch::answer(&self.batch.work[i],&self.state.answers).map_err(|e|format!("{id}: {e}"))?;
                        if let (Some(store),Some(k))=(&mut self.store,k){store.save(k,i,&r);}
                        r
                    }
                };
                render_receipt::<false>(id.as_ref(), receipt, &mut out, None)?;
            }
        }
        for path in Rc::unwrap_or_clone(paths) {
            let p = sdk::RelativePath(path.into());
            if linked.reads(&p) != release_sdk::languages::Recognition::Reads
                && !linked.0.iter().any(|reader| reader.primary_role(release_sdk::languages::SourcePath(&p.0)) == release_language::domain::files::types::PrimaryFileRole::Configuration)
            {
                out.unread.push(p.0);
            }
        }
        let mut manifest=BTreeMap::<String,usize>::new();
        for p in &self.state.manifest_failures{*manifest.entry(p.0.to_string()).or_default()+=1;}
        for (p,n) in restored_unread{let total=manifest.entry(p).or_default();*total=(*total).max(n);}
        for (p,n) in manifest{out.unread.extend(std::iter::repeat_n(Box::<str>::from(p),n));}
        out.unread.sort_unstable();
        self.batch.clocks.assemble += thread_cpu() - cpu;
        self.delivery_wall += deliver.elapsed().as_secs_f64();
        Ok(out)
    }
}

type Dictionaries<'a> =
    HashMap<usize, std::sync::RwLockReadGuard<'a, crate::finish::answer::Words>>;

fn render_chunk(entries: &mut [(&str, super::Receipt<'_>)]) -> Result<(LiveReport, usize), String> {
    let mut result = LiveReport::default();
    let mut dictionaries = Dictionaries::new();
    for (id, receipt) in entries {
        render_receipt::<true>(id, std::mem::take(receipt), &mut result, Some(&mut dictionaries))?;
    }
    Ok((result, dictionaries.len()))
}

fn render_receipt<'a, const SHARED: bool>(
    id: &str,
    receipt: super::Receipt<'a>,
    out: &mut LiveReport,
    mut dictionaries: Option<&mut Dictionaries<'a>>,
) -> Result<(), String> {
    for row in receipt.rows {
        let words = if SHARED {
            let table = row.dictionary();
            Some(&**dictionaries.as_deref_mut().expect("delivery worker dictionary cache")
                .entry(std::ptr::from_ref(table) as usize)
                .or_insert_with(|| table.read().expect("answer dictionary")))
        } else {
            None
        };
        let value = |column| match words {
            Some(words) => row.value_from(column, words),
            None => row.value(column),
        };
        let subject = row.layout.identity.first()
            .map(|&i| value(i))
            .ok_or("missing row subject")?;
        out.findings.push((
            id.into(),
            slot(&subject),
            row.layout.names.iter().zip((0..row.width()).map(value))
                .map(|(n, v)| (n.as_str().into(), slot(&v)))
                .collect(),
        ));
    }
    for (p, msg) in receipt.unknowns {
        out.unknowns.push((id.into(), p.clone(), msg.clone()));
    }
    Ok(())
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
    project_counts: [usize; 4],
    calls: usize,
    requests: usize,
    refused: usize,
    reasons: BTreeMap<String, usize>,
    elapsed: f64,
    worker_cpu: f64,
    file_shards: usize,
    dispatch_wall: [f64; 5],
    largest_file: String,
    parallel_rows: usize,
    delivery_dictionaries: usize,
    empty_file_epochs: usize,
    omitted_file_bindings: usize,
    admission_shards: usize,
    project_shards: usize,
    project_prepare_wall: f64,
    call_wall: f64,
    phase_wall: [f64; 4],
    parts: [f64; 6],
    counts: [usize; 8],
    retained: [u64; 4],
}
static MEASURED: std::sync::Mutex<Measurement> = std::sync::Mutex::new(Measurement {
    sessions: 0,
    project_counts: [0; 4],
    calls: 0,
    requests: 0,
    refused: 0,
    reasons: BTreeMap::new(),
    elapsed: 0.0,
    worker_cpu: 0.0,
    file_shards: 0,
    dispatch_wall: [0.; 5],
    largest_file: String::new(),
    parallel_rows: 0,
    delivery_dictionaries: 0,
    empty_file_epochs: 0,
    omitted_file_bindings: 0,
    admission_shards: 0,
    project_shards: 0,
    project_prepare_wall: 0.,
    call_wall: 0.0,
    phase_wall: [0.0; 4],
    parts: [0.0; 6],
    counts: [0; 8],
    retained: [0; 4],
});

#[derive(Clone, Copy)]
pub enum HostPhase { Conversion, Validation, Emission }

static HOST: std::sync::Mutex<[f64; 3]> = std::sync::Mutex::new([0.; 3]);

pub fn host_wall(phase: HostPhase, seconds: f64) {
    HOST.lock().expect("host clocks")[phase as usize] += seconds;
}

pub fn host_snapshot() -> (f64, f64) {
    let sdk = MEASURED.lock().expect("SDK clocks").call_wall;
    (sdk, HOST.lock().expect("host clocks")[HostPhase::Conversion as usize])
}

/// Print the one executor's cumulative counters to the benchmark log after the host renders.
pub fn print_measurement() {
    let [conversion, validation, emission] = *HOST.lock().expect("host clocks");
    eprintln!("LIVE_HOST_WALL conversion={conversion:.6} validation={validation:.6} emission={emission:.6}");
    let m = MEASURED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (reason, count) in &m.reasons {
        eprintln!("UNANSWERED\t{count}\t{reason}");
    }
    let [versions,hits,evictions,peak_charge]=m.retained;
    eprintln!("FILE_CACHE versions={versions} hits={hits} evictions={evictions} peak_charge={peak_charge}");
    let [prepare, read, first, second, project, assembly] = m.parts;
    let [inventory, join, critical, busy, largest] = m.dispatch_wall;
    eprintln!("LIVE_FILE_WALL inventory={inventory:.6} join={join:.6} critical={critical:.6} busy={busy:.6} largest={largest:.6}");
    if largest > 0. {
        eprintln!("LIVE_LARGEST_FILE {:?}", m.largest_file);
    }
    let [admission_wall, file_wall, project_wall, delivery_wall] = m.phase_wall;
    eprintln!("LIVE_DELIVERY rows={} dictionary_borrows={}", m.parallel_rows, m.delivery_dictionaries);
    eprintln!(
        "LIVE_WORKERS empty_file_epochs={} omitted_file_bindings={} file_shards={} admission_shards={} project_shards={} child_cpu={:.6} project_prepare_wall={:.6} project_execute_wall={:.6}",
        m.empty_file_epochs, m.omitted_file_bindings, m.file_shards, m.admission_shards, m.project_shards, m.worker_cpu,
        m.project_prepare_wall, project_wall - m.project_prepare_wall,
    );
    eprintln!("LIVE_PHASE_WALL calls={:.6} admission={admission_wall:.6} files={file_wall:.6} project={project_wall:.6} delivery={delivery_wall:.6}",m.call_wall);
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
        m.elapsed + m.worker_cpu - prepare - read - first - second - project - assembly
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
