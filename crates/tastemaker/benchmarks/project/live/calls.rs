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
#[path = "../../../../ryo/crates/engine/benches/lint_image/sdk_request.rs"]
mod sdk_request;
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
    time::Instant,
    sync::{Arc, atomic::{AtomicU64, Ordering}},
};

use release_sdk::linked_sdk::{finding, report, unknown, unread};

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

// This cache accepts only immutable, parameterless requests with a pure report.
// The version already identifies the normalized input bytes for Given authority.
#[derive(PartialEq)]
struct CallKey {
    version: Option<Arc<str>>,
    inventory: usize,
    filter: usize,
    tables: Vec<usize>,
    rules: Vec<(sdk::RuleId, crate::check_session::query::Query, usize, usize)>,
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

enum RoundRequest {
    Complete(sdk::LintReport),
    Scheduled(Pending),
}

type WorkEntry = (
    sdk::RuleId,
    Result<super::stored::Key, (usize, Option<super::stored::Key>)>,
);

struct Pending {
    ids: Vec<WorkEntry>,
    named_reader: bool,
    paths: Rc<Vec<String>>,
    projects: std::collections::BTreeSet<usize>,
    restored_unread: BTreeMap<String, usize>,
    memo: Option<(usize, CallKey)>,
}

struct Live {
    batch: Batch,
    store:Option<super::stored::Store>,
    directory:Option<std::path::PathBuf>,
    state: Active<pinned::File, Dictionary>,
    languages: Vec<Pinned>,
    filters: HashMap<Vec<sdk::Glob>, (usize, Rc<PathFilter>)>,
    cache: CallCache,
    completed_calls: [VecDeque<(CallKey, sdk::LintReport)>; 2],
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
    environments: HashMap<Arc<str>, Arc<crate::check_session::project::Environment>>,
    versions: HashMap<(Authority, BTreeMap<String, (Arc<[u8]>, crate::check_session::Permission)>, Option<Arc<str>>), Arc<str>>,
}

impl Drop for Live {
    fn drop(&mut self){
        let (writes, pages) = self.state.aliases.counts();
        eprintln!("PAIR_ALIASES pairs={} aliases={writes} allocated_pages={pages}", self.batch.schedule.pairs);
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
            admission_wall: 0.0,
            admission_workers_cpu: 0.0,
            admission_shards: 0,
            delivery_wall: 0.0,
            file_epoch_wall: 0.0,
            parallel_rows: 0,
            delivery_dictionaries: 0,
            store:None, directory:None,
            state,
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
            environments: HashMap::new(),
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
    eprintln!("plan refusal for {id}: {reason}");
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

fn unread_path(linked: &sdk::Languages, path: &sdk::RelativePath) -> bool {
    linked.reads(path) != release_sdk::languages::Recognition::Reads
        && !linked.0.iter().any(|reader| {
            reader.primary_role(release_sdk::languages::SourcePath(&path.0))
                == release_language::domain::files::types::PrimaryFileRole::Configuration
        })
}

fn small_report(report: &sdk::LintReport) -> bool {
    fn bytes(slot: &sdk::Slot) -> usize {
        match slot {
            sdk::Slot::Is(sdk::Cell::File(path)) => path.0.len(),
            sdk::Slot::Is(sdk::Cell::Text(text)) => text.0.len(),
            _ => 0,
        }
    }
    report.unknowns.is_empty() && report.unread.is_empty() && report.findings.len() <= 16
        && report.findings.iter().all(|f| {
            f.columns.len() <= 16 && f.rule.as_ref().len() + bytes(&f.subject)
                + f.key.0.iter().map(bytes).sum::<usize>()
                + f.columns.iter().map(|column| column.name.0.len() + bytes(&column.value)).sum::<usize>() <= 256
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
            self.store=directory.map(|p|super::stored::Store::directory(p.to_owned(), &self.languages)).transpose().map_err(|e|e.to_string())?;
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

    fn child_cpu(&self) -> f64 {
        let c = &self.batch.clocks;
        c.delivery_workers_cpu + c.file_workers_cpu + c.project_workers_cpu + self.admission_workers_cpu
    }

    fn phase_totals(&self) -> [f64; 6] {
        let c = &self.batch.clocks;
        [c.prepare, c.read_parse, c.screen, c.evaluate, c.project + c.project_workers_cpu, c.assemble + c.delivery_workers_cpu]
    }

    fn lint(
        &mut self,
        requests: Vec<sdk::LintRequest>,
        linked: &sdk::Languages,
        ) -> Vec<Reply> {
        // A retained proof references the currently installed source overlay.
        // Close that epoch before installing the next overlay. Both lifetimes
        // execute the same admission, visit and delivery below.
        if self.store.is_some() {
            return requests.into_iter().flat_map(|request| self.epoch(vec![request], linked)).collect();
        }
        self.epoch(requests, linked)
    }

    fn epoch(&mut self, requests: Vec<sdk::LintRequest>, linked: &sdk::Languages) -> Vec<Reply> {
        let previous_children = self.child_cpu();
        let previous = self.phase_totals();
        let options = check_options();
        self.batch.workers = options.workers;
        let wall = Instant::now();
        let admission = Instant::now();
        if self.languages.is_empty() {
            self.languages = linked.0.iter().cloned().map(Pinned::from).collect();
            match crate::check_session::named::Catalog::from_readers(&self.languages) {
                Ok(catalog) => self.batch.file_catalog = catalog,
                Err(reason) => {
                    self.languages.clear();
                    return requests.into_iter().map(|_| Err(execution(reason.clone()))).collect();
                }
            }
        }
        let count = requests.iter().map(|request| request.rules.len()).sum::<usize>();
        let pending = requests.into_iter().map(|request| {
            let start = self.batch.work.len();
            let result = self.admit(request, linked, options.reverse_files, true);
            if result.is_err() {
                self.batch.work.truncate(start);
            }
            result
        }).collect::<Vec<_>>();
        self.admission_wall += admission.elapsed().as_secs_f64();
        let epoch = Instant::now();
        if !self.batch.work.is_empty() {
            self.epochs += 1;
            if let Some(store) = &mut self.store {
                self.batch.epoch_with_store(&self.languages, &mut self.state, Some(store));
            } else {
                self.batch.epoch(&self.languages, &mut self.state);
            }
        }
        self.file_epoch_wall += epoch.elapsed().as_secs_f64();
        let results = pending.into_iter().map(|request| {
            request.and_then(|request| match request {
                RoundRequest::Complete(report) => Ok(report),
                RoundRequest::Scheduled(work) => self.deliver(work, linked),
            })
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
        last.bound_proofs = c.bound_proofs;
        last.literal_positions = c.literal_positions;
        last.certified = c.certified;
        last.requirement_programs = c.requirement_programs;
        last.requirement_bindings = c.requirement_bindings;
        let children = self.child_cpu();
        last.worker_cpu += children - previous_children;
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
        results
    }

    fn admit(&mut self, req: sdk::LintRequest, linked: &sdk::Languages, reverse_files: bool, unbounded: bool) -> Result<RoundRequest, sdk::LintError> {
        if req.given_project_inventory != Default::default()
        {
            return Err(execution("unsupported source or project authority".into()));
        }
        let physical: Arc<str> = Arc::from(req.root.to_str().ok_or_else(|| sdk::LintError::PathNotText { path: req.root.clone() })?);
        let (authority, mut listed, mut given) = match req.corpus {
            sdk::Corpus::Given(files) => (Authority::Given, Vec::new(), files),
            sdk::Corpus::Listed(paths) => {
                (Authority::Disk(physical.to_string()), paths, Vec::new())
            }
            _ => return Err(execution("a closed source inventory is required".into())),
        };
        let authority = match req.project_search_scope {
            sdk::ProjectSearchScope::Undeclared => authority,
            sdk::ProjectSearchScope::Declared(roots) => authority.with_roots(roots.into_iter().map(|p| p.0.into_string()).collect()),
        };
        if reverse_files {
            listed.reverse();
            given.reverse();
        }
        let mut overlay = BTreeMap::new();
        for source in given.into_iter().chain(req.beside) {
            use crate::check_session::Permission;
            let permission = match source.executable {
                sdk::ExecutablePermission::Unknown => Permission::Unknown,
                sdk::ExecutablePermission::Unsupported => Permission::Unsupported,
                sdk::ExecutablePermission::PosixExecutable => Permission::PosixExecutable,
                sdk::ExecutablePermission::PosixNotExecutable => Permission::PosixNotExecutable,
            };
            overlay.insert(source.path.0.into_string(), (Arc::<[u8]>::from(source.text.0.as_bytes()), permission));
        }
        let (inventory, paths) = self.cache.inventory(listed.iter().map(|p| p.0.as_ref())
            .chain(overlay.keys().map(String::as_str)).collect());
        let reader: Option<Arc<str>> = match req.source_language {
            sdk::SourceLanguage::Recognized => None,
            sdk::SourceLanguage::Named(name) => {
                if linked.0.iter().filter(|l| l.name() == &name).count() != 1 {
                    return Err(execution(format!("named reader {} is absent or ambiguous", name.0)));
                }
                Some(Arc::from(name.0))
            }
        };
        let mut key = (authority.clone(), overlay, reader.clone());
        let mut prepared_root=None;
        for (index, relation) in req.hosted.iter().enumerate() {
            if req.hosted[..index].iter().any(|earlier| earlier.name == relation.name) {
                return Err(sdk::LintError::DuplicateHostRelation { name: relation.name.clone() });
            }
        }
        let filter = self.globs(&req.files).map_err(execution)?;
        let mut host_keys = self.store.as_ref().map(|_| Vec::new());
        let (table_ids, hosted): (Vec<_>, Vec<_>) = req
            .hosted
            .into_iter()
            .map(|t| self.cache.table(t, host_keys.as_mut()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .unzip();
        self.registered_tables += hosted.len();
        let inputs = sdk_request::inputs(&req.rules)?;
        let memoize = unbounded && self.store.is_none() && authority == Authority::Given
            && table_ids.len() <= 32 && !req.rules.is_empty() && req.rules.len() <= 16
            && req.rules.iter().all(|(id, rule)| id.as_ref().len() <= 256
                && rule.applicability == sdk::RuleApplicability::Always
                && rule.observations.is_empty() && rule.parameters.is_empty());
        let memo_pool = usize::from(!paths.is_empty());
        let memo = if memoize {
            let mut rules = Vec::with_capacity(req.rules.len());
            for (id, rule) in &req.rules {
                rules.push((id.clone(), sdk_request::query(&mut self.cache, rule, inputs.get(id)),
                    self.globs(&rule.include).map_err(execution)?.0, self.globs(&rule.exclude).map_err(execution)?.0));
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
                && (reader.is_some() || paths.iter().all(|path| !unread_path(linked, &sdk::RelativePath(path.as_str().into()))))
            {
                let start = thread_cpu();
                let report = report.clone();
                self.batch.clocks.assemble += thread_cpu() - start;
                self.skipped_calls += 1;
                self.skipped_rules += memo.rules.len();
                return Ok(RoundRequest::Complete(report));
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
                    .collect::<Result<Parameters, String>>().map_err(execution)?;
                let query = sdk_request::query(&mut self.cache, rule, inputs.get(id));
                let program = self.batch.prepared_rule(query.clone(), &parameters);
                let digest = self.cache.program_digest(program, &query, &parameters);
                peers.push(super::stored::key((id.as_ref(), digest, &rule.include, &rule.exclude)));
                retained_rules.push((query, parameters, program, digest));
            }
            let shape = hosted.iter().map(|h| (&h.name, &h.columns)).collect::<Vec<_>>();
            self.store.as_mut().unwrap().overlay(
                std::path::Path::new(&*physical),
                self.cache.path_digest(&paths),
                super::stored::key((peers, shape, &reader)),
                &key.1,
            );
            Some(super::stored::key((&physical, &authority, &reader)))
        } else {
            None
        };
        let mut retained_rules = retained_rules.into_iter();
        let mut restored_unread=BTreeMap::<String,usize>::new();
        let mut projects = std::collections::BTreeSet::new();
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
        for (id, mut rule, typed) in serial.chain(prepared) {
            let inc = self.globs(&rule.include).map_err(execution)?;
            let exc = self.globs(&rule.exclude).map_err(execution)?;
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
                let parameters = typed.unwrap_or_else(|| parameters(std::mem::take(&mut rule.parameters))).map_err(execution)?;
                (sdk_request::query(&mut self.cache, &rule, inputs.get(&id)), parameters, None)
            };
            if let (Some(store),Some(k))=(&mut self.store,contract)
                && let Some((packet,unread))=store.lookup(k){
                let mut counts=BTreeMap::<String,usize>::new();
                for p in unread{*counts.entry(p.clone()).or_default()+=1;}
                if let Some(reason)=&store.packet(&packet).refusal{return Err(rule_error(id, rule.query, linked, &hosted, reason.clone()))}
                for (p,n) in counts{let total=restored_unread.entry(p).or_default();*total=(*total).max(n);}
                ids.push((id,Ok(packet)));
                continue
            }
            if prepared_root.is_none(){
        let root = if let Some(id) = self.versions.get(&key) {
            id.clone()
        } else {
            let id: Arc<str> = Arc::from(format!("check-input:{}", self.versions.len()));
            use crate::check_session::source::Source as _;
            let environment = self.languages.iter().any(|p| p.uses_project() && reader.as_deref().is_none_or(|name| name == p.name())).then(|| Arc::new(crate::check_session::project::Environment {
                id: id.clone(), authority: authority.clone(), supplied: key.1.keys().cloned().collect(),
            }));
            if let Some(project) = &environment { self.environments.insert(id.clone(), project.clone()); }
            for (path, (bytes, permission)) in &mut key.1 {
                let shared = self.batch.schedule.provide(&id, path, bytes, *permission, &reader, &environment).map_err(execution)?;
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
                self.batch.schedule.listed(&root, &physical, &p.0, &reader, &self.environments.get(&root).cloned()).map_err(execution)?;
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
        self.batch.last_sources = None;
        let memo = memo.zip(prepared_root).map(|(mut key, version)| {
            key.version = Some(version);
            (memo_pool, key)
        });
        Ok(RoundRequest::Scheduled(Pending { ids, paths, projects, restored_unread, memo, named_reader: reader.is_some() }))
    }

    fn deliver(&mut self, request: Pending, linked: &sdk::Languages) -> Reply {
        let Pending { ids, paths, projects, restored_unread, memo, named_reader } = request;
        let deliver=Instant::now();
        let cpu=thread_cpu();
        let mut out = report(Vec::new(), Vec::new(), Vec::new());
        let count = self.batch.workers.map_or(1, std::num::NonZeroUsize::get);
        // Validation and fixture calls usually contain a single request.
        // Keep the existing per-request writer and amortize launches over a batch.
        if self.store.is_none() && count > 1 && ids.len() >= 128 {
            let mut receipts = ids.iter().map(|(id, entry)| {
                let (i, _) = entry.as_ref().err().expect("fresh receipt");
                let receipt = Batch::answer(&self.batch.work[*i], &self.state.answers)
                    .map_err(|e| sdk::LintError::Unrunnable { id: id.clone(), reason: e.to_string() })?;
                Ok::<_, sdk::LintError>((id, receipt))
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
                    out.skipped.extend(part.skipped);
                    out.findings.extend(part.findings);
                    out.unknowns.extend(part.unknowns);
                    cpu += time;
                }
                Ok::<_, sdk::LintError>((cpu, dictionaries))
            })?;
            drop(receipts);
            for (_, entry) in ids {
                let (i, _) = entry.expect_err("fresh receipt");
                self.batch.work[i] = super::Work::Invalid(String::new());
            }
            self.parallel_rows += out.findings.len();
            self.delivery_dictionaries += other_cpu.1;
            self.batch.clocks.delivery_workers_cpu += other_cpu.0;
        } else {
            for (id,entry) in ids {
                let consumed = entry.as_ref().err().map(|(i, _)| *i);
                let held;
                let receipt=match entry {
                    Ok(k)=>{
                        held=self.store.as_ref().unwrap().packet(&k).report();
                        super::Receipt {rejected_syntax:held.rejected_syntax.iter().collect(),rows:held.rows.iter().collect(),unknowns:held.unknowns.iter().collect()}
                    }
                    Err((i,k))=>{
                        let r=Batch::answer(&self.batch.work[i],&self.state.answers).map_err(|e| sdk::LintError::Unrunnable { id: id.clone(), reason: e.to_string() })?;
                        if let (Some(store),Some(k))=(&mut self.store,k){store.save(k,i,&r);}
                        r
                    }
                };
                render_receipt::<false>(&id, receipt, &mut out, None)?;
                if let Some(i) = consumed {
                    // Epoch evaluation and any retained proof are complete.
                    // Drop this request's source lists or closed rows at delivery.
                    self.batch.work[i] = super::Work::Invalid(String::new());
                }
            }
        }
        self.delivered_rows += out.findings.len();
        self.delivered_cells += out.findings.iter().map(|f| f.columns.len()).sum::<usize>();
        for path in Rc::unwrap_or_clone(paths) {
            let p = sdk::RelativePath(path.into());
            if !named_reader && unread_path(linked, &p) {
                out.unread.push(unread(p, sdk::Unreadable::NoLanguage));
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
            out.unread.extend(reasons.into_iter().map(|reason| {
                unread(sdk::RelativePath(path.clone()), sdk::Unreadable::Failed(reason))
            }));
        }
        out.unread.sort_by(|left, right| left.path.cmp(&right.path));
        if let Some((memo_pool, memo)) = memo && small_report(&out) {
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

fn render_chunk(entries: &mut [(&sdk::RuleId, super::Receipt<'_>)]) -> Result<(sdk::LintReport, usize), sdk::LintError> {
    let mut result = report(Vec::new(), Vec::new(), Vec::new());
    let mut dictionaries = Dictionaries::new();
    for (id, receipt) in entries {
        render_receipt::<true>(id, std::mem::take(receipt), &mut result, Some(&mut dictionaries))?;
    }
    Ok((result, dictionaries.len()))
}

fn render_receipt<'a, const SHARED: bool>(
    id: &sdk::RuleId,
    receipt: super::Receipt<'a>,
    out: &mut sdk::LintReport,
    mut dictionaries: Option<&mut Dictionaries<'a>>,
) -> Result<(), sdk::LintError> {
    out.skipped.extend(receipt.rejected_syntax.into_iter().map(|path| sdk::Skipped {
        rule: id.clone(), path: sdk::RelativePath(path.as_str().into()), reason: sdk::SkipReason::SyntaxRejected,
    }));
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
        let (&subject, keys) = row.layout.identity.split_first()
            .ok_or_else(|| execution("missing row subject".into()))?;
        let columns: Vec<_> = row.layout.names.iter().zip((0..row.width()).map(value))
            .map(|(name, cell)| sdk::FindingColumn {
                name: sdk::ColumnName(name.as_str().into()), value: worker_slot(&cell),
            }).collect();
        let key = sdk::FindingKey(keys.iter().map(|&i| columns[i].value.clone()).collect());
        out.findings.push(finding(id.clone(), columns[subject].value.clone(), key, sdk::Place::Missing, columns));
    }
    for (path, message) in receipt.unknowns {
        out.unknowns.push(unknown(id.clone(), sdk::RelativePath(path.as_str().into()), sdk::Message(message.as_str().into())));
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
    rounds: Vec<usize>,
    empty_reader_epochs: usize,
    bulk_screen_files: usize,
    project_counts: [usize; 4],
    calls: usize,
    requests: usize,
    refused: usize,
    reasons: BTreeMap<String, usize>,
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
    certified: usize,
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
    rounds: Vec::new(),
    empty_reader_epochs: 0,
    bulk_screen_files: 0,
    project_counts: [0; 4],
    calls: 0,
    requests: 0,
    refused: 0,
    reasons: BTreeMap::new(),
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
    certified: 0,
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
    if m.rounds.iter().any(|&size| size > 1) {
        eprintln!("TM_ROUND_COUNTS epochs={} max_width={} closed_only={} bulk_screen_files={}",
            m.rounds.len(), m.rounds.iter().max().unwrap(), m.empty_reader_epochs, m.bulk_screen_files);
    }
    let [versions,hits,evictions,peak_charge]=m.retained;
    eprintln!("FILE_CACHE versions={versions} hits={hits} evictions={evictions} peak_charge={peak_charge}");
    eprintln!(
        "LIVE_PROJECT_WALL provider={:.6} numbering={:.6} numbered={} empty_epochs={}",
        m.project_provider_wall, m.project_number_wall, m.project_numbered, m.empty_project_epochs
    );
    let [prepare, read, first, second, project, assembly] = m.parts;
    let total = (WORK_NANOS.load(Ordering::Relaxed) + DISPATCH_NANOS.load(Ordering::Relaxed)) as f64 / 1e9;
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
        "CHECK_SESSION live calls={} requests={} failed_calls={} programs={plans} bindings={bindings} sources={sources} reads={reads} parses={parses} distinct_pairs={pairs} skipped={skips} evaluated={eval} productive={productive} empty={empty} refused={refused} prep={prepare:.6} read_parse={read:.6} first={first:.6} second={second:.6} project={project:.6} assembly={assembly:.6} admission={:.6} bound_proofs={:.6} literal_positions={} certified_pairs={} requirement_programs={} requirement_bindings={}",
        m.calls,
        m.requests,
        m.refused,
        total + m.worker_cpu - m.parts.iter().sum::<f64>(),
        m.bound_proofs,
        m.literal_positions,
        m.certified,
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
