thread_local! {
    static SESSIONS: std::cell::RefCell<HashMap<usize, Live>> = Default::default();
}

fn release_session(id: usize) {
    SESSIONS.with(|sessions| drop(sessions.borrow_mut().remove(&id)));
}

fn execution(reason: String) -> sdk::LintError {
    sdk::LintError::Execution { reason }
}

fn execute_round(
    id: usize,
    requests: Vec<sdk::LintRequest>,
    languages: &sdk::Languages,
    cache: &sdk::Cache,
    workers: std::num::NonZeroUsize,
) -> Vec<Reply> {
    SESSIONS.with(|sessions| {
        let mut sessions = sessions.borrow_mut();
        let session = sessions.entry(id).or_default();
        let directory = match cache {
            sdk::Cache::At(path) => Some(path.as_path()),
            _ => None,
        };
        if let Err(reason) = session.link_readers(languages).and_then(|()| session.use_cache(directory)) {
            return requests.into_iter().map(|_| Err(execution(reason.clone()))).collect();
        }
        session.batch.workers = Some(workers);
        session.lint(requests, languages)
    })
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
    restored_unread: BTreeMap<(String, String), usize>,
    memo: Option<(usize, CallKey)>,
}

struct Live {
    batch: Batch,
    store:Option<super::stored::Store>,
    directory:Option<std::path::PathBuf>,
    state: Active,
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
    observation_epoch: Arc<std::sync::atomic::AtomicUsize>,
    environments: HashMap<Arc<str>, Arc<crate::check_session::project::Environment>>,
    versions: HashMap<(Authority, BTreeMap<String, (Arc<[u8]>, crate::check_session::Permission)>, Option<Arc<str>>), Arc<str>>,
}

impl Drop for Live {
    fn drop(&mut self){
        let (writes, pages) = self.state.aliases.counts();
        eprintln!("PAIR_ALIASES pairs={} aliases={writes} allocated_pages={pages}", self.batch.schedule.pairs);
        if let Some(store)=&mut self.store { store.costs(); if let Err(e)=store.publish(){eprintln!("publish completed session: {e}");} }
    }
}
impl Default for Live {
    fn default() -> Self {
        MEASURED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .sessions += 1;
        let state: Active = Active::default();
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
            observation_epoch: Default::default(),
            environments: HashMap::new(),
            versions: HashMap::new(),
        }
    }
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
    fn link_readers(&mut self, linked: &sdk::Languages) -> Result<(), String> {
        if self.languages.is_empty() {
            let readers = linked.0.iter().cloned().map(Pinned::from).collect::<Vec<_>>();
            self.batch.file_catalog = crate::check_session::named::Catalog::from_readers(&readers)?;
            self.languages = readers;
        }
        Ok(())
    }

    fn use_cache(&mut self,directory:Option<&std::path::Path>)->Result<(),String>{
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
        self.epoch(requests, linked)
    }

    fn epoch(&mut self, requests: Vec<sdk::LintRequest>, linked: &sdk::Languages) -> Vec<Reply> {
        self.observation_epoch.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let previous_children = self.child_cpu();
        let previous = self.phase_totals();
        let options = check_options();
        let wall = Instant::now();
        let admission = Instant::now();
        let count = requests.iter().map(|request| request.rules.len()).sum::<usize>();
        let pending = requests.into_iter().map(|request| {
            let start = self.batch.work.len();
            let result = self.admit(request, linked, options.reverse_files, true);
            if result.is_err() {
                self.batch.work.truncate(start);
                if let Some(store) = &mut self.store { store.truncate_work(start); }
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
        if let Err(error) = crate::finish::operators::flush() {
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
        let mut supplied_versions = BTreeMap::new();
        let scope = if self.store.is_some() {
            // The host supplies the inventory and relations in memory. Reuse
            // the prepared program, selection and table identities across calls.
            let mut peers = Vec::with_capacity(req.rules.len());
            for (id, rule) in &req.rules {
                let parameters = rule.parameters.iter()
                    .map(|(k, v)| Ok((k.0.clone(), parameter(v.clone())?)))
                    .collect::<Result<Parameters, String>>().map_err(execution)?;
                let query = sdk_request::query(&mut self.cache, rule, inputs.get(id));
                let digest = super::stored::key((&query, &parameters));
                peers.push(super::stored::key((id.as_ref(), digest, &rule.include, &rule.exclude)));
                retained_rules.push((query, parameters, digest));
            }
            let shape = hosted.iter().map(|h| (&h.name, &h.columns)).collect::<Vec<_>>();
            let supplied: BTreeMap<_, _> = key.1.iter().map(|(path, (bytes, permission))| {
                let digest = *blake3::hash(bytes).as_bytes();
                supplied_versions.insert(path.clone(), super::stored::key((path, &reader, permission, digest)));
                (path.clone(), super::stored::key((digest, permission)))
            }).collect();
            let request_scope = super::stored::key((&physical, &authority, &reader, &supplied));
            let proof = self.store.as_mut().unwrap().request_inputs(
                std::path::Path::new(&*physical),
                self.cache.path_digest(&paths),
                super::stored::key((peers, shape, &reader)),
                supplied,
            );
            Some((proof, request_scope))
        } else {
            None
        };
        let mut retained_rules = retained_rules.into_iter();
        let mut restored_unread=BTreeMap::<(String, String),usize>::new();
        let mut projects = std::collections::BTreeSet::new();
        let mut rules = req.rules.into_iter();
        let mut prepared = Vec::new();
        let workers = self.batch.workers.map_or(1, std::num::NonZeroUsize::get);
        if self.store.is_none() && workers > 1 && rules.len() >= 128 {
            prepared.extend(rules.by_ref().map(|(id, rule)| (id, rule, None)));
            let chunk_size = prepared.len().div_ceil(workers);
            self.admission_shards += prepared.len().div_ceil(chunk_size);
            let chunks: Vec<_> = prepared.chunks_mut(chunk_size).collect();
            let (_, cpu) = crate::check_session::jobs::run(chunks, |chunk| {
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
                let identity = super::stored::key((
                    scope, digest, self.cache.path_digest(&files), consumed,
                ));
                (query, parameters, Some(identity))
            } else {
                let parameters = typed.unwrap_or_else(|| parameters(std::mem::take(&mut rule.parameters))).map_err(execution)?;
                (sdk_request::query(&mut self.cache, &rule, inputs.get(&id)), parameters, None)
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
        let root = if let Some(id) = self.versions.get(&key).cloned() {
            for (path, (bytes, _)) in &key.1 {
                self.batch.schedule.reprovide(&id, path, bytes);
            }
            id
        } else {
            let id: Arc<str> = Arc::from(format!("check-input:{}", self.versions.len()));
            use crate::check_session::source::Source as _;
            let environment = self.languages.iter().any(|p| p.uses_project() && reader.as_deref().is_none_or(|name| name == p.name())).then(|| {
                let mut environment = crate::check_session::project::Environment::new(
                    id.clone(), authority.clone(), key.1.keys().cloned().collect(), self.observation_epoch.clone(),
                );
                environment.version_scope = scope.map(|(_, request_scope)| request_scope);
                Arc::new(environment)
            });
            if let Some(project) = &environment { self.environments.insert(id.clone(), project.clone()); }
            for (path, (bytes, permission)) in &mut key.1 {
                let (slot, shared) = self.batch.schedule.provide(&id, path, bytes, *permission, &reader, &environment).map_err(execution)?;
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
                    |_| Ok(None),
                    Some(LiveAdmission { version: root, selection, host_tables: &table_ids }),
                    // The retained store must key these pairs by the layout's
                    // physical observations before they can use the file path.
                    self.store.is_none().then(|| (
                        self.languages.as_slice(),
                        self.environments.get(root).map(Arc::as_ref),
                    )),
                )
                .map_err(|reason| rule_error(id.clone(), rule.query.clone(), linked, &hosted, reason))?;
            if let super::Work::Invalid(reason)=&work{
                if let (Some(store),Some(k))=(&mut self.store,contract){store.decline(k,reason);}
                return Err(rule_error(id, rule.query, linked, &hosted, reason.clone()))
            }
            if let super::Work::Closed { project: Some(id), .. } = &work {
                projects.insert(*id);
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
        if self.store.is_none() && count > 1 && ids.len() >= 128 {
            let mut receipts = ids.iter().map(|(id, entry)| {
                let (i, _) = entry.as_ref().err().expect("fresh receipt");
                let receipt = Batch::answer(&self.batch.work[*i], &self.state.answers)
                    .map_err(|e| sdk::LintError::Unrunnable { id: id.clone(), reason: e.to_string() })?;
                Ok::<_, sdk::LintError>((id, receipt))
            }).collect::<Result<Vec<_>, _>>()?;
            let chunk_size = receipts.len().div_ceil(count);
            let chunks: Vec<_> = receipts.chunks_mut(chunk_size).collect();
            let (parts, child_cpu) = crate::check_session::jobs::run(chunks, render_chunk);
            let mut dictionaries = 0;
            for part in parts {
                let (part, count) = part?;
                dictionaries += count;
                out.skipped.extend(part.skipped);
                out.findings.extend(part.findings);
                out.unknowns.extend(part.unknowns);
            }
            drop(receipts);
            for (_, entry) in ids {
                let (i, _) = entry.expect_err("fresh receipt");
                self.batch.work[i] = super::Work::Invalid(String::new());
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
        let place = match row.place {
            (0, _) | (_, 0) => sdk::Place::Missing,
            (line, column) => sdk::Place::At { line: sdk::Line(line), column: sdk::Column(column) },
        };
        out.findings.push(finding(id.clone(), columns[subject].value.clone(), key, place, columns));
    }
    for (path, message) in receipt.unknowns {
        out.unknowns.push(unknown(id.clone(), sdk::RelativePath(path.as_str().into()), sdk::Message(message.as_str().into())));
    }
    Ok(())
}
