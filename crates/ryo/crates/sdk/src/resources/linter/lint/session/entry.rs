use super::*;

thread_local! {
    static SESSIONS: std::cell::RefCell<HashMap<usize, Live>> = Default::default();
}

pub(crate) fn release_session(id: usize) {
    SESSIONS.with(|sessions| drop(sessions.borrow_mut().remove(&id)));
}

pub(super) fn execution(reason: String) -> sdk::LintError {
    sdk::LintError::Execution { reason }
}

/// Answers one request as rows: every row its queries emit, the first `limit` of them rendered and all of them counted.
pub(crate) fn execute_rows(
    id: usize,
    request: sdk::LintRequest,
    languages: &sdk::Languages,
    workers: std::num::NonZeroUsize,
    limit: usize,
) -> Reply {
    let mut session = SESSIONS.with(|sessions| sessions.borrow_mut().remove(&id)).unwrap_or_default();
    let answer = if let Err(reason) = session.link_readers(languages).and_then(|()| session.use_cache(None)) {
        Err(execution(reason))
    } else {
        session.batch.workers = Some(workers);
        session.rows = Some(limit);
        let mut answered = session.lint(vec![request], languages);
        session.rows = None;
        answered.pop().unwrap_or_else(|| Err(execution("the executor answered no request".into())))
    };
    drop(SESSIONS.with(|sessions| sessions.borrow_mut().insert(id, session)));
    answer
}

pub(crate) fn execute_round(
    id: usize,
    requests: Vec<sdk::LintRequest>,
    languages: &sdk::Languages,
    cache: &sdk::Cache,
    workers: std::num::NonZeroUsize,
) -> Vec<Reply> {
    // The session is taken out of the map for the round, so that a request made while it runs (a host that lints inside a lint, on
    // another linter) finds the map free: a session is never borrowed while it works.
    let mut session = SESSIONS.with(|sessions| sessions.borrow_mut().remove(&id)).unwrap_or_default();
    let directory = match cache {
        sdk::Cache::At(path) => Some(path.as_path()),
        _ => None,
    };
    let answers = if let Err(reason) = session.link_readers(languages).and_then(|()| session.use_cache(directory)) {
        requests.into_iter().map(|_| Err(execution(reason.clone()))).collect()
    } else {
        session.batch.workers = Some(workers);
        session.lint(requests, languages)
    };
    // Everything the cache is to keep is on disk when the call returns: a host may end its process without running destructors.
    session.publish();
    drop(SESSIONS.with(|sessions| sessions.borrow_mut().insert(id, session)));
    answers
}

// This cache accepts only immutable, parameterless requests with a pure report.
// The version already identifies the normalized input bytes for Given authority.
#[derive(PartialEq)]
struct CallKey {
    version: Option<Arc<str>>,
    inventory: usize,
    filter: usize,
    tables: Vec<usize>,
    rules: Vec<(sdk::RuleId, ryo_executor::check_session::query::Query, usize, usize)>,
}

enum RoundRequest {
    Complete(sdk::LintReport),
    Scheduled(Pending),
}

type WorkEntry = (
    sdk::RuleId,
    Result<replay::stored::Key, (usize, Option<replay::stored::Key>)>,
);

struct Pending {
    ids: Vec<WorkEntry>,
    named_reader: bool,
    paths: Rc<Vec<String>>,
    projects: std::collections::BTreeSet<usize>,
    restored_unread: BTreeMap<(String, String), usize>,
    memo: Option<(usize, CallKey)>,
}

struct Live {
    /// Set while a round answers its query as rows: how many of them are rendered.
    rows: Option<usize>,
    batch: Batch,
    store:Option<replay::stored::Store>,
    directory:Option<std::path::PathBuf>,
    state: Active,
    languages: Vec<Linked>,
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
    observation_epoch: Arc<std::sync::atomic::AtomicUsize>,
    environments: HashMap<Arc<str>, Arc<ryo_executor::check_session::project::Environment>>,
    versions: HashMap<(Authority, BTreeMap<String, (Arc<[u8]>, ryo_executor::check_session::Permission)>, Option<Arc<str>>), Arc<str>>,
}

impl Live {
    /// Publishes what the session keeps for the next run: its store, when it has one.
    fn publish(&mut self){
        if let Some(store)=&mut self.store { store.costs(); if let Err(e)=store.publish(){eprintln!("publish completed session: {e}");} }
    }

    /// What a finished session writes: its alias counts (the images' tools read them) and its store.
    fn finish(&mut self){
        #[cfg(feature = "bench-hooks")]
        {
            let (writes, pages) = self.state.aliases.counts();
            eprintln!("PAIR_ALIASES pairs={} aliases={writes} allocated_pages={pages}", self.batch.schedule.pairs);
        }
        self.publish();
    }
}
impl Drop for Live {
    fn drop(&mut self){ self.finish(); }
}
impl Default for Live {
    fn default() -> Self {
        MEASURED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .sessions += 1;
        let state: Active = Active::default();
        Self {
            rows: None,
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
            observation_epoch: Default::default(),
            environments: HashMap::new(),
            versions: HashMap::new(),
        }
    }
}

/// The failure a run's refusal is answered with: a language that wrote output outside its file is `Provider` with the fault, any other
/// refusal is `Unrunnable` with its sentence.
fn run_error(id: sdk::RuleId, refusal: ryo_executor::check_session::refusal::Refusal, languages: &[Linked]) -> sdk::LintError {
    use ryo_executor::check_session::refusal::{ProviderFault, Refusal};
    let Refusal::Provider { provider, fault, .. } = refusal else {
        return sdk::LintError::Unrunnable { id, reason: refusal.to_string() };
    };
    // The provider is named where the refusal was made; where it was not, the one language linked is the one that wrote the output.
    let provider = if provider.is_empty() {
        match languages {
            [only] => ryo_executor::check_session::source::Source::name(only).to_owned(),
            _ => provider,
        }
    } else {
        provider
    };
    sdk::LintError::Provider {
        provider: sdk::ProviderName(provider.into()),
        code: match fault {
            ProviderFault::SpanOutsideFile => sdk::ProviderCode::SpanOutsideFile,
            ProviderFault::NodeOutsideTable => sdk::ProviderCode::NodeOutsideTable,
            ProviderFault::NodeOfAnotherFile => sdk::ProviderCode::NodeOfAnotherFile,
            ProviderFault::FileNotVisited => sdk::ProviderCode::FileNotVisited,
        },
    }
}

fn rule_error(
    id: sdk::RuleId,
    query: crate::resources::query::RyoQlQuery,
    linked: &sdk::Languages,
    hosted: &[Hosted],
    refusal: ryo_executor::check_session::refusal::Refusal,
) -> sdk::LintError {
    use ryo_executor::check_session::refusal::Refusal;
    let reason = match refusal {
        Refusal::MissingParameter { .. } => return sdk::LintError::MissingParameter { id },
        Refusal::UnknownKind { .. } => return sdk::LintError::Unrunnable { id, reason: refusal.to_string() },
        other => other.to_string(),
    };
    use crate::resources::query::{ValidateError, ValidateRequest, validate};

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
    })
}

fn parameters(
    input: BTreeMap<sdk::ParameterName, sdk::ParameterValue>,
) -> Result<Parameters, String> {
    input.into_iter().map(|(k, v)| Ok((k.0, parameter(v)?))).collect()
}

fn worker_slot(v: &Option<Cell<Arc<str>>>) -> sdk::Slot {
    let Some(c) = v else {
        return sdk::Slot::Nothing;
    };
    use crate::resources::languages::{Offset, Span};
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
    linked.reads(path) != crate::resources::languages::Recognition::Reads
        && !linked.0.iter().any(|reader| {
            reader.primary_role(crate::resources::languages::SourcePath(&path.0))
                == ryo_language::domain::files::types::PrimaryFileRole::Configuration
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
    fn link_readers(&mut self, linked: &sdk::Languages) -> Result<(), String> {
        if self.languages.is_empty() {
            let readers = natives(linked.0.iter().cloned());
            self.batch.file_catalog = ryo_executor::check_session::named::Catalog::from_readers(&readers)?;
            self.languages = readers;
        }
        Ok(())
    }

    fn use_cache(&mut self,directory:Option<&std::path::Path>)->Result<(),String>{
        if self.directory.as_deref()!=directory {
            if let Some(old)=&mut self.store {let _ = old.publish().map_err(|e|e.to_string())?;}
            self.store=directory.map(|p|replay::stored::Store::directory(p.to_owned(), &self.languages)).transpose().map_err(|e|e.to_string())?;
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
        drop(self.filters.insert(names.to_vec(), set.clone()));
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
        self.epoch(requests, linked)
    }

    fn epoch(&mut self, requests: Vec<sdk::LintRequest>, linked: &sdk::Languages) -> Vec<Reply> {
        let _ = self.observation_epoch.fetch_add(1, Ordering::Relaxed);
        let previous_children = self.child_cpu();
        let previous = self.phase_totals();
        
        let wall = Instant::now();
        let admission = Instant::now();
        let count = requests.iter().map(|request| request.rules.len()).sum::<usize>();
        let pending = requests.into_iter().map(|request| {
            let start = self.batch.work.len();
            let result = self.admit(request, linked, true);
            if result.is_err() {
                self.batch.work.truncate(start);
                if let Some(store) = &mut self.store { store.truncate_work(start); }
            }
            result
        }).collect::<Vec<_>>();
        self.admission_wall += admission.elapsed().as_secs_f64();
        let epoch = Instant::now();
        let (runs_before, nodes_before) = (self.batch.clocks.node_runs, self.batch.clocks.nodes_made);
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
        // The node counters of the round, for a subscriber that listens: how many node runs the visits made.
        let (runs, nodes) = (self.batch.clocks.node_runs - runs_before, self.batch.clocks.nodes_made - nodes_before);
        tracing::event!(name: "run", target: "ryo::node_counters", tracing::Level::INFO, runs = runs, nodes = nodes);
        self.batch.work.clear();
        if let Err(error) = ryo_executor::finish::operators::flush() {
            eprintln!("operator ledger: {error}");
        }
        let mut last = MEASURED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let c = &self.batch.clocks;
        last.calls += results.len();
        last.rounds.push(results.len());
        last.empty_reader_epochs = self.batch.empty_reader_epochs;
        last.bulk_screen_files = c.bulk_screen_files;
        last.old_route = c.readers.iter().map(|(name, parts)| (name.clone(), parts.old_route)).collect();
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
        last.witnesses = c.witnesses;
        last.named_counts = c.named_counts;
        last.exact_inputs = c.exact_inputs;
        last.named_cpu = c.named_cpu;
        last.certified = c.certified;
        last.empty = c.empty;
        last.requirement_programs = c.requirement_programs;
        last.requirement_bindings = c.requirement_bindings;
        last.visit_projects = c.visit_projects;
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

    /// The version of one request: its id, the environment its files that need one take and its listed files. The environment
    /// belongs to the version and exists as soon as a file of the version takes it, so a later request that lists such a file
    /// on a version that has none creates it there. Which files take it is `takes_environment`, which sits next to the
    /// recognizers the scheduler routes by, so the two change together.
    fn admit_version<L: ryo_executor::check_session::source::Source>(
        &mut self,
        languages: &[L],
        key: &mut (Authority, BTreeMap<String, (Arc<[u8]>, ryo_executor::check_session::Permission)>, Option<Arc<str>>),
        listed: &[sdk::RelativePath],
        physical: &Arc<str>,
        version_scope: Option<[u8; 32]>,
        supplied_versions: &BTreeMap<String, [u8; 32]>,
    ) -> Result<Arc<str>, sdk::LintError> {
        use ryo_executor::check_session::source::takes_environment;
        let reader = key.2.clone();
        let reader_name = reader.as_deref();
        let new_environment = |id: &Arc<str>, authority: &Authority, supplied: &BTreeMap<String, (Arc<[u8]>, ryo_executor::check_session::Permission)>, epoch: &Arc<std::sync::atomic::AtomicUsize>| {
            let mut environment = ryo_executor::check_session::project::Environment::new(
                id.clone(), authority.clone(), supplied.keys().cloned().collect(), epoch.clone(),
            );
            environment.version_scope = version_scope;
            Arc::new(environment)
        };
        let listed_takes = || listed.iter().any(|p| takes_environment(languages, reader_name, &p.0));
        let root = if let Some(id) = self.versions.get(&*key).cloned() {
            for (path, (bytes, _)) in &key.1 {
                self.batch.schedule.reprovide(&id, path, bytes);
            }
            if !self.environments.contains_key(&id) && listed_takes() {
                let environment = new_environment(&id, &key.0, &key.1, &self.observation_epoch);
                drop(self.environments.insert(id.clone(), environment));
            }
            id
        } else {
            let id: Arc<str> = Arc::from(format!("check-input:{}", self.versions.len()));
            let environment = (key.1.keys().any(|path| takes_environment(languages, reader_name, path)) || listed_takes())
                .then(|| new_environment(&id, &key.0, &key.1, &self.observation_epoch));
            if let Some(project) = &environment { drop(self.environments.insert(id.clone(), project.clone())); }
            for (path, (bytes, permission)) in &mut key.1 {
                let own = environment.clone().filter(|_| takes_environment(languages, reader_name, path));
                let (slot, shared) = self.batch.schedule.provide(&id, path, bytes, *permission, &reader, &own).map_err(|refusal| execution(refusal.to_string()))?;
                if let Some(store) = &mut self.store {
                    store.bind_version(slot, supplied_versions[path]);
                }
                // The scheduler already owns the exact path and bytes. Retain
                // its allocation in the version key and drop the duplicate.
                if !Arc::ptr_eq(bytes, &shared) {
                    self.shared_overlay_files += 1;
                    self.shared_overlay_bytes += bytes.len();
                    *bytes = shared;
                }
            }
            drop(self.versions.insert(key.clone(), id.clone()));
            id
        };
        for p in listed {
            if !key.1.contains_key(p.0.as_ref()) {
                let own = self.environments.get(&root).cloned().filter(|_| takes_environment(languages, reader_name, &p.0));
                self.batch.schedule.listed(&root, physical, &p.0, &reader, &own).map_err(|refusal| execution(refusal.to_string()))?;
            }
        }
        Ok(root)
    }

    fn admit(&mut self, req: sdk::LintRequest, linked: &sdk::Languages, unbounded: bool) -> Result<RoundRequest, sdk::LintError> {
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
        let mut overlay = BTreeMap::new();
        for source in given.into_iter().chain(req.beside) {
            use ryo_executor::check_session::Permission;
            let permission = match source.executable {
                sdk::ExecutablePermission::Unknown => Permission::Unknown,
                sdk::ExecutablePermission::Unsupported => Permission::Unsupported,
                sdk::ExecutablePermission::PosixExecutable => Permission::PosixExecutable,
                sdk::ExecutablePermission::PosixNotExecutable => Permission::PosixNotExecutable,
            };
            drop(overlay.insert(source.path.0.into_string(), (Arc::<[u8]>::from(source.text.0.as_bytes()), permission)));
        }
        let (inventory, paths) = self.cache.inventory(listed.iter().map(|p| p.0.as_ref())
            .chain(overlay.keys().map(String::as_str)).collect());
        let reader: Option<Arc<str>> = match req.source_language {
            sdk::SourceLanguage::Recognized => None,
            sdk::SourceLanguage::Named(name) => {
                if linked.0.iter().filter(|l| l.name() == &name).count() != 1 {
                    return Err(sdk::LintError::SourceLanguage { name: name.0.to_string() });
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
                rules.push((id.clone(), sdk_request::query(&mut self.cache, rule, inputs.get(id), self.rows.is_some()),
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
        let mut supplied_versions = BTreeMap::new();
        let scope = if self.store.is_some() {
            // The host supplies the inventory and relations in memory. Reuse
            // the prepared program, selection and table identities across calls.
            let mut peers = Vec::with_capacity(req.rules.len());
            for (id, rule) in &req.rules {
                let parameters = rule.parameters.iter()
                    .map(|(k, v)| Ok((k.0.clone(), parameter(v.clone())?)))
                    .collect::<Result<Parameters, String>>().map_err(execution)?;
                let query = sdk_request::query(&mut self.cache, rule, inputs.get(id), self.rows.is_some());
                let digest = replay::stored::key((&query, &parameters));
                peers.push(replay::stored::key((id.as_ref(), digest, &rule.include, &rule.exclude)));
                retained_rules.push((query, parameters, digest));
            }
            let shape = hosted.iter().map(|h| (&h.name, &h.columns)).collect::<Vec<_>>();
            let supplied: BTreeMap<_, _> = key.1.iter().map(|(path, (bytes, permission))| {
                let digest = *blake3::hash(bytes).as_bytes();
                let _ = supplied_versions.insert(path.clone(), replay::stored::key((path, &reader, permission, digest)));
                (path.clone(), replay::stored::key((digest, permission)))
            }).collect();
            let request_scope = replay::stored::key((&physical, &authority, &reader, &supplied));
            let proof = self.store.as_mut().unwrap().request_inputs(
                std::path::Path::new(&*physical),
                self.cache.path_digest(&paths),
                replay::stored::key((peers, shape, &reader)),
                supplied,
            );
            Some((proof, request_scope))
        } else {
            None
        };
        let mut retained_rules = retained_rules.into_iter();
        let mut restored_unread=BTreeMap::<(String, String),usize>::new();
        let mut projects = std::collections::BTreeSet::new();
        if self.store.is_none() {
            // The request's rules are lowered together once; each admission below then finds its program prepared.
            let items = req.rules.iter().filter_map(|(id, rule)| {
                let query = sdk_request::query(&mut self.cache, rule, inputs.get(id), self.rows.is_some());
                Some((query, parameters(rule.parameters.clone()).ok()?))
            }).collect();
            let _compile = tracing::info_span!("compile").entered();
            self.batch.prepare_rules(items);
        }
        let mut rules = req.rules.into_iter();
        let mut prepared = Vec::new();
        let workers = self.batch.workers.map_or(1, std::num::NonZeroUsize::get);
        if self.store.is_none() && workers > 1 && rules.len() >= 128 {
            prepared.extend(rules.by_ref().map(|(id, rule)| (id, rule, None)));
            let chunk_size = prepared.len().div_ceil(workers);
            self.admission_shards += prepared.len().div_ceil(chunk_size);
            let chunks: Vec<_> = prepared.chunks_mut(chunk_size).collect();
            let (_, cpu) = ryo_executor::check_session::jobs::run(chunks, |chunk| {
                for (_, rule, typed) in chunk {
                    *typed = Some(parameters(std::mem::take(&mut rule.parameters)));
                }
            });
            self.admission_workers_cpu += cpu;
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
            let (query, parameters, contract) = if let Some((_, scope)) = scope {
                let (query, parameters, digest) =
                    retained_rules.next().expect("retained rule identity");
                let store = self.store.as_mut().expect("retained request");
                if !hosted.is_empty() && store.program_inputs(digest).is_none() {
                    let program = self.batch.prepared_rule(query.clone(), &parameters);
                    if let Ok(plan) = &self.batch.schedule.programs[program] {
                        store.remember_program_inputs(digest, &plan.bases);
                    }
                }
                let inputs = store.program_inputs(digest).unwrap_or_default();
                let consumed: Vec<_> = hosted.iter().zip(host_keys.as_deref().unwrap_or_default())
                    .filter(|(table, _)| inputs.contains(&table.name))
                    .map(|(_, key)| *key)
                    .collect();
                let identity = replay::stored::key((
                    scope, digest, self.cache.path_digest(&files), consumed,
                ));
                (query, parameters, Some(identity))
            } else {
                let parameters = typed.unwrap_or_else(|| parameters(std::mem::take(&mut rule.parameters))).map_err(execution)?;
                (sdk_request::query(&mut self.cache, &rule, inputs.get(&id), self.rows.is_some()), parameters, None)
            };
            if let (Some(store),Some(k),Some((proof, _)))=(&mut self.store,contract,scope)
                && let Some((packet,unread))=store.lookup_in(proof,k){
                let mut counts=BTreeMap::<(String, String),usize>::new();
                for p in unread{*counts.entry(p.clone()).or_default()+=1;}
                if let Some(reason)=&store.packet(&packet).refusal{return Err(rule_error(id, rule.query, linked, &hosted, reason.clone()))}
                for (p,n) in counts{let total=restored_unread.entry(p).or_default();*total=(*total).max(n);}
                ids.push((id,Ok(packet)));
                continue
            }
            if prepared_root.is_none(){
        let languages = std::mem::take(&mut self.languages);
        let root = self.admit_version(&languages, &mut key, &listed, &physical, scope.map(|(_, request_scope)| request_scope), &supplied_versions);
        self.languages = languages;
        let root = root?;
                prepared_root=Some(root);
            }
            let root=prepared_root.as_ref().unwrap();
            // A word for a kind no linked language has is refused before any file is read, as the old engine refused it.
            let program = self.batch.prepared_rule(query.clone(), &parameters);
            if let Ok(program) = &self.batch.schedule.programs[program]
                && let Some(refusal) = program.refuse_unknown_kind(|| self.languages.iter().map(|language| (ryo_executor::check_session::source::Source::name(language).to_owned(), ryo_executor::check_session::source::Source::kinds(language).into_iter().map(str::to_owned).collect())).collect())
            {
                return Err(rule_error(id, rule.query, linked, &hosted, refusal));
            }
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
                    |_| Ok(None),
                    Some(LiveAdmission { version: root, selection, host_tables: &table_ids }),
                    // The retained store must key these pairs by the layout's
                    // physical observations before they can use the file path.
                    self.store.is_none().then(|| (
                        self.languages.as_slice(),
                        self.environments.get(root).cloned(),
                    )),
                )
                .map_err(|reason| rule_error(id.clone(), rule.query.clone(), linked, &hosted, reason.into()))?;
            if let replay::Work::Invalid(reason)=&work{
                if let (Some(store),Some(k))=(&mut self.store,contract){store.decline(k,reason);}
                return Err(rule_error(id, rule.query, linked, &hosted, reason.clone()))
            }
            if let replay::Work::Closed { project: Some(id), .. } = &work {
                let _ = projects.insert(*id);
            }
            if let (Some(store), Some((proof, _))) = (&mut self.store, scope) {
                store.bind_work(self.batch.work.len(), proof);
            }
            let index = self.batch.add_work(work, self.store.as_mut().zip(contract));
            ids.push((id,Err((index,contract))));
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
        if self.store.is_none() && self.rows.is_none() && count > 1 && ids.len() >= 128 {
            let mut receipts = ids.iter().map(|(id, entry)| {
                let (i, _) = entry.as_ref().err().expect("fresh receipt");
                let receipt = Batch::answer(&self.batch.work[*i], &self.state.answers)
                    .map_err(|e| run_error(id.clone(), e.into_owned(), &self.languages))?;
                Ok::<_, sdk::LintError>((id, receipt))
            }).collect::<Result<Vec<_>, _>>()?;
            let chunk_size = receipts.len().div_ceil(count);
            let chunks: Vec<_> = receipts.chunks_mut(chunk_size).collect();
            let (parts, child_cpu) = ryo_executor::check_session::jobs::run(chunks, render_chunk);
            let mut dictionaries = 0;
            for part in parts {
                let (part, count) = part?;
                dictionaries += count;
                out.skipped.extend(part.skipped);
                out.rows_total += part.rows_total;
                out.findings.extend(part.findings);
                out.unknowns.extend(part.unknowns);
            }
            drop(receipts);
            for (_, entry) in ids {
                let (i, _) = entry.expect_err("fresh receipt");
                self.batch.work[i] = replay::Work::Invalid(String::new().into());
            }
            self.parallel_rows += out.findings.len();
            self.delivery_dictionaries += dictionaries;
            self.batch.clocks.delivery_workers_cpu += child_cpu;
        } else {
            for (id,entry) in ids {
                let consumed = entry.as_ref().err().map(|(i, _)| *i);
                let held;
                let receipt=match entry {
                    Ok(k)=>{
                        held=self.store.as_ref().unwrap().packet(&k).report();
                        replay::Receipt {rejected_syntax:held.rejected_syntax.iter().collect(),rows:held.rows.iter().collect(),unknowns:held.unknowns.iter().collect(),unread:held.unread.iter().collect()}
                    }
                    Err((i,k))=>{
                        let r=Batch::answer(&self.batch.work[i],&self.state.answers).map_err(|e| run_error(id.clone(), e.into_owned(), &self.languages))?;
                        if let (Some(store),Some(k))=(&mut self.store,k){let _ = store.save(k,i,&r);}
                        r
                    }
                };
                render_receipt::<false>(&id, receipt, &mut out, None, self.rows)?;
                if let Some(i) = consumed {
                    // Epoch evaluation and any retained proof are complete.
                    // Drop this request's source lists or closed rows at delivery.
                    self.batch.work[i] = replay::Work::Invalid(String::new().into());
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
            if let Some(unread) = self.state.project_unread.get(&project) {
                for refused in unread {
                    manifest.entry(refused.path.0.clone()).or_default()
                        .push(sdk::ReadFailure(refused.reason.0.clone()));
                }
            }
        }
        for ((path, message), count) in restored_unread {
            let reasons = manifest.entry(path.into()).or_default();
            let reason = sdk::ReadFailure(message.into());
            let present = reasons.iter().filter(|existing| **existing == reason).count();
            reasons.extend(std::iter::repeat_n(reason, count.saturating_sub(present)));
        }
        for (path, mut reasons) in manifest {
            reasons.sort_unstable();
            out.unread.extend(reasons.into_iter().map(|reason| {
                unread(sdk::RelativePath(path.clone()), sdk::Unreadable::Failed(reason))
            }));
        }
        // A file named unread for its failed read is not also named for having no language: the reason it was not read is the failure.
        let failed: std::collections::HashSet<sdk::RelativePath> = out.unread.iter().filter(|each| matches!(each.reason, sdk::Unreadable::Failed(_))).map(|each| each.path.clone()).collect();
        out.unread.retain(|each| !(matches!(each.reason, sdk::Unreadable::NoLanguage) && failed.contains(&each.path)));
        out.unread.sort_by(|left, right| left.path.cmp(&right.path));
        out.unread.dedup();
        if let Some((memo_pool, memo)) = memo && small_report(&out) {
            let entries = &mut self.completed_calls[memo_pool];
            if entries.len() == 128 {
                drop(entries.pop_front());
            }
            entries.push_back((memo, out.clone()));
        }
        self.batch.clocks.assemble += thread_cpu() - cpu;
        self.delivery_wall += deliver.elapsed().as_secs_f64();
        Ok(out)
    }
}

type Dictionaries<'a> =
    HashMap<usize, std::sync::RwLockReadGuard<'a, ryo_executor::finish::answer::Words>>;

fn render_chunk(entries: &mut [(&sdk::RuleId, replay::Receipt<'_>)]) -> Result<(sdk::LintReport, usize), sdk::LintError> {
    let mut result = report(Vec::new(), Vec::new(), Vec::new());
    let mut dictionaries = Dictionaries::new();
    for (id, receipt) in entries {
        render_receipt::<true>(id, std::mem::take(receipt), &mut result, Some(&mut dictionaries), None)?;
    }
    Ok((result, dictionaries.len()))
}

fn render_receipt<'a, const SHARED: bool>(
    id: &sdk::RuleId,
    receipt: replay::Receipt<'a>,
    out: &mut sdk::LintReport,
    mut dictionaries: Option<&mut Dictionaries<'a>>,
    limit: Option<usize>,
) -> Result<(), sdk::LintError> {
    out.skipped.extend(receipt.rejected_syntax.into_iter().map(|path| sdk::Skipped {
        rule: id.clone(), path: sdk::RelativePath(path.as_str().into()), reason: sdk::SkipReason::SyntaxRejected,
    }));
    for row in receipt.rows {
        out.rows_total += 1;
        if limit.is_some_and(|limit| out.findings.len() >= limit) {
            continue;
        }
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
        let place = match row.place {
            (0, _) | (_, 0) => sdk::Place::Missing,
            (line, column) => sdk::Place::At { line: sdk::Line(line), column: sdk::Column(column) },
        };
        out.findings.push(finding(id.clone(), columns[subject].value.clone(), key, place, columns));
    }
    for (path, message) in receipt.unknowns {
        out.unknowns.push(unknown(id.clone(), sdk::RelativePath(path.as_str().into()), sdk::Message(message.as_str().into())));
    }
    // A file the rule needed and the reader could not read is unread, with why: the rule's other files are answered.
    for (path, reason) in receipt.unread {
        out.unread.push(unread(sdk::RelativePath(path.as_str().into()), sdk::Unreadable::Failed(sdk::ReadFailure(reason.as_str().into()))));
    }
    Ok(())
}

#[cfg(test)]
mod version_environment_tests {
    use super::{Authority, BTreeMap, Live, sdk};
    use super::linked::Fake;
    use std::sync::Arc;

    fn path(text: &str) -> sdk::RelativePath { sdk::RelativePath(text.into()) }

    #[test]
    fn a_later_request_on_the_same_overlay_that_lists_a_project_file_gives_it_the_environment() {
        let languages = [
            Fake { name: "project", extension: ".pyx", project: true },
            Fake { name: "plain", extension: ".rs", project: false },
        ];
        let physical: Arc<str> = Arc::from("/repository");
        let mut live = Live::default();
        let mut key = (Authority::Disk(physical.to_string()), BTreeMap::new(), None);
        let first = live.admit_version(&languages, &mut key, &[path("x.rs")], &physical, None, &BTreeMap::new()).expect("first request");
        assert!(!live.environments.contains_key(&first), "no file of the first request takes an environment");
        let second = live.admit_version(&languages, &mut key, &[path("x.rs"), path("a.pyx")], &physical, None, &BTreeMap::new()).expect("second request");
        assert_eq!(first, second, "the same overlay is the same version");
        assert!(live.environments.contains_key(&second), "the second request makes the environment on the version it reuses");
        let held = |name: &str| live.batch.schedule.sources.iter().filter(|source| source.path == name).map(|source| source.environment.is_some()).collect::<Vec<_>>();
        assert_eq!(held("a.pyx"), [true], "the project file takes it");
        assert_eq!(held("x.rs"), [false], "the other file keeps the shared path");
    }
}
