//! Typed calls from the pinned check. A refusal returns to the host as an error.
use super::super::{HostCell, Parameters};
use super::{Active, Authority, Batch, Hosted, Input, LiveAdmission};
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
    collections::{BTreeMap, HashMap, VecDeque},
    rc::Rc,
    sync::Arc,
    time::Instant,
};

#[derive(Clone, Default)]
pub struct LiveReport {
    pub findings: Vec<(Box<str>, sdk::Slot, Vec<(Box<str>, sdk::Slot)>)>,
    pub unknowns: Vec<(Box<str>, String, String)>,
    pub unread: Vec<Box<str>>,
}

// This cache accepts only immutable, parameterless requests with a pure report.
// The version already identifies the normalized input bytes for Given authority.
#[derive(PartialEq)]
struct CallKey {
    version: Option<Arc<str>>,
    inventory: usize,
    filter: usize,
    tables: Vec<usize>,
    rules: Vec<(sdk::RuleId, Rc<str>, usize, usize)>,
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
    completed_calls: [VecDeque<(CallKey, LiveReport)>; 2],
    skipped_calls: usize,
    skipped_rules: usize,
    registered_tables: usize,
    delivered_cells: usize,
    delivered_rows: usize,
    epochs: usize,
    shared_overlay_files: usize,
    shared_overlay_bytes: usize,
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
            completed_calls: Default::default(),
            skipped_calls: 0,
            skipped_rules: 0,
            registered_tables: 0,
            delivered_cells: 0,
            delivered_rows: 0,
            epochs: 0,
            shared_overlay_files: 0,
            shared_overlay_bytes: 0,
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

fn worker_slot(v: &Option<Cell<std::sync::Arc<str>>>) -> sdk::Slot {
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

fn slot(v: Option<Cell>) -> sdk::Slot {
    let Some(c) = v else {
        return sdk::Slot::Nothing;
    };
    use release_sdk::languages::{Offset, Span};
    sdk::Slot::Is(match c {
        Cell::File(p) => sdk::Cell::File(sdk::RelativePath(p)),
        Cell::Text(s) => sdk::Cell::Text(sdk::HostText(s)),
        Cell::Number(n) => sdk::Cell::Number(sdk::HostNumber(n.0)),
        Cell::Span(s) => sdk::Cell::Span(Span {
            start: Offset(s.start.0),
            end: Offset(s.end.0),
        }),
    })
}

fn unread_path(linked: &sdk::Languages, path: &sdk::RelativePath) -> bool {
    linked.reads(path) != release_sdk::languages::Recognition::Reads
        && !linked.0.iter().any(|reader| {
            reader.primary_role(release_sdk::languages::SourcePath(&path.0))
                == release_language::domain::files::types::PrimaryFileRole::Configuration
        })
}

fn small_report(report: &LiveReport) -> bool {
    fn bytes(slot: &sdk::Slot) -> usize {
        match slot {
            sdk::Slot::Is(sdk::Cell::File(path)) => path.0.len(),
            sdk::Slot::Is(sdk::Cell::Text(text)) => text.0.len(),
            _ => 0,
        }
    }
    report.unknowns.is_empty() && report.unread.is_empty() && report.findings.len() <= 16
        && report.findings.iter().all(|(id, subject, columns)| {
            columns.len() <= 16 && id.len() + bytes(subject)
                + columns.iter().map(|(name, cell)| name.len() + bytes(cell)).sum::<usize>() <= 256
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
        let result = self.answer(req, linked, options.reverse_files, limit.is_none());
        crate::finish::operators::flush();
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
        last.bound_proofs = c.bound_proofs;
        last.literal_positions = c.literal_positions;
        last.requirement_programs = c.requirement_programs;
        last.requirement_bindings = c.requirement_bindings;
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
        last.project_numbered = c.project_numbered;
        last.empty_project_epochs = c.empty_project_epochs;
        last.project_number_wall = c.project_number_wall;
        last.project_provider_wall = c.project_provider_wall;
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
        last.reused_binding_entries = self.batch.reused_binding_entries;
        last.pattern_visits = self.cache.pattern_visits;
        last.overlay = [self.shared_overlay_files, self.shared_overlay_bytes];
        last.registration = [self.batch.source_lookups_removed, self.epochs, self.skipped_calls,
            self.skipped_rules, self.batch.host_binding_counts[0], self.batch.host_binding_counts[1],
            self.registered_tables, self.delivered_rows, self.delivered_cells, self.batch.slot_lookups_removed];
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
            c.productive,
            c.refused,
        ];
        result
    }

    fn answer(
        &mut self,
        req: sdk::LintRequest,
        linked: &sdk::Languages,
        reverse_files: bool,
        unbounded: bool,
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
        let mut key = (authority.clone(), overlay);
        let mut prepared_root=None;
        let filter = self.globs(&req.files)?;
        let mut host_keys = self.store.as_ref().map(|_| Vec::new());
        let (table_ids, hosted): (Vec<_>, Vec<_>) = req
            .hosted
            .into_iter()
            .map(|t| self.cache.table(t, host_keys.as_mut()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .unzip();
        self.registered_tables += hosted.len();
        let memoize = unbounded && self.store.is_none() && authority == Authority::Given
            && table_ids.len() <= 32 && !req.rules.is_empty() && req.rules.len() <= 16
            && req.rules.iter().all(|(id, rule)| id.as_ref().len() <= 256
                && rule.applicability == sdk::RuleApplicability::Always
                && rule.observations.is_empty() && rule.parameters.is_empty());
        let memo_pool = usize::from(!paths.is_empty());
        let memo = if memoize {
            let mut rules = Vec::with_capacity(req.rules.len());
            for (id, rule) in &req.rules {
                rules.push((id.clone(), self.cache.query(rule.query.as_ref()),
                    self.globs(&rule.include)?.0, self.globs(&rule.exclude)?.0));
            }
            let memo = CallKey {
                version: self.versions.get(&key).cloned(),
                inventory,
                filter: filter.0,
                tables: table_ids.clone(),
                rules,
            };
            if self.batch.schedule.pending.is_empty()
                && let Some((_, report)) = self.completed_calls[memo_pool].iter().find(|(saved, _)| saved == &memo)
                && paths.iter().all(|path| !unread_path(linked, &sdk::RelativePath(path.as_str().into())))
            {
                let start = thread_cpu();
                let report = report.clone();
                self.batch.clocks.assemble += thread_cpu() - start;
                self.skipped_calls += 1;
                self.skipped_rules += memo.rules.len();
                return Ok(report);
            }
            Some(memo)
        } else {
            None
        };
        self.batch.last_sources = None;
        let mut ids = Vec::new();
        let mut retained_rules = Vec::new();
        let scope = if self.store.is_some() {
            // The host supplies the inventory and relations in memory. Reuse
            // the prepared program, selection and table identities across calls.
            let mut peers = Vec::with_capacity(req.rules.len());
            for (id, rule) in &req.rules {
                let parameters = rule.parameters.iter()
                    .map(|(k, v)| Ok((k.0.clone(), parameter(v.clone())?)))
                    .collect::<Result<Parameters, String>>()?;
                let query = self.cache.query(rule.query.as_ref());
                let program = self.batch.prepared_rule(query.clone(), &parameters);
                let digest = self.cache.program_digest(program, &query, &parameters);
                peers.push(super::stored::key((id.as_ref(), digest, &rule.include, &rule.exclude)));
                retained_rules.push((query, parameters, program, digest));
            }
            let shape = hosted.iter().map(|h| (&h.name, &h.columns)).collect::<Vec<_>>();
            self.store.as_mut().unwrap().overlay(
                std::path::Path::new(&*physical),
                self.cache.path_digest(&paths),
                super::stored::key((peers, shape)),
                &key.1,
            );
            Some(super::stored::key((&physical, &authority)))
        } else {
            None
        };
        let mut retained_rules = retained_rules.into_iter();
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
            let (selection, files) = self.cache.files(
                (inventory, filter.0, inc.0, exc.0),
                &filter.1,
                &inc.1,
                &exc.1,
            );
            let (query, parameters, contract) = if let Some(scope) = scope {
                let (query, parameters, program, digest) =
                    retained_rules.next().expect("retained rule identity");
                let plan = self.batch.schedule.programs[program].as_ref().ok();
                let consumed: Vec<_> = hosted.iter().enumerate()
                    .filter(|(_, table)| plan.is_some_and(|p| p.bases.contains(&table.name)))
                    .map(|(i, _)| host_keys.as_ref().unwrap()[i])
                    .collect();
                let identity = super::stored::key((
                    scope, digest, self.cache.path_digest(&files), consumed,
                ));
                (query, parameters, Some(identity))
            } else {
                let parameters = typed.unwrap_or_else(|| parameters(rule.parameters))?;
                (self.cache.query(rule.query.as_ref()), parameters, None)
            };
            if let (Some(store),Some(k))=(&mut self.store,contract)
                && let Some((packet,unread))=store.lookup(k){
                let mut counts=BTreeMap::<String,usize>::new();
                for p in unread{*counts.entry(p.clone()).or_default()+=1;}
                if let Some(reason)=&store.packet(&packet).refusal{return Err(format!("{id}: {reason}"))}
                for (p,n) in counts{let total=restored_unread.entry(p).or_default();*total=(*total).max(n);}
                ids.push((id,Ok(packet)));
                continue
            }
            if prepared_root.is_none(){
        let root = if let Some(id) = self.versions.get(&key) {
            id.clone()
        } else {
            let id: Arc<str> = Arc::from(format!("check-input:{}", self.versions.len()));
            for (path, bytes) in &mut key.1 {
                let shared = self.batch.schedule.provide(&id, path, bytes)?;
                // The scheduler already owns the exact path and bytes. Retain
                // its allocation in the version key and drop the duplicate.
                if !Arc::ptr_eq(bytes, &shared) {
                    self.shared_overlay_files += 1;
                    self.shared_overlay_bytes += bytes.len();
                    *bytes = shared;
                }
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
                    Some(LiveAdmission { version: root, selection, host_tables: &table_ids }),
                )
                .map_err(|e| format!("{id}: {e}"))?;
            if let super::Work::Invalid(reason)=&work{
                if let (Some(store),Some(k))=(&mut self.store,contract){store.decline(k,reason);}
                return Err(format!("{id}: {reason}"))
            }
            ids.push((id,Err((self.batch.work.len(),contract))));
            self.batch.work.push(work);
        }
        self.epochs += 1;
        self.batch.last_sources = None;
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
            for row in receipt.rows {
                let columns: Vec<_> = row.layout.names.iter()
                    .zip(row.cells_with(|text| Box::<str>::from(text.as_ref())))
                    .map(|(name, value)| (name.as_str().into(), slot(value)))
                    .collect();
                let subject = row.layout.identity.first().and_then(|&i| columns.get(i))
                    .ok_or("missing row subject")?.1.clone();
                self.delivered_rows += 1;
                self.delivered_cells += columns.len();
                out.findings.push((id.as_ref().into(), subject, columns));
            }
            for (p, msg) in receipt.unknowns {
                out.unknowns
                    .push((id.as_ref().into(), p.clone(), msg.clone()));
            }
            }
        }
        for path in Rc::unwrap_or_clone(paths) {
            let p = sdk::RelativePath(path.into());
            if unread_path(linked, &p) {
                out.unread.push(p.0);
            }
        }
        let mut manifest=BTreeMap::<String,usize>::new();
        for p in &self.state.manifest_failures{*manifest.entry(p.0.to_string()).or_default()+=1;}
        for (p,n) in restored_unread{let total=manifest.entry(p).or_default();*total=(*total).max(n);}
        for (p,n) in manifest{out.unread.extend(std::iter::repeat_n(Box::<str>::from(p),n));}
        out.unread.sort_unstable();
        if let Some(mut memo) = memo && small_report(&out)
            && let Some(version) = prepared_root
        {
            memo.version = Some(version);
            let entries = &mut self.completed_calls[memo_pool];
            if entries.len() == 128 {
                entries.pop_front();
            }
            entries.push_back((memo, out.clone()));
        }
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
            worker_slot(&subject),
            row.layout.names.iter().zip((0..row.width()).map(value))
                .map(|(n, v)| (n.as_str().into(), worker_slot(&v)))
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
    project_provider_wall: f64,
    project_number_wall: f64,
    project_numbered: usize,
    empty_project_epochs: usize,
    call_wall: f64,
    phase_wall: [f64; 4],
    parts: [f64; 6],
    bound_proofs: f64,
    literal_positions: usize,
    requirement_programs: usize,
    requirement_bindings: usize,
    counts: [usize; 10],
    retained: [u64; 4],
    reused_binding_entries: usize,
    pattern_visits: [usize; 4],
    registration: [usize; 10],
    overlay: [usize; 2],
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
    project_provider_wall: 0.,
    project_number_wall: 0.,
    project_numbered: 0,
    empty_project_epochs: 0,
    call_wall: 0.0,
    phase_wall: [0.0; 4],
    parts: [0.0; 6],
    bound_proofs: 0.,
    literal_positions: 0,
    requirement_programs: 0,
    requirement_bindings: 0,
    counts: [0; 10],
    retained: [0; 4],
    reused_binding_entries: 0,
    pattern_visits: [0; 4],
    registration: [0; 10],
    overlay: [0; 2],
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
    eprintln!("ADMISSION_EPOCH reused_binding_entries={}", m.reused_binding_entries);
    let [before, after, selections, searches] = m.pattern_visits;
    eprintln!("ADMISSION_PATHS prior_visits={before} visits={after} indexed_selections={selections} searches={searches}");
    let [paths, epochs, calls, rules, host_uses, row_hashes, tables, rows, cells, slots] = m.registration;
    eprintln!("ADMISSION_SMALL source_lookups_removed={paths} slot_lookups_removed={slots} epochs={epochs} whole_calls_reused={calls} registrations_removed={rules} bound_host_uses={host_uses} row_hashes_removed={row_hashes} supplied_tables={tables} delivery_rows={rows} delivery_cells={cells}");
    let [files, bytes] = m.overlay;
    eprintln!("ADMISSION_OVERLAY shared_versions={files} duplicate_source_bytes_removed={bytes}");
    let [versions,hits,evictions,peak_charge]=m.retained;
    eprintln!("FILE_CACHE versions={versions} hits={hits} evictions={evictions} peak_charge={peak_charge}");
    eprintln!(
        "LIVE_PROJECT_WALL provider={:.6} numbering={:.6} numbered={} empty_epochs={}",
        m.project_provider_wall, m.project_number_wall, m.project_numbered, m.empty_project_epochs
    );
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
    let [plans, bindings, sources, reads, parses, pairs, skips, eval, productive, refused] = m.counts;
    let empty = eval - productive - refused;
    let [inventories, builds, relations, repeated] = m.project_counts;
    eprintln!(
        "PROJECT_POPULATIONS sessions={} inventories={inventories} builds={builds} relations={relations} repeated_relations={repeated}",
        m.sessions,
    );
    eprintln!(
        "CHECK_SESSION live calls={} requests={} failed_calls={} programs={plans} bindings={bindings} sources={sources} reads={reads} parses={parses} distinct_pairs={pairs} skipped={skips} evaluated={eval} productive={productive} empty={empty} refused={refused} prep={prepare:.6} read_parse={read:.6} first={first:.6} second={second:.6} project={project:.6} assembly={assembly:.6} admission={:.6} bound_proofs={:.6} literal_positions={} requirement_programs={} requirement_bindings={}",
        m.calls,
        m.requests,
        m.refused,
        m.elapsed + m.worker_cpu - m.parts.iter().sum::<f64>(),
        m.bound_proofs,
        m.literal_positions,
        m.requirement_programs,
        m.requirement_bindings
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
