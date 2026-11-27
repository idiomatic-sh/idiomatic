//! The batch of requests a session admits, schedules and answers.
use super::*;

#[derive(Deserialize)]
pub struct Input {
    pub root: String,
    #[serde(deserialize_with = "read_paths")]
    pub files: Rc<Vec<String>>,
    #[serde(default, deserialize_with = "read_inventory")]
    pub inventory: Option<Rc<Vec<String>>>,
    pub repository: Option<Authority>,
    pub hosted: Vec<Hosted>,
    #[serde(deserialize_with = "read_parameters")]
    pub parameters: Parameters,
}


pub enum Work {
    Project(Rc<Answer>),
    Closed {
        root: Arc<str>,
        project: Option<usize>,
        pairs: Vec<usize>,
        binding: usize,
        inventory: Rc<Vec<String>>,
        files: Rc<Vec<String>>,
    },
    Pairs(PairSlots),
    Refused(crate::check_session::refusal::Refusal),
    Invalid(crate::check_session::refusal::Refusal),
}

/// New pairs receive consecutive scheduler slots. Preserve an irregular suffix
/// verbatim when a request reuses old slots, including repeated admissions.
#[derive(Default)]
pub struct PairSlots {
    pub first: std::ops::Range<usize>,
    pub rest: Vec<usize>,
}

impl FromIterator<usize> for PairSlots {
    fn from_iter<I: IntoIterator<Item = usize>>(source: I) -> Self {
        let mut slots = source.into_iter();
        let Some(first) = slots.next() else { return Self::default(); };
        let mut end = first + 1;
        while let Some(slot) = slots.next() {
            if slot != end {
                let rest = std::iter::once(slot).chain(slots).collect();
                return Self { first: first..end, rest };
            }
            end += 1;
        }
        Self { first: first..end, rest: Vec::new() }
    }
}

impl session::ReportSelection for PairSlots {
    fn assigned(first: std::ops::Range<usize>) -> Self { Self { first, rest: Vec::new() } }
}

impl Work {
    pub fn pair_slots(&self) -> impl Iterator<Item = usize> + Clone + '_ {
        let (first, rest): (_, &[usize]) = match self {
            Self::Pairs(slots) => (slots.first.clone(), &slots.rest),
            Self::Closed { pairs, .. } => (0..0, pairs),
            _ => (0..0, &[]),
        };
        first.chain(rest.iter().copied())
    }
}

pub struct ProjectSpec {
    pub paths: Rc<Vec<String>>,
    pub slots: Vec<usize>,
    pub authority: Authority,
    pub wanted: BTreeSet<l::Relation>,
}

pub type HostSchema = Vec<(String, Vec<(String, String)>)>;

#[derive(Clone, Copy)]
pub struct LiveAdmission<'a> {
    pub version: &'a Arc<str>,
    pub selection: usize,
    pub host_tables: &'a [usize],
}

pub struct Admission {
    pub id: usize,
    pub contract: Arc<session::completeness::Contract>,
    pub project_bases: Vec<l::Relation>,
    pub closed: bool,
    pub file_cells: bool,
    /// A caller table keyed by File is read in the visit: its states must be settled at admission.
    pub sliced_host: bool,
    pub file_inventory: bool,
    pub layout: Option<(Vec<l::Relation>, Arc<session::completeness::Contract>)>,
    pub cut: Option<Arc<finish::partition::Partition>>,
}

#[derive(Default)]
pub struct Batch {
    pub patterns: finish::Patterns,
    pub file_catalog: session::named::Catalog,
    pub closed_reuse: [usize; 5],
    pub workers: Option<std::num::NonZeroUsize>,
    pub schedule: session::Schedule,
    pub program_of: Vec<usize>,
    pub class_of: Vec<usize>,
    pub bindings: Vec<Arc<[Hosted]>>,
    pub host_rows: HashMap<std::sync::Arc<[std::sync::Arc<[super::HostCell]>]>, usize>,
    pub binding_ids: HashMap<(usize, Option<usize>, Vec<(usize, Option<String>)>), usize>,
    pub layout_of: Vec<Option<usize>>,
    pub layouts: Vec<Option<project::Layout>>,
    pub layout_ids: HashMap<(String, Authority, Rc<Vec<String>>, Vec<l::Relation>), Option<usize>>,
    pub work: Vec<Work>,
    pub projects: Vec<ProjectSpec>,
    pub cut_of: Vec<Option<Arc<finish::partition::Partition>>>,
    pub local_bindings: HashMap<usize, usize>,
    pub project_keys: HashMap<(String, Authority, Rc<Vec<String>>), usize>,
    pub admissions: HashMap<(usize, HostSchema), Rc<Admission>>,
    pub bound_contracts: Vec<Arc<session::completeness::Contract>>,
    pub reused_binding_entries: usize,
    pub host_binding_counts: [usize; 2],
    pub last_sources: Option<(Arc<str>, usize, Arc<session::SelectedFiles>)>,
    pub source_lookups_removed: usize,
    pub slot_lookups_removed: usize,
    pub clocks: Clock,
    pub empty_reader_epochs: usize,
    pub populations_built: usize,
    pub relations_built: usize,
    pub relations_rebuilt: usize,
}

#[derive(Default)]
pub struct Active {
    pub paths: Arc<Vec<String>>,
    pub manifests: Option<BTreeSet<&'static str>>,
    pub readers: Vec<session::ReaderState>,
    pub world: session::World,
    pub answers: ReportSlots,
    pub aliases: session::worker::PairAliases,
    pub contributions: Vec<Option<Rc<project::Contribution>>>,
    pub project_unread: HashMap<usize, Vec<l::Refused>>,
    pub populations: HashMap<usize, (BTreeSet<l::Relation>, Result<Population, crate::check_session::refusal::Refusal>)>,
}

impl Admission {
    pub fn prepare(id: usize, program: &Program, hosted: &[Hosted], catalog: &session::named::Catalog, patterns: &finish::Patterns) -> Result<Self, crate::check_session::refusal::Refusal> {
        let held: Vec<_> = program
            .bases
            .iter()
            .filter_map(|n| relations::service::relation_named(NameRef(n)))
            .filter(|r| catalog::service::scope_of(*r) == catalog::types::RowScope::Project)
            .collect();
        let scope_only = program.bases.iter().all(|name| {
            name == "filesystem.file"
                || held.iter().any(|r| relations::service::name(*r).0 == name)
                || hosted.iter().any(|h| &h.name == name)
        });
        // File-valued caller cells use the admitted request's file dictionary.
        // The same partition can consume its source subtree locally and bind those
        // cells at the join once the inventory is numbered.
        let has_files = |table: &Hosted| table.columns.iter().any(|(_, kind)| kind == "File");
        let numbered_host = hosted.iter().any(has_files);
        let mut run = finish::Run::schema_run(patterns);
        catalog.declare_bases(&mut run, &program.bases);
        let declarations = hosted.iter().map(|h| Hosted {
            name: h.name.clone(), columns: h.columns.clone(), rows: Default::default(), partial: h.partial.clone(),
        }).collect::<Vec<_>>();
        let _ = bind_tables(&declarations, &mut run, |_| None)?;
        let mut exec = finish::Exec::new(&program.plan);
        let local = finish::partition::file_steps(&mut exec, &mut run, |name| {
            held.iter().any(|r| relations::service::name(*r).0 == name) || hosted.iter().any(|t| t.name == name)
        }, |name| hosted.iter().any(|t| t.name == name && has_files(t)))?;
        // A caller table keyed by File that every reader binds to its own file is
        // read in the visit, so it need not wait for the numbered inventory.
        let unsliced = |local: &[bool]| program.plan.nodes.iter().enumerate().any(|(at, step)| {
            !local[at] && step.base().is_some_and(|name| hosted.iter().any(|t| t.name == name && has_files(t)))
        });
        let unsliced_host = unsliced(&local);
        let contract = Arc::new(session::completeness::describe(program, &local)?);
        let wide = contract.cross || unsliced_host;
        let layout = if !scope_only
            && !held.is_empty()
            // One File cell per row and no node or span cell: the rows are the file's own. A relation that needs the file's notes is
            // read in the visit of the file from its own notes (`project::layout` asks the language whether it can answer so, and
            // the program goes the closed way when it cannot).
            && held.iter().all(|relation| run.catalog(relations::service::name(*relation).0).is_some_and(|(_, _, types)| {
                types.iter().filter(|ty| **ty == finish::Ty::File).count() == 1
                    && !types.iter().any(|ty| matches!(ty, finish::Ty::Node | finish::Ty::Span))
            }))
        {
            let names: Vec<&str> = held.iter().map(|relation| relations::service::name(*relation).0).collect();
            finish::partition::with_file_slice(&mut exec, &mut run, &names, |base| {
                hosted.iter().any(|table| table.name == base)
            }, |base| hosted.iter().any(|t| t.name == base && has_files(t)))?.filter(|local| !unsliced(local)).map(|local| session::completeness::describe(program, &local)
                .map(|contract| (held.clone(), Arc::new(contract)))).transpose()?
        } else {
            None
        };
        let cut = if !scope_only && wide {
            Some(Arc::new(finish::partition::partition(&mut exec, &mut run, local)?))
        } else { None };
        Ok(Self {
            id,
            contract,
            project_bases: held,
            closed: scope_only || wide,
            file_cells: numbered_host,
            sliced_host: numbered_host && !unsliced_host,
            file_inventory: program.bases.iter().any(|name| name == "filesystem.file"),
            cut,
            layout,
        })
    }
}

impl Batch {
    pub fn prepare_rules(&mut self, items: Vec<(Query, Parameters)>) {
        let t = thread_cpu();
        self.schedule.prepare_many(items);
        self.clocks.prepare += thread_cpu() - t;
    }
    pub fn prepared_rule(&mut self,query:Query,parameters:&Parameters)->usize{
        let t = thread_cpu();
        let rule = self
            .schedule
            .prepare(query.clone(), parameters.clone(), |modules, literals| {
                let mut ctx = ryo_query_language::domain::plan_lowering::types::RuleContext::default();
                ctx.parameters
                    .insert("query".into(), parameters.clone());
                Program::prepare("query", &query, ctx, Some("/"), modules, literals)
            });
        self.clocks.prepare += thread_cpu() - t;
        crate::finish::operators::query(rule, &query.text);
        rule
    }
    pub fn register_query<L: session::source::Source>(
        &mut self,
        query: Query,
        mut req: Input,
        inventory_for: impl FnOnce(&str) -> Result<Option<Rc<Vec<String>>>, String>,
        sdk: Option<LiveAdmission<'_>>,
        layouts: Option<(&[L], Option<Arc<project::Environment>>)>,
    ) -> Result<Work, crate::check_session::refusal::Refusal> {
        let mut inventory_for = Some(inventory_for);
        let rule=self.prepared_rule(query.clone(),&req.parameters);
        let program=match &self.schedule.programs[rule]{Ok(p)=>p,Err(reason)=>return Ok(Work::Invalid(reason.clone()))};
        let mut host_ids = Vec::new();
        let mut position = 0;
        req.hosted.retain(|host| {
            let referenced = program.bases.iter().any(|name| name == &host.name);
            if let Some(live) = sdk && referenced {
                host_ids.push(live.host_tables[position]);
            }
            position += 1;
            referenced
        });
        let shape: Vec<_> = req
            .hosted
            .iter()
            .map(|t| (t.name.clone(), t.columns.clone()))
            .collect();
        let schema_key = (rule, shape);
        let class = match self.admissions.get(&schema_key) {
            Some(class) => class.clone(),
            None => {
                let class = Rc::new(Admission::prepare(self.admissions.len(), program, &req.hosted, &self.file_catalog, &self.patterns)?);
                self.admissions.insert(schema_key, class.clone());
                class
            }
        };
        if class.sliced_host {
            // The closed phase refuses a partial caller table and an inventory it cannot
            // number. A program read in the visits may see no file, so refuse here.
            if let Some(table) = req.hosted.iter().find(|table| table.partial.is_some()) {
                let reason = table.partial.as_deref().unwrap_or_default();
                return Ok(Work::Refused(format!("partial {}: {reason}", table.name).into()));
            }
            if (!class.closed || (req.files.is_empty() && !class.contract.cross)) && req.inventory.is_none() && inventory_for.take().ok_or("inventory reader")?(&req.root)?.is_none() {
                return Err("incomplete project inventory".into());
            }
        }
        if let (Some((relations, _)), Some((readers, environment)), Some(inventory), Some(authority)) =
            (&class.layout, layouts, &req.inventory, &req.repository)
            && !req.files.is_empty()
            && req.files.iter().all(|path| inventory.binary_search(path).is_ok())
            && (req.files.windows(2).all(|pair| pair[0] < pair[1])
                || req.files.windows(2).all(|pair| pair[0] > pair[1])
                || req.files.iter().collect::<BTreeSet<_>>().len() == req.files.len())
        {
            let key = (req.root.clone(), authority.clone(), inventory.clone(), relations.clone());
            let layout = *self.layout_ids.entry(key).or_insert_with(|| {
                let table = project::layout(readers, authority, inventory, environment.clone(), relations)?;
                let id = self.layouts.len();
                self.layouts.push(Some(table));
                Some(id)
            });
            if let Some(layout) = layout {
                let existing = self.bindings.len();
                let binding = self.binding(rule, &class, req.hosted, sdk.map(|_| host_ids.as_slice()), Some(layout));
                let pairs = self.admit(&req.root, &req.files, binding, binding < existing, sdk)?;
                return Ok(Work::Pairs(pairs));
            }
        }
        let held = &class.project_bases;
        let contract = &class.contract;
        if req.files.is_empty() && !contract.cross {
            return Ok(Work::Pairs(PairSlots::default()));
        }
        if class.closed {
            let t = thread_cpu();
            let inventory = match req.inventory {
                Some(paths) => Some(paths),
                None => inventory_for.take().ok_or("inventory reader")?(&req.root)?,
            }
                .or_else(|| {
                    (!class.file_cells && (!req.files.is_empty() || !class.file_inventory)).then(|| {
                        let mut paths = req.files.as_ref().clone();
                        paths.sort_unstable();
                        Rc::new(paths)
                    })
                })
                .ok_or("incomplete project inventory")?;
            let project = if held.is_empty() {
                None
            } else {
                let authority = req
                    .repository
                    .ok_or("project source authority was not declared")?;
                let key = (req.root.clone(), authority.clone(), inventory.clone());
                let id = match self.project_keys.get(&key) {
                    Some(&i) => i,
                    None => {
                        let i = self.projects.len();
                        self.projects.push(ProjectSpec {
                            paths: inventory.clone(),
                            slots: self.schedule.project_inputs(&req.root, &inventory)?,
                            authority,
                            wanted: BTreeSet::new(),
                        });
                        self.project_keys.insert(key, i);
                        i
                    }
                };
                self.projects[id].wanted.extend(held.iter().copied());
                Some(id)
            };
            if req.files.is_empty() && !inventory.is_empty() && contract.frontier && held.is_empty()
            {
                return Ok(Work::Project(Rc::new(Ok(session::Reported::skipped(&inventory)))));
            }
            let existing = self.bindings.len();
            let binding = self.binding(rule, &class, req.hosted, sdk.map(|_| host_ids.as_slice()), None);
            let pairs = if class.cut.is_some() {
                // Caller rows are consumed only after closure. Sources with the same
                // prepared program and caller schema produce identical local outputs.
                let local = *self.local_bindings.entry(class.id).or_insert(binding);
                self.admit(&req.root, &req.files, local, local < existing, sdk)?
            } else {
                Vec::new()
            };
            let root: Arc<str> = sdk.map_or_else(|| req.root.as_str().into(), |live| live.version.clone());
            self.clocks.project += thread_cpu() - t;
            return Ok(Work::Closed {
                root,
                pairs,
                project,
                binding,
                inventory,
                files: req.files,
            });
        }
        if req.files.is_empty() {
            return Err("no complete source contribution for query".into());
        }
        let existing = self.bindings.len();
        let binding = self.binding(rule, &class, req.hosted, sdk.map(|_| host_ids.as_slice()), None);
        let pairs = self.admit(&req.root, &req.files, binding, binding < existing, sdk)?;
        Ok(Work::Pairs(pairs))
    }

    pub fn admit<T: session::ReportSelection>(
        &mut self,
        root: &str,
        files: &[String],
        binding: usize,
        known_binding: bool,
        sdk: Option<LiveAdmission<'_>>,
    ) -> Result<T, crate::check_session::refusal::Refusal> {
        let Some(LiveAdmission { version, selection, .. }) = sdk else {
            let sources = self.schedule.select_files(&Arc::from(root), files)?;
            return Ok(self.schedule.admit_ids(&sources, binding, known_binding));
        };
        // Slot histories precede this binding. A fresh ID cannot be in any of them.
        if !known_binding {
            self.slot_lookups_removed += files.len();
        }
        if let Some((prior, id, sources)) = &self.last_sources
            && Arc::ptr_eq(prior, version) && *id == selection
        {
            self.source_lookups_removed += sources.len();
            return Ok(self.schedule.admit_ids(sources, binding, known_binding));
        }
        let sources = self.schedule.select_files(version, files)?;
        let pairs = self.schedule.admit_ids(&sources, binding, known_binding);
        self.last_sources = Some((version.clone(), selection, sources));
        Ok(pairs)
    }

    pub fn binding(
        &mut self,
        rule: usize,
        class: &Admission,
        mut hosted: Vec<Hosted>,
        canonical: Option<&[usize]>,
        layout: Option<usize>,
    ) -> usize {
        let mut payloads = Vec::with_capacity(hosted.len());
        for (position, table) in hosted.iter_mut().enumerate() {
            let id = if let Some(ids) = canonical {
                self.host_binding_counts[0] += 1;
                self.host_binding_counts[1] += table.rows.len();
                ids[position]
            } else if let Some((rows, &id)) = self.host_rows.get_key_value(&table.rows) {
                table.rows = rows.clone();
                id
            } else {
                let id = self.host_rows.len();
                self.host_rows.insert(table.rows.clone(), id);
                id
            };
            payloads.push((id, table.partial.clone()));
        }
        // The class binds the program and ordered schemas. Each payload ID binds
        // all its cells by value, so the binding lookup need not hash them again.
        *self.binding_ids.entry((class.id, layout, payloads)).or_insert_with(|| {
            self.program_of.push(rule);
            self.class_of.push(class.id);
            self.layout_of.push(layout);
            self.cut_of.push(if layout.is_some() { None } else { class.cut.clone() });
            self.bound_contracts.push(if layout.is_some() {
                class.layout.as_ref().expect("proved file layout").1.clone()
            } else {
                class.contract.clone()
            });
            self.bindings.push(hosted.into());
            self.bindings.len() - 1
        })
    }

    pub fn run_sources<L: session::source::Source>(&mut self, readers: &[L]) -> ReportSlots {
        // The closed, store-disabled capture cannot register another request.
        // Release the admission-only maps before the reader and join heaps grow.
        self.project_keys = Default::default();
        self.admissions = Default::default();
        self.local_bindings = Default::default();
        self.release_admission();
        let mut state = Active::default();
        self.epoch(readers, &mut state);
        state.answers
    }

    pub fn release_admission(&mut self) {
        self.host_rows.clear();
        self.last_sources = None;
        self.schedule.close();
        self.binding_ids.clear();
    }

    pub fn epoch<L: session::source::Source>(&mut self,languages:&[L],state:&mut Active) {
        self.epoch_with_store(languages,state,None)
    }
    pub fn epoch_with_store<L: session::source::Source>(
        &mut self,
        languages: &[L],
        state: &mut Active,
        mut store:Option<&mut stored::Store>,
    ) {
        self.schedule.finish_selection();
        self.schedule.finish_preparation();
        if state.world.run.texts.is_empty() {
            state.world.declare_words(languages);
        }
        state.answers.grow(self.schedule.pairs);
        state.contributions.resize_with(self.schedule.sources.len(), || None);
        // A later round may need notes that were released after a project closed.
        let needed_projects: BTreeSet<_> = self.work.iter().filter_map(|work| match work {
            Work::Closed { project, .. } => *project,
            _ => None,
        }).collect();
        // A project whose population is already built for the same relations reads no notes.
        let built = |project: usize, populations: &HashMap<usize, (BTreeSet<l::Relation>, Result<Population, crate::check_session::refusal::Refusal>)>|
            populations.get(&project).is_some_and(|(wanted, _)| *wanted == self.projects[project].wanted);
        let mut notes_needed = BTreeSet::new();
        for project in needed_projects {
            if built(project, &state.populations) { continue; }
            for &id in &self.projects[project].slots {
                notes_needed.insert(id);
                if state.contributions[id].is_none() {
                    self.schedule.sources[id].project = true;
                    self.schedule.pending.insert(id);
                }
            }
        }
        // A source pending only as a project input, whose notes no unbuilt population needs, is not opened.
        let schedule = &mut self.schedule;
        let idle: Vec<usize> = schedule.pending.iter().copied().filter(|&id| {
            let source = &schedule.sources[id];
            source.project && source.bindings.is_empty() && !notes_needed.contains(&id)
        }).collect();
        for id in idle {
            schedule.pending.remove(&id);
            schedule.sources[id].project = false;
        }
        let mut probe = Vec::new();
        if let Some(store) = store.as_deref_mut() {
            let started = thread_cpu();
            store.prepare(self);
            let (files, candidates) = store.probe(self);
            probe = files;
            if !probe.is_empty() {
                self.read_pending(languages, state, &probe, false);
                store.local(self, &state.world, &state.answers, &state.aliases, &state.contributions, &probe);
                store.reuse_closed(self, candidates);
            }
            store.admit(self, &mut state.world, &mut state.answers, &mut state.contributions, &probe);
            eprintln!("store_admit_cpu={:.6}", thread_cpu() - started);
        }
        let pending: Vec<_> = std::mem::take(&mut self.schedule.pending).into_iter().collect();
        self.read_pending(languages, state, &pending, store.is_none());
        let Active { readers: reader_states, world, answers, aliases, contributions, populations, project_unread, .. } = state;
        if let Some(store) = store.as_deref_mut() {
            let started = thread_cpu();
            store.local(self, world, answers, aliases, contributions, &pending);
            eprintln!("store_local_cpu={:.6}", thread_cpu() - started);
        }
        if !aliases.is_empty() {
            for work in &mut self.work {
                match work {
                    Work::Pairs(slots) => {
                        *slots = slots.first.clone().chain(slots.rest.iter().copied())
                            .map(|slot| aliases.lookup(slot)).collect();
                    }
                    Work::Closed { pairs, .. } => {
                        for slot in pairs { *slot = aliases.lookup(*slot); }
                    }
                    _ => {}
                }
            }
        }
        *aliases = Default::default();
        self.schedule.delivered(&probe, |id| self.cut_of[id].is_none());
        self.schedule.delivered(&pending, |id| self.cut_of[id].is_none());
        self.finish_projects(languages, reader_states, contributions, world, answers, populations, project_unread, store.as_deref_mut());
        // The closed readers above placed their rows with the bytes held here.
        reader_states.clear();
        for layout in self.layout_ids.drain().filter_map(|(_, id)| id) {
            self.layouts[layout] = None;
        }

    }

    pub fn read_pending<L: session::source::Source>(
        &mut self,
        languages: &[L],
        state: &mut Active,
        pending: &[usize],
        release_files: bool,
    ) {
        let Active { paths, manifests, readers: reader_states, world, answers, aliases, contributions, .. } = state;
        if pending.is_empty() { self.empty_reader_epochs += 1; }
        let unclaimed = languages.len();
        reader_states.resize_with(unclaimed + 1, Default::default);
        let mut groups = vec![Vec::new(); unclaimed + 1];
        for &i in pending {
            let src = &self.schedule.sources[i];
            if src.bindings.is_empty() && contributions[i].is_some() {
                continue;
            }
            let path = src.path.as_str();
            let mut found = session::source::routed(languages, src.reader.as_deref(), path);
            match (found.next(), found.next()) {
                (Some((index, _)), None) => groups[index].push(i),
                (None, None) => groups[unclaimed].push(i),
                _ => {
                    for (_, slot) in src.bindings.iter() {
                        answers.set(slot, Err(format!("multiple source readers for {}", src.path).into()));
                    }
                }
            }
        }
        self.clocks.pairs = self.schedule.pairs;
        if groups.iter().any(|files| !files.is_empty()) {
            let manifests = manifests.get_or_insert_with(|| languages.iter().flat_map(|l| l.manifests()).collect());
            self.reused_binding_entries += self.cut_of.len() + self.bound_contracts.len() + self.bindings.len();
            self.clocks.omitted_file_bindings += self.bindings.len();
            let hosted = &self.bindings;
            let file_workers = self.workers;
            let start = paths.len();
            if start < self.schedule.sources.len() {
                Arc::make_mut(paths).extend(self.schedule.sources[start..].iter().map(|s| s.path.clone()));
            }
            let readers = languages.iter().map(Some).chain([None]);
            for (reader_id, (language, files)) in readers.zip(groups).enumerate() {
                if files.is_empty() {
                    continue;
                }
                session::worker::dispatch(
                    file_workers,
                    languages,
                    paths,
                    &mut reader_states[reader_id],
                    language,
                    (reader_id != unclaimed).then_some(reader_id),
                    manifests,
                    contributions,
                    &self.schedule.programs,
                    &self.program_of,
                    &self.bound_contracts,
                    &self.cut_of,
                    (&self.layout_of, &self.layouts),
                    world,
                    &self.schedule.sources,
                    &files,
                    aliases,
                    answers,
                    |binding, run| {
                        // A file program reads a caller table keyed by File here, through the
                        // visit's numbering. A closed program reads it only after closure.
                        let keyed = self.cut_of[binding].is_none()
                            && hosted[binding].iter().any(|t| t.columns.iter().any(|(_, kind)| kind == "File"));
                        let paths = run.paths.clone();
                        let numbers: HashMap<&str, u32> = if keyed {
                            paths.iter().zip(0..).map(|(path, number)| (path.as_str(), number)).collect()
                        } else {
                            HashMap::new()
                        };
                        Ok(bind_tables(&hosted[binding], run, |path| numbers.get(path).copied())?
                            .into_iter()
                            .map(|(n, t)| (n, Rc::new(t)))
                            .collect())
                    },
                    &mut self.clocks,
                );
                if release_files {
                    // Other readers have disjoint files. Answers and project contributions
                    // already own everything the closed phase will read from this group.
                    self.schedule.delivered(&files, |id| self.cut_of[id].is_none());
                    for i in files {
                        self.schedule.sources[i].provided = None;
                    }
                }
            }
        } else {
            self.clocks.empty_file_epochs += 1;
            self.clocks.omitted_file_bindings += self.bindings.len();
        }
    }

    pub fn add_work(&mut self, work: Work, retained: Option<(&mut stored::Store, stored::Key)>) -> usize {
        let index = self.work.len();
        if let Some((store, key)) = retained
            && matches!(&work, Work::Pairs(_) | Work::Closed { .. } | Work::Project(_)) {
            store.offer(index, key);
        }
        self.work.push(work);
        index
    }

    pub fn finish_projects<L: session::source::Source>(
        &mut self,
        languages: &[L],
        reader_states: &[session::ReaderState],
        contributions: &mut Vec<Option<Rc<project::Contribution>>>,
        world: &mut session::World,
        answers: &mut ReportSlots,
        populations: &mut HashMap<usize, (BTreeSet<l::Relation>, Result<Population, crate::check_session::refusal::Refusal>)>,
        project_unread: &mut HashMap<usize, Vec<l::Refused>>,
        mut store:Option<&mut stored::Store>,
    ) {
        let wall = std::time::Instant::now();
        let t = thread_cpu();
        let _profile = finish::operators::scope(usize::MAX, "closed");
        let preparing = finish::operators::enter(usize::MAX - 3, 0);
        // Bound values select a different population. Only changed column declarations
        // require another prepared layout for the same lowered program.
        let mut groups = BTreeMap::new();
        for (i, work) in self.work.iter().enumerate() {
            if let Work::Closed { binding, .. } = work {
                let rule = self.program_of[*binding];
                // Admission has already identified the program and ordered host schema.
                groups
                    .entry((rule, self.class_of[*binding]))
                    .or_insert_with(Vec::new)
                    .push(i);
            }
        }

        let active: BTreeSet<_> = self
            .work
            .iter()
            .filter_map(|w| match w {
                Work::Closed { project, .. } => *project,
                _ => None,
            })
            .collect();
        for &id in &active {
            let s = &self.projects[id];
            if populations
                .get(&id)
                .is_some_and(|(wanted, _)| wanted == &s.wanted)
            {
                continue;
            }
            self.populations_built += 1;
            self.relations_built += s.wanted.len();
            if let Some((prior, _)) = populations.get(&id) {
                self.relations_rebuilt += s.wanted.intersection(prior).count();
            }
            let provider = std::time::Instant::now();
            let population = (|| {
                // A project input the host could not read has no contribution: the project is read without it, and the file is
                // named unread (a refused input), as a manifest the tools refuse is.
                let mut missing = Vec::new();
                let sources = s
                    .slots
                    .iter()
                    .map(|&slot| {
                        contributions[slot].clone().unwrap_or_else(|| {
                            missing.push(slot);
                            Rc::default()
                        })
                    })
                    .collect::<Vec<_>>();
                let mut read = project::collect(
                    world,
                    languages,
                    &s.authority,
                    &s.paths,
                    { let mut held = s.slots.iter().filter_map(|&slot| self.schedule.sources[slot].environment.as_ref()); let first = held.next(); debug_assert!(held.all(|each| first.is_some_and(|first| Arc::ptr_eq(first, each))), "one project input spans two versions"); first.map(Arc::as_ref) },
                    &sources,
                    &s.wanted.iter().copied().collect::<Vec<_>>(),
                )?;
                read.refused.extend(missing.into_iter().map(|slot| l::Refused {
                    path: l::FilePath(self.schedule.sources[slot].path.as_str().into()),
                    // The host's own detail of why the file could not be read.
                    reason: ryo_language::domain::reasons::types::ToolMessage(
                        std::fs::File::open(std::path::Path::new(&*self.schedule.sources[slot].root).join(&self.schedule.sources[slot].path))
                            .err()
                            .map_or_else(|| "cannot read the file".to_owned(), |error| error.to_string())
                            .into(),
                    ),
                }));
                Ok(read)
            })();
            self.clocks.project_provider_wall += provider.elapsed().as_secs_f64();
            if let Ok(p) = &population && !p.refused.is_empty() {
                project_unread.insert(id, p.refused.clone());
            } else { project_unread.remove(&id); }
            populations.insert(id, (s.wanted.clone(), population));
        }
        contributions.clear();
        if let Some(start) = preparing {
            finish::operators::leave(start, "closed-prepare-populations", 0, true);
        }
        self.clocks.project_prepare_wall += wall.elapsed().as_secs_f64();
        if groups.is_empty() {
            self.clocks.empty_project_epochs += 1;
            self.clocks.project += thread_cpu() - t;
            self.clocks.project_wall += wall.elapsed().as_secs_f64();
            return;
        }
        self.file_catalog.declare(&mut world.run);
        let mut readers=readers::Closed::new(&self.work,&self.cut_of);
        let workers = self.workers.map_or(1, std::num::NonZeroUsize::get);
        // Small SDK validation cohorts use the same closed reader without
        // reconstructing a word table and posting caches on each worker.
        if store.is_none() && workers > 1 && self.work.len() >= 128 && groups.len() > 1 {
            let groups: Vec<_> = groups.into_values().collect();
            self.clocks.project_shards += workers.min(groups.len());
            let worker_cpu = closed_workers::run(
                workers, self, world, populations, answers, groups.into_iter(), &mut readers, reader_states,
            );
            readers.release_finished(|slot| {
                world.release(slot);
                answers.release(slot);
            }, |id| { populations.remove(&id); });
            self.clocks.project_workers_cpu += worker_cpu;
            self.clocks.project_number_wall += readers.number_wall;
            self.clocks.project_numbered += readers.numbered;
            self.clocks.project += thread_cpu() - t;
            self.clocks.project_wall += wall.elapsed().as_secs_f64();
            return;
        }
        let mut coordinates = HashMap::new();
        let mut reused = [0; 3];
        for ((rule, _), requests) in groups {
            let plan = &self.schedule.programs[rule].as_ref().unwrap().plan;
            let mut exec = finish::Exec::new(plan);
            let mut operators = finish::session::reuse::Reuse::new(requests.len());
            let mut evaluated = readers::Evaluated::default();
            for i in requests {
                let Work::Closed {
                    root, pairs,
                    project,
                    binding,
                    inventory,
                    files,
                } = &self.work[i]
                else {
                    unreachable!()
                };
                let population = project.map(|p| populations[&p].1.as_ref()).transpose();
                let _profile = finish::operators::scope(rule, "closed");
                let scope = readers.scope(i);
                let answer = if let Some(prior)=readers.duplicate[i] {
                    let Work::Project(value)=&self.work[prior] else {unreachable!("same binding evaluated in request order")};
                    value.clone()
                } else {Rc::new(population.map_err(Clone::clone).and_then(|p| {
                    let delivering = finish::operators::enter(usize::MAX, 0);
                    let absent = &closed_workers::absent_files(&self.schedule, &self.bound_contracts[*binding], self.cut_of[*binding].is_some(), root, files);
                    let inputs = closed_workers::inputs(&world.run, &self.bound_contracts[*binding],
                        &world.unavailable, pairs, inventory, files, absent, self.cut_of[*binding].as_deref(), p, answers,
                        |readable, steps| readers.inputs(i, || world.joined(readable, steps)));
                    if let Some(start) = delivering {
                        finish::operators::leave(start, "closed-deliver", 0, true);
                    }
                    let inputs = inputs?;
                    let (carried, incomplete, unknowns, skipped) = match inputs {
                        closed_workers::Inputs::Withheld(answer) => return Ok(answer),
                        closed_workers::Inputs::Ready { carried, incomplete, unknowns, skipped } => (carried, incomplete, unknowns, skipped),
                    };
                    let reusable = unknowns.is_empty() && incomplete.is_empty() && skipped.is_empty() && readers.may_repeat(scope);
                    if reusable && let Some(answer) = evaluated.find(scope, p, &carried) {
                        return Ok(answer);
                    }
                    let numbering = finish::operators::enter(usize::MAX - 3, 0);
                    let numbers = readers.numbers(i, inventory, world);
                    if let Some(start) = numbering {
                        finish::operators::leave(start, "closed-number", 0, true);
                    }
                    let evaluating = finish::operators::enter(usize::MAX - 1, 0);
                    let rows = execute_ids(
                        &mut exec,
                        &mut world.run,
                        numbers,
                        &carried,
                        inventory,
                        &closed_workers::accepted_files(files, &skipped),
                        absent,
                        &self.bindings[*binding],
                        p,
                        &incomplete,
                        skipped.is_empty().then_some((&mut operators, scope)),
                    );
                    if let Some(start) = evaluating {
                        finish::operators::leave(start, "closed-execute", rows.as_ref().map_or(0, Vec::len), true);
                    }
                    let rows = rows?;
                    let reported = session::Reported::new(rows, unknowns, skipped);
                    if reusable { evaluated.remember(scope, p, &carried, &reported); }
                    Ok(reported)
                }).and_then(|mut report| {
                    let locating = finish::operators::enter(usize::MAX - 3, 0);
                    let located = self.schedule.locate_closed(root, reader_states, &mut report, &mut coordinates);
                    if let Some(start) = locating {
                        finish::operators::leave(start, "closed-locate", 0, true);
                    }
                    located?;
                    Ok(report)
                }))};
                let releasing = finish::operators::enter(usize::MAX - 3, 0);
                exec.release_request();
                if let Some(s)=store.as_deref_mut() {
                    let pop=project.and_then(|id| populations[&id].1.as_ref().ok());
                    s.closed(i,&answer,pop,project.map(|id| (id, &self.projects[id].authority)));
                }
                // Closed slots are never re-admitted via Source::known.
                // Store::local and Store::closed have already consumed their evidence.
                if let Some(finished)=readers.release(i,pairs,*project,|slot| {
                    world.release(slot);
                    answers.release(slot);
                }) {
                    populations.remove(&finished);
                }
                if let Some(start) = releasing {
                    finish::operators::leave(start, "closed-release", 0, true);
                }
                self.work[i] = Work::Project(answer);
            }
            reused[0] += operators.empty_emits;
            reused[1] += evaluated.hits;
            reused[2] += operators.restored;
        }
        let (posting, capture) = finish::session::reuse::take_counts();
        for (count, additional) in self.closed_reuse.iter_mut().zip([
            reused[0], reused[1], reused[2], posting, capture,
        ]) {
            *count += additional;
        }
        self.clocks.project_number_wall += readers.number_wall;
        self.clocks.project_numbered += readers.numbered;
        self.clocks.project += thread_cpu() - t;
        self.clocks.project_wall += wall.elapsed().as_secs_f64();
    }

    pub fn answer<'a>(work: &'a Work, answers: &'a ReportSlots) -> Result<Receipt<'a>, std::borrow::Cow<'a, crate::check_session::refusal::Refusal>> {
        let mut receipt = Receipt::default();
        match work {
            Work::Project(result) => receipt.append(result)?,
            Work::Closed { .. } => return Err(std::borrow::Cow::Owned("project inputs still open".into())),
            Work::Refused(reason) => return Err(std::borrow::Cow::Borrowed(reason)),
            Work::Invalid(reason) => return Err(std::borrow::Cow::Borrowed(reason)),
            Work::Pairs(slots) => {
                answers.visit_pending(slots.first.clone(), |slot| receipt.append(&answers[slot]))?;
                for &slot in &slots.rest { receipt.append(&answers[slot])?; }
            }
        }
        receipt.unknowns.sort_unstable();
        receipt.unknowns.dedup();
        receipt.unread.sort_unstable();
        receipt.unread.dedup();
        Ok(receipt)
    }
}

#[derive(Default)]
pub struct Receipt<'a> {
    pub rejected_syntax: Vec<&'a String>,
    pub rows: Vec<&'a finish::answer::Row>,
    pub unknowns: Vec<&'a (String, String)>,
    pub unread: Vec<&'a (String, String)>,
}
impl<'a> Receipt<'a> {
    pub fn append(&mut self, answer: &'a Answer) -> Result<(), std::borrow::Cow<'a, crate::check_session::refusal::Refusal>> {
        let held = answer.as_ref().map_err(|refusal| match refusal {
            crate::check_session::refusal::Refusal::Other(s) if s.is_empty() => std::borrow::Cow::Owned("no source reader".into()),
            other => std::borrow::Cow::Borrowed(other),
        })?;
        self.rejected_syntax.extend(&held.rejected_syntax);
        self.rows.extend(&held.rows);
        self.unknowns.extend(&held.unknowns);
        self.unread.extend(&held.unread);
        Ok(())
    }
}

#[cfg(test)]
mod slot_tests {
    use super::{PairSlots, Work};

    #[test]
    fn compact_slots_preserve_report_order_and_duplicates() {
        for (input, suffix) in [
            (&[][..], &[][..]),
            (&[4][..], &[][..]),
            (&[4, 5, 6][..], &[][..]),
            (&[4, 5, 5, 1, 4, 7][..], &[5, 1, 4, 7][..]),
        ] {
            let slots: PairSlots = input.iter().copied().collect();
            assert_eq!(slots.rest, suffix);
            let work = Work::Pairs(slots);
            assert!(work.pair_slots().eq(input.iter().copied()));
        }
    }
}
