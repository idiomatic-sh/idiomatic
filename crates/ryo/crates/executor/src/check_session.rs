//! Schedule all consumers of a source before walking it. Keep the emitted rows per
//! bound program and source version. Parser scratch is released after its last consumer.
use super::{finish, first, lower, reader, reader_run, thread_cpu};
use finish::{Exec, Plan, Run, Table};
use super::pair_skip::{BoundSummary as BoundRequirements, Demand, FileSummary, PrerequisiteClauses};
use crate::languages::{self as l, ByteSlice, FileId, SourcePath};
use ryo_relations::domain::{catalog, labels::types::NameRef, relations};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
    rc::Rc,
    sync::Arc,
};

pub mod completeness;
pub mod pairs;
pub mod refusal;
pub mod kinds;
#[cfg(test)]
mod kinds_test;
#[cfg(test)]
mod refusal_test;
pub mod jobs;
pub mod file_programs;
use file_programs::FilePrograms;
pub mod project;

pub mod source;
use source::Source as FileSource;
pub mod named;
pub mod frontier;
pub use frontier::World;
pub use frontier::SharedFrontier;
pub mod shared;

pub mod stored_block;

pub mod query;
use query::Query;

pub type Parameters =
    BTreeMap<Box<str>, ryo_values::domain::finish_plans::expressions::types::PlanConstant>;

pub type Result<T> = std::result::Result<T, String>;
pub type Answer = Result<Reported>;

pub mod report_slots;
pub mod hosted;
pub mod replay;

pub use finish::answer::Reported;
pub type Tables = Vec<(String, Rc<Table>)>;

pub struct Program {
    pub typed: Arc<ryo_values::domain::finish_plans::types::FinishPlan>,
    pub plan: Plan,
    pub accepted_only: bool,
    pub bases: Vec<String>,
    pub base_steps: Vec<(usize, Option<l::Relation>)>,
    pub renders: bool,
    /// The program's file requirements for each reader that has asked: they depend on the
    /// program and the reader's declaration only, so they are made once per prepared program.
    pub requirements: std::sync::Mutex<Vec<(Option<usize>, std::sync::Arc<Requirements>)>>,
    /// Whether a word the program writes for a kind names none of the linked languages' kinds, made once: the languages are the session's.
    pub unknown_kind: std::sync::OnceLock<Option<refusal::Refusal>>,
}

pub type Requirements = (bool, Vec<Vec<String>>, Vec<Vec<String>>);

pub fn reader_declaration<L: FileSource>(parser: Option<&L>) -> first::Language {
    let mut language = first::declaration();
    if let Some(words) = parser.and_then(FileSource::source_words) {
        language.is_identifier = words.identifier;
        language.spelled.extend(words.columns.iter().map(|&(relation, column, spelling)| {
            let separator = match spelling {
                l::Spelling::Whole => None,
                l::Spelling::LastSegment(c) => Some(c),
            };
            (relation.to_owned(), column.to_owned(), separator)
        }));
    }
    language
}

impl Program {
    pub fn prepare(
        alias: &str,
        query: &Query,
        context: ryo_engine::domain::plan_lowering::types::RuleContext,
        capture_separator: Option<&str>,
        modules: ryo_values::domain::program::types::ParsedModules,
        literals: &mut lower::SharedPlanData,
    ) -> std::result::Result<Self, refusal::Refusal> {
        let (plan, pack) = lower::query_parts(alias, &query.text, context, true, capture_separator, modules, &query.observations, literals)?;
        let typed = pack.typed.take().expect("prepared query");
        Self::assemble(query, plan, typed)
    }

    /// A lowered query's plan with what admission reads from it.
    pub fn assemble(
        query: &Query,
        plan: Plan,
        typed: Arc<ryo_values::domain::finish_plans::types::FinishPlan>,
    ) -> std::result::Result<Self, refusal::Refusal> {
        if typed.rules.is_empty() { return Err("missing query emit".into()); }
        let (bases, base_steps) = plan.nodes.iter().enumerate().filter_map(|(offset, step)| {
            step.base().map(|name| {
                (name.to_owned(), (offset, relations::service::relation_named(NameRef(name))))
            })
        }).unzip();
        use ryo_values::domain::finish_plans::types::FinishStep;
        let renders = typed.steps.iter().any(|step| matches!(step, FinishStep::RenderSyntax(_) | FinishStep::FitsRendered(_)));
        let mut plan = plan;
        plan.every_row = query.every_row;
        Ok(Self {
            renders,
            plan,
            accepted_only: query.accepted_only,
            typed,
            bases,
            base_steps,
            requirements: Default::default(),
            unknown_kind: Default::default(),
        })
    }

    /// The refusal of a word written for a kind no language of `languages` (names with their kinds) has: asked once for the program.
    pub fn refuse_unknown_kind(&self, languages: impl FnOnce() -> Vec<(String, Vec<String>)>) -> Option<refusal::Refusal> {
        self.unknown_kind.get_or_init(|| kinds::unknown(&self.typed, languages)).clone()
    }

    pub fn requirements(&self, reader: Option<usize>, make: impl FnOnce() -> Requirements) -> (std::sync::Arc<Requirements>, bool) {
        let mut held = self.requirements.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((_, found)) = held.iter().find(|(id, _)| *id == reader) {
            return (found.clone(), false);
        }
        let made = std::sync::Arc::new(make());
        held.push((reader, made.clone()));
        (made, true)
    }
}

#[derive(Default)]
pub struct Clock {
    pub readers: BTreeMap<String, ReaderParts>,
    pub inventory: f64,
    pub inventory_wall: f64,
    pub join_wall: f64,
    pub shard_critical_wall: f64,
    pub shard_busy_wall: f64,
    pub largest_file_wall: f64,
    pub largest_file: String,
    pub measure_file_wall: bool,
    pub join: f64,
    pub prepare: f64,
    pub read_parse: f64,
    pub read_versions: f64,
    pub build_file: f64,
    pub screen: f64,
    pub bound_proofs: f64,
    pub literal_positions: usize,
    pub witnesses: [usize; 4],
    pub exact_inputs: [usize; 8],
    pub named_counts: [usize; 4],
    pub named_cpu: f64,
    pub requirement_programs: usize,
    pub requirement_bindings: usize,
    /// The files a project relation was read for in their own visit, from their own notes.
    pub visit_projects: usize,
    /// The node runs of the visits: what the run says of its work to a subscriber that listens (`ryo::node_counters`).
    pub node_runs: u64,
    pub bulk_screen_files: usize,
    pub old_route: usize,
    pub evaluate: f64,
    pub project: f64,
    pub assemble: f64,
    pub project_wall: f64,
    pub project_workers_cpu: f64,
    pub project_prepare_wall: f64,
    pub project_provider_wall: f64,
    pub project_number_wall: f64,
    pub project_numbered: usize,
    pub empty_project_epochs: usize,
    pub project_shards: usize,
    pub delivery_workers_cpu: f64,
    pub file_workers_cpu: f64,
    pub file_shards: usize,
    pub empty_file_epochs: usize,
    pub omitted_file_bindings: usize,
    pub files: usize,
    pub reads: usize,
    pub unique_pairs: usize,
    pub pairs: usize,
    pub skipped: usize,
    pub evaluated: usize,
    pub certified: usize,
    pub productive: usize,
    pub empty: usize,
    pub refused: usize,
}

#[derive(Default)]
pub struct ReaderParts {
    pub total: f64,
    pub read: f64,
    pub build: f64,
    pub tree: f64,
    pub facts: f64,
    pub parsed: usize,
    /// The files this reader's group visited that no numbered file was lent for: the old route for a claimed file, and for
    /// the `unclaimed` group the files no reader claims.
    pub old_route: usize,
}

/// The root and path identify the caller's immutable effective source. This
/// distinguishes in-memory replacements staged at different roots.
pub struct Source {
    pub environment: Option<Arc<project::Environment>>,
    pub permission: Permission,
    pub root: Arc<str>,
    pub path: String,
    pub reader: Option<Arc<str>>,
    pub bindings: FilePrograms,
    pub known: FilePrograms,
    pub project: bool,
    pub provided: Option<Arc<[u8]>>,
}



pub use ryo_values::domain::files::types::ExecutablePermission as Permission;
pub type PathIds = HashMap<Rc<str>, usize>;

pub trait ReportSelection: FromIterator<usize> {
    fn assigned(slots: std::ops::Range<usize>) -> Self;
}
impl ReportSelection for Vec<usize> {
    fn assigned(slots: std::ops::Range<usize>) -> Self { slots.collect() }
}

/// One immutable input to all programs that select the same ordered files.
pub struct SelectedFiles {
    pub ids: Vec<usize>,
    pub distinct: bool,
}
impl SelectedFiles {
    pub fn len(&self) -> usize { self.ids.len() }
}

/// Per-check program identities and the inverse admission index. The caller binds
/// external relations and hands the index a stable identity for each binding.
#[derive(Default)]
pub struct Schedule {
    pub sources: Vec<Source>,
    pub source_ids: HashMap<Arc<str>, PathIds>,
    pub supplied_ids: HashMap<(Rc<str>, Arc<[u8]>, Permission, Option<Arc<str>>, Option<Arc<str>>), usize>,
    pub named_disk_ids: HashMap<(Arc<str>, Option<Arc<str>>, Option<Arc<str>>), PathIds>,
    pub pending: BTreeSet<usize>,
    pub programs: Vec<crate::check_session::refusal::Planned>,
    pub prepared: HashMap<(Query, Parameters), usize>,
    pub shared_plan_data: lower::SharedPlanData,
    pub module_catalogs: HashMap<query::Modules, Result<ryo_values::domain::program::types::ParsedModules>>,
    pub pending_programs: Option<(Arc<SelectedFiles>, Vec<(usize, usize)>)>,
    pub pairs: usize,
}

impl Schedule {
    /// A delivered source releases its project flag. Its answered open-file pairs stay
    /// addressable as the one record of completed work; closed pairs are consumed once.
    pub fn delivered(&mut self, ids: &[usize], reusable: impl Fn(usize) -> bool) {
        let mut retained = HashMap::new();
        for &i in ids {
            let source = &mut self.sources[i];
            source.bindings.keep_programs(&reusable, &mut retained);
            source.known.append(&mut source.bindings);
            source.project = false;
        }
    }
    /// The complete lowered program is cached by its source and inputs. Parsed
    /// module bodies are scratch shared by preparations in the current round.
    pub fn finish_preparation(&mut self) {
        self.module_catalogs = HashMap::new();
    }

    /// Resolve locations using the exact request overlay and the bytes verified
    /// during its file visit. A report never opens a new version from disk.
    pub fn locate_closed(
        &self, root: &str, readers: &[ReaderState], report: &mut Reported,
        coordinates: &mut HashMap<usize, l::contract::FindingCoordinates>,
    ) -> Result<()> {
        for row in &mut report.rows {
            let Some(path) = row.span_file() else { continue; };
            let Some(&id) = self.source_ids.get(root).and_then(|files| files.get(path.as_ref())) else { continue; };
            let bytes = self.sources[id].provided.as_ref()
                .or_else(|| readers.iter().find_map(|state| state.bytes.get(&id)));
            if let Some(bytes) = bytes {
                row.locate(&path, None, bytes, coordinates.entry(id).or_default())?;
            }
        }
        Ok(())
    }

    pub fn close(&mut self) {
        self.shared_plan_data = Default::default();
        self.prepared = HashMap::new();
        self.finish_preparation();
        self.source_ids = HashMap::new();
        self.supplied_ids = HashMap::new();
        self.named_disk_ids = HashMap::new();
        for source in &mut self.sources {
            source.known.clear();
        }
    }
    /// Lower the queries of this round that share a module catalog together, once each, and
    /// record them as prepared; whatever is not prepared here is prepared alone by `prepare`.
    pub fn prepare_many(&mut self, items: Vec<(Query, Parameters)>) {
        let mut groups: Vec<(Option<query::Modules>, Vec<(Query, Parameters)>)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (query, parameters) in items {
            if !query.observations.is_empty() || self.prepared.contains_key(&(query.clone(), parameters.clone())) {
                continue;
            }
            if !seen.insert((query.clone(), parameters.clone())) { continue; }
            match groups.iter_mut().find(|(key, _)| *key == query.modules) {
                Some((_, group)) => group.push((query, parameters)),
                None => groups.push((query.modules.clone(), vec![(query, parameters)])),
            }
        }
        for (key, group) in groups {
            if group.len() < 2 { continue; }
            let modules = match key {
                None => Ok(Default::default()),
                Some(sources) => self.module_catalogs.entry(sources).or_insert_with(|| group[0].0.parse_modules()).clone(),
            };
            let queries: Vec<_> = group.iter().map(|(query, parameters)| (&*query.text, parameters)).collect();
            let lowered = match modules {
                Ok(modules) => lower::query_batch(&queries, "/", modules, &mut self.shared_plan_data),
                Err(reason) => queries.iter().map(|_| Err(reason.clone().into())).collect(),
            };
            drop(queries);
            for ((query, parameters), result) in group.into_iter().zip(lowered) {
                self.programs.push(result.and_then(|(plan, typed)| Program::assemble(&query, plan, typed)));
                self.prepared.insert((query, parameters), self.programs.len() - 1);
            }
        }
    }
    pub fn prepare(
        &mut self,
        text: Query,
        parameters: Parameters,
        make: impl FnOnce(ryo_values::domain::program::types::ParsedModules, &mut lower::SharedPlanData) -> crate::check_session::refusal::Planned,
    ) -> usize {
        let catalog = text.modules.clone();
        *self.prepared.entry((text, parameters)).or_insert_with_key(|(query, _)| {
            let modules = match catalog {
                None => Ok(Default::default()),
                Some(sources) => self.module_catalogs.entry(sources).or_insert_with(|| query.parse_modules()).clone(),
            };
            self.programs.push(modules.map_err(refusal::Refusal::from).and_then(|modules| make(modules, &mut self.shared_plan_data)));
            self.programs.len() - 1
        })
    }
    pub fn source_in(
        sources: &mut Vec<Source>,
        ids: &mut PathIds,
        root: &Arc<str>,
        path: &str,
    ) -> Result<usize> {
        if let Some(&id) = ids.get(path) {
            return Ok(id);
        }
        if Path::new(path)
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(format!("invalid relative source path {path}"));
        }
        let id = sources.len();
        ids.insert(Rc::from(path), id);
        sources.push(Source {
            root: root.clone(),
            environment: None,
            path: path.to_owned(),
            reader: None,
            bindings: Default::default(),
            known: Default::default(),
            project: false,
            provided: None,
            permission: Permission::Unknown,
        });
        Ok(id)
    }

    /// Attach immutable caller bytes before admitting any consumer of this source root.
    pub fn provide(
        &mut self,
        root: &Arc<str>,
        path: &str,
        bytes: &Arc<[u8]>,
        permission: Permission,
        reader: &Option<Arc<str>>,
        environment: &Option<Arc<project::Environment>>,
    ) -> Result<(usize, Arc<[u8]>)> {
        let key = (Rc::from(path), bytes.clone(), permission, reader.clone(), environment.as_ref().map(|e| e.id.clone()));
        let (id, canonical) = match self.supplied_ids.get_key_value(&key) {
            Some(((_, held, _, _, _), &id)) => {
                // Delivery released the bytes of an earlier call on this version.
                self.sources[id].provided.get_or_insert_with(|| held.clone());
                (id, held.clone())
            }
            None => {
                let ids = self.source_ids.entry(root.clone()).or_default();
                let id = Self::source_in(&mut self.sources, ids, root, path)?;
                self.sources[id].environment = environment.clone();
                self.sources[id].provided = Some(bytes.clone());
                self.sources[id].permission = permission;
                self.sources[id].reader = reader.clone();
                self.supplied_ids.insert(key, id);
                (id, bytes.clone())
            }
        };
        self.alias_source(root, path, id)?;
        Ok((id, canonical))
    }

    /// A source whose bytes were released at delivery receives them again when a later
    /// call names the same immutable version.
    pub fn reprovide(&mut self, root: &Arc<str>, path: &str, bytes: &Arc<[u8]>) {
        if let Some(&id) = self.source_ids.get(root).and_then(|ids| ids.get(path))
            && self.sources[id].provided.is_none()
        {
            self.sources[id].provided = Some(bytes.clone());
        }
    }

    pub fn listed(&mut self, root: &Arc<str>, directory: &Arc<str>, path: &str, reader: &Option<Arc<str>>, environment: &Option<Arc<project::Environment>>) -> Result<()> {
        let ids = if reader.is_some() || environment.is_some() {
            self.named_disk_ids.entry((directory.clone(), reader.clone(), environment.as_ref().map(|e| e.id.clone()))).or_default()
        } else {
            self.source_ids.entry(directory.clone()).or_default()
        };
        let id = Self::source_in(&mut self.sources, ids, directory, path)?;
        self.sources[id].reader = reader.clone();
        self.sources[id].environment = environment.clone();
        self.alias_source(root, path, id)
    }

    pub fn alias_source(&mut self, root: &Arc<str>, path: &str, id: usize) -> Result<()> {
        let ids = self.source_ids.entry(root.clone()).or_default();
        if let Some(&prior) = ids.get(path) {
            if prior != id {
                return Err("immutable source version changed".into());
            }
        } else {
            ids.insert(Rc::from(path), id);
        }
        Ok(())
    }

    pub fn project_inputs(&mut self, root: &str, paths: &[String]) -> Result<Vec<usize>> {
        let root: Arc<str> = Arc::from(root);
        let ids = self.source_ids.entry(root.clone()).or_default();
        paths
            .iter()
            .map(|p| {
                let index = Self::source_in(&mut self.sources, ids, &root, p)?;
                self.sources[index].project = true;
                self.pending.insert(index);
                Ok(index)
            })
            .collect()
    }
    pub fn select_files(&mut self, root: &Arc<str>, files: &[String]) -> Result<Arc<SelectedFiles>> {
        let paths = self.source_ids.entry(root.clone()).or_default();
        let ids = files.iter().map(|path| Self::source_in(&mut self.sources, paths, root, path))
            .collect::<Result<Vec<_>>>()?;
        let distinct = ids.windows(2).all(|pair| pair[0] < pair[1])
            || ids.iter().collect::<std::collections::HashSet<_>>().len() == ids.len();
        Ok(Arc::new(SelectedFiles { ids, distinct }))
    }

    pub fn finish_selection(&mut self) {
        let Some((files, programs)) = self.pending_programs.take() else { return; };
        let programs: Arc<[_]> = programs.into();
        for (ordinal, &id) in files.ids.iter().enumerate() {
            let source = &mut self.sources[id];
            if source.bindings.is_empty() { self.pending.insert(id); }
            source.bindings.share(&programs, ordinal);
        }
    }

    pub fn admit_ids<T: ReportSelection>(
        &mut self, files: &Arc<SelectedFiles>, binding: usize, known_binding: bool,
    ) -> T {
        if self.pending_programs.as_ref().is_some_and(|(prior, _)| !Arc::ptr_eq(prior, files)) {
            self.finish_selection();
        }
        if !known_binding && files.distinct {
            let (_, programs) = self.pending_programs.get_or_insert_with(|| (files.clone(), Vec::new()));
            let start = self.pairs;
            self.pairs += files.len();
            programs.push((binding, start));
            return T::assigned(start..self.pairs);
        }
        // Repeated files or an existing binding can reuse an earlier address.
        self.finish_selection();
        files.ids.iter().map(|&id| {
            Self::slot(&mut self.sources[id], id, binding, known_binding, &mut self.pairs, &mut self.pending)
        }).collect()
    }

    pub fn slot(
        source: &mut Source,
        src: usize,
        binding: usize,
        known_binding: bool,
        pairs: &mut usize,
        pending: &mut BTreeSet<usize>,
    ) -> usize {
        if known_binding && let Some(slot) = source.known.lookup(binding) {
            return slot;
        }
        let first = source.bindings.is_empty();
        source.bindings.admit(binding, || {
            let slot = *pairs;
            *pairs += 1;
            if first {
                pending.insert(src);
            }
            slot
        })
    }
}

/// New bindings normally arrive in allocation order. Later rounds and overlaid
/// files may supply older IDs; preserve exactly the same canonical slot then.
pub fn admitted_slot(slots: &mut Vec<(usize, usize)>, binding: usize, new: impl FnOnce() -> usize) -> usize {
    let pos = if slots.last().is_none_or(|p| p.0 < binding) {
        slots.len()
    } else {
        match slots.binary_search_by_key(&binding, |p| p.0) {
            Ok(i) => return slots[i].1,
            Err(i) => i,
        }
    };
    let slot = new();
    slots.insert(pos, (binding, slot));
    slot
}

/// Admission assigns dense binding IDs. Only bindings used by this reader own an executor.
pub struct Bindings<'p> { index: Vec<usize>, entries: Vec<Result<Bound<'p>>> }

impl<'p> std::ops::Index<&usize> for Bindings<'p> {
    type Output = Result<Bound<'p>>;

    fn index(&self, id: &usize) -> &Self::Output {
        &self.entries[self.index[*id]]
    }
}

impl<'p> std::ops::IndexMut<&usize> for Bindings<'p> {
    fn index_mut(&mut self, id: &usize) -> &mut Self::Output {
        &mut self.entries[self.index[*id]]
    }
}

pub type FactMask = [u64; relations::service::ALL.len().div_ceil(64)];

pub fn requested_facts(mask: FactMask, out: &mut Vec<l::Relation>) {
    out.clear();
    for (group, mut bits) in mask.into_iter().enumerate() {
        while bits != 0 {
            let id = group * 64 + bits.trailing_zeros() as usize;
            out.push(relations::service::ALL[id]);
            bits &= bits - 1;
        }
    }
}

pub struct Bound<'p> {
    pub named: Vec<&'static source::FileRelation>,
    pub executable: bool,
    pub fact_mask: FactMask,
    pub requirements: Rc<BoundRequirements>,
    pub projects_words: bool,
    pub reads_fields: bool,
    pub empty_frontier: Option<frontier::EmptyFrontier>,
    pub plan: &'p Plan,
    pub slots: Rc<Vec<usize>>,
    pub test: usize,
    pub absent: Option<(u32, Vec<bool>)>,
    pub outputs: HashMap<usize,usize>,
    pub projections: BTreeMap<usize,finish::session::contribution::Projection>,
    pub facts: Vec<l::Relation>,
    pub filters: BTreeMap<usize,Vec<(usize,u64)>>,
    pub hosted: Tables,
    pub layout: Vec<(l::Relation, String)>,

    pub lines: bool,
    pub coverage: reader_run::coverage::Coverage,
}

pub fn bind<'p, L: FileSource>(
    registry: &mut finish::session::contribution::Registry,
    program: &'p Program,
    parser: Option<&L>,
    run: &mut Run,
    hosted: Tables,
    cut: Option<&finish::partition::Partition>,
    layout: Vec<(l::Relation, String)>,
    (requirements, test): (Rc<BoundRequirements>, usize),
) -> Result<Bound<'p>> {
    let mut named = Vec::new();
    let mut facts = Vec::new();
    let mut projects_words = requirements.demand().words;
    let mut reads_fields = requirements.demand().fields;
    for (name, (at, relation)) in program.bases.iter().zip(&program.base_steps) {
        if cut.is_some_and(|c| !c.file_steps[*at]) || hosted.iter().any(|(n, _)| n == name) {
            continue;
        }
        let Some(rel) = *relation else {
            let schema = parser.and_then(|p| p.declarations().iter().find(|s| s.name == name))
                .ok_or_else(|| format!("no reader declares file relation {name}"))?;
            named::declare(run, schema);
            if !named.contains(&schema) { named.push(schema); }
            continue;
        };
        if layout.iter().any(|(relation, _)| *relation == rel) {
            continue;
        }
        let name = name.as_str();
        projects_words |= matches!(rel, l::Relation::SyntaxTree | l::Relation::SemanticSyntax | l::Relation::TokenLink);
        reads_fields |= matches!(rel, l::Relation::SemanticChild | l::Relation::SyntaxChild);
        if crate::host_columns::supplies(rel) {
            let (_, names, tys) = run.catalog(name).ok_or("no engine schema")?;
            let cols: Vec<_> = names
                .iter()
                .zip(tys.iter())
                .map(|(n, t)| (n.as_str(), *t))
                .collect();
            run.declare(name, &cols, finish::Supplier::Host);
            continue;
        }
        if catalog::service::scope_of(rel) == catalog::types::RowScope::Project {
            return Err(format!("{name} needs closed project facts"));
        }
        // The language says whether the relation covers what a file embeds: a node relation it does not cover is read over the
        // file's own tree, whatever the catalog allows.
        if parser.is_some_and(|p| !p.expands(rel)) {
            run.own_only.insert(name.to_owned());
        }
        // The supplier certifies its complete projection on this exact loan.
        // Expanded positions already in the loan share its node namespace.
        if parser.is_some_and(|p| p.available(rel) && p.numbered_projections().contains(&rel))
            && finish::reads_numbered(run, name)
        {
            continue;
        }
        // The node-table relations are read from the numbered table by the executor; every other relation the language fills.
        if catalog::service::fact_form(rel) != relations::types::FactForm::NodeTable {
            if parser.is_some_and(|p| p.expands(rel)) {
                run.reader_tables.insert(name.to_owned());
            }
            // A relation of the catalog that is optional and that this provider does not advertise is not a refusal: its file filler answers
            // that the capability is unavailable, so a rule over it is an unknown at each file of the provider and never an absence.
            facts.push(rel);
        }
    }
    let filters: BTreeMap<usize, Vec<(usize, u64)>>=cut.into_iter().flat_map(|c|&c.filters).map(|(&node,alts)|{
        (node,alts.iter().map(|(col,w)|(*col,u64::from(run.sym(w)))).collect())
    }).collect();
    let projections = cut.filter(|partition| !partition.columns.is_empty()).map_or_else(
        BTreeMap::new,
        |partition| finish::session::contribution::Projection::bind(
            &mut Exec::new(&program.plan), run, Some(partition), &filters,
        ),
    );
    let outputs = registry.identify(&projections, cut);
    let mut fact_mask = FactMask::default();
    for &rel in &facts {
        let id = relations::service::number_of(rel).0;
        fact_mask[id / 64] |= 1 << (id % 64);
    }
    Ok(Bound {
        executable: program.base_steps.iter().any(|(_, rel)| *rel == Some(l::Relation::FilesystemExecutable)),
        named,
        fact_mask,
        requirements,
        reads_fields,
        projects_words,
        absent: None,
        empty_frontier: None,
        outputs,
        projections,
        filters,
        plan: &program.plan,
        slots: Rc::default(),
        test,
        facts,
        hosted,
        layout,
        lines: program
            .bases
            .iter()
            .any(|n| n == relations::service::name(l::Relation::FilesystemLineCount).0),
        coverage: Default::default(),
    })
}

pub struct Version<'s> {
    pub origin: usize,
    pub project_slots: Vec<usize>,
    pub data: Arc<[u8]>,
    pub jobs: std::borrow::Cow<'s, FilePrograms>,
    /// Why the file could not be read: it is then visited as a host-only file (no bytes, no parse) that says what the host says of it.
    pub failed: Option<String>,
}

/// A relative path in two overlaid roots has the same local answers exactly when
/// the parser sees the same bytes and the host sees the same executable mode.
/// Hard-linked listed files in the immutable snapshot need only one physical read.
pub fn versions<'s>(
    group: &[usize],
    sources: &'s [Source],
    answers: &mut (impl Slots<Answer> + ?Sized),
    aliases: &mut (impl Slots<usize> + ?Sized),
    reads: &mut usize,
    retained: &mut HashMap<usize, Arc<[u8]>>,
    needed_later: impl Fn(&Source) -> bool,
) -> Vec<Version<'s>> {
    use std::os::unix::fs::MetadataExt;
    if let [idx] = *group {
        let src = &sources[idx];
        let data = if let Some(bytes) = &src.provided {
            Ok(Arc::clone(bytes))
        } else {
            let path = Path::new(&*src.root).join(&src.path);
            (|| -> std::io::Result<Arc<[u8]>> {
                *reads += 1;
                let bytes: Arc<[u8]> = std::fs::read(path)?.into();
                // Only a pending closed reader reads these bytes again, to place its rows.
                if needed_later(src) {
                    retained.insert(idx, bytes.clone());
                }
                Ok(bytes)
            })()
        };
        return match data {
            Ok(data) => vec![Version {
                origin: idx,
                project_slots: src.project.then_some(idx).into_iter().collect(),
                data,
                jobs: std::borrow::Cow::Borrowed(&src.bindings),
                failed: None,
            }],
            // A file the reader could not read is run as a host-only file, as one no reader claims is: its read failure is the
            // host's row, its other relations are read only in part, and the file is named unread.
            Err(error) => vec![Version {
                origin: idx,
                project_slots: src.project.then_some(idx).into_iter().collect(),
                data: Arc::from(Vec::new()),
                jobs: std::borrow::Cow::Borrowed(&src.bindings),
                failed: Some(error.to_string()),
            }],
        };
    }
    let mut files = Vec::<Version>::new();
    let mut inodes = HashMap::new();
    let mut contents = BTreeMap::new();
    for &idx in group {
        let src = &sources[idx];
        let path = Path::new(&*src.root).join(&src.path);
        let found = (|| -> std::io::Result<usize> {
            let (data, inode, perm) = if let Some(bytes) = src.provided.as_ref() {
                (Arc::clone(bytes), None, None)
            } else {
                let meta = std::fs::metadata(&path)?;
                let inode = (meta.dev(), meta.ino(), src.environment.as_ref().map(|e| &e.id));
                if let Some(&i) = inodes.get(&inode) {
                    return Ok(i);
                }
                *reads += 1;
                (
                    Arc::from(std::fs::read(path)?),
                    Some(inode),
                    Some(meta.mode() & 0o111),
                )
            };
            if src.provided.is_none() && needed_later(src) {
                retained.entry(idx).or_insert_with(|| Arc::clone(&data));
            }
            // Length and executable mode usually decide before the byte comparison.
            // The ordered key also handles different files of equal length exactly.
            let key = (data.len(), src.provided.as_ref().map(|_| src.permission), perm, src.environment.as_ref().map(|e| &e.id), Arc::clone(&data));
            let i = *contents.entry(key).or_insert_with(|| {
                let i = files.len();
                files.push(Version {
                    origin: idx,
                    project_slots: Vec::new(),
                    data,
                    jobs: std::borrow::Cow::Owned(FilePrograms::default()),
                    failed: None,
                });
                i
            });
            if let Some(key) = inode {
                inodes.insert(key, i);
            }
            Ok(i)
        })();
        match found {
            Ok(i) => {
                if src.project {
                    files[i].project_slots.push(idx);
                }
                for (binding, slot) in src.bindings.iter() {
                    let origin = files[i].jobs.to_mut().admit(binding, || slot);
                    if origin != slot {
                        aliases.set(slot, origin);
                    }
                }
            }
            Err(e) => files.push(Version {
                origin: idx,
                project_slots: src.project.then_some(idx).into_iter().collect(),
                data: Arc::from(Vec::new()),
                jobs: std::borrow::Cow::Borrowed(&src.bindings),
                failed: Some(e.to_string()),
            }),
        }
    }
    files
}

/// One epoch's reader scratch. Only the bytes a pending closed reader still needs
/// outlive the file visit that read them.
#[derive(Default)]
pub struct ReaderState {
    pub run: Run,
    pub screens: pairs::Retained,
    pub bytes: HashMap<usize, Arc<[u8]>>,
}

#[derive(Default)]
pub struct HeldFile {
    pub tree: l::NodeTable,
    pub contents: FileSummary,
    pub literals: std::cell::RefCell<crate::pair_checks::LiteralPositions>,
    pub proof_atoms: crate::pair_checks::AtomAnswers,
}

/// Each binding gets its own prepared column positions for this reader and host
/// schema. Symbols and file IDs live in the reader session. Source trees never
/// escape the visit. Every request for the same pair refers to its answer slot.
pub mod worker;
use worker::Slots;

pub fn visit<L: FileSource>(
    state: &mut ReaderState,
    parser: Option<&L>,
    reader_id: Option<usize>,
    manifests: &BTreeSet<&str>,
    contributions: &mut (impl Slots<Option<Rc<project::Contribution>>> + ?Sized),
    programs: &[crate::check_session::refusal::Planned],
    program_of: &[usize],
    contracts: &[Arc<completeness::Contract>],
    cut_of: &[Option<Arc<finish::partition::Partition>>],
    layouts: (&[Option<usize>], &[Option<project::Layout>]),
    world: &mut World,
    sources: &[Source],
    selected: &[usize],
    aliases: &mut (impl Slots<usize> + ?Sized),
    answers: &mut (impl Slots<Answer> + ?Sized),
    mut host: impl FnMut(usize, &mut Run) -> Result<Tables>,
    clocks: &mut Clock,
) {
    let old_route_before = clocks.old_route;
    let before = (clocks.read_parse, clocks.read_versions, clocks.build_file, clocks.files);
    let mut fact_cpu = 0.;
    let mut run = std::mem::take(&mut state.run);
    let unknown = &{
        run.file_field_codes = true;
        run.empty = run.sym(b"");
        run.links();
        let public =
            reader::public_words(&mut run, &parser.map_or_else(Vec::new, FileSource::words));
        if let Some(p) = &parser {
            run.kind_sym = p.kinds().iter().map(|k| run.sym(k.as_bytes())).collect();
        }
        public
    };
    let mut fixed_fields_ready = false;
    debug_assert_eq!(run.paths.len(), sources.len());
    let t = thread_cpu();
    let mut needed = vec![false; program_of.len()];
    let mut seen_selections = rustc_hash::FxHashSet::default();
    for &index in selected {
        let bindings = &sources[index].bindings;
        if bindings.common_id().is_some_and(|id| !seen_selections.insert(id)) { continue; }
        for (binding, _) in bindings.iter() {
            needed[binding] = true;
        }
    }
    let (scope, screen, positions) = state.screens.merged(parser, programs, program_of, &needed, cut_of);
    clocks.bulk_screen_files += selected.len();
    let language = reader_declaration(parser);
    let mut reader_requirements: HashMap<usize, (Rc<BoundRequirements>, usize)> = HashMap::new();
    let mut requirement_ids: HashMap<Rc<BoundRequirements>, usize> = HashMap::new();
    let mut prerequisite_clauses = PrerequisiteClauses::default();
    let pattern_cache = &crate::pair_checks::SymbolMatches::default();
    clocks.prepare += thread_cpu() - t;
    let mut bound = Bindings { index: vec![usize::MAX; program_of.len()], entries: Vec::new() };
    let mut registry = finish::session::contribution::Registry::default();
    #[derive(Default)]
    struct FileDemand<'a> {
        closure_columns: [std::cell::OnceCell<frontier::ColumnDemand<'a>>; 2],
        named: Vec<&'static source::FileRelation>,
        rejected_facts: FactMask,
        closed: bool,
        facts: FactMask,
        executable: bool,
        native: bool,
        words: bool,
        fields: bool,
        has_plan: bool,
        refused: Vec<(usize, String)>,
    }
    let mut demands = rustc_hash::FxHashMap::default();
    let mut requested = Vec::new();
    let mut facts = Vec::new();
    let mut active_buffer: Vec<(usize, usize, bool)> = Vec::new();
    let mut distinct = std::cell::RefCell::new(crate::pair_checks::FileSymbols::default());
    let mut scratch = HeldFile::default();
    // The dictionary a file no reader claims is read against: it has no provider words.
    let unclaimed_symbols = l::Symbols::default();
    let mut packet = reader_run::packet::Packet::new();
    let mut empty_named = named::EmptyTables::default();
    let mut witness_tables = crate::pair_checks::WitnessTables::default();
    let scoped = parser.and_then(FileSource::scoped);
    let fixed_fields = scoped.and_then(|supplier| supplier.fields());
    let mut numbered_reader = None;
    let mut numbered_symbols = l::Symbols::default();
    let mut by_path = BTreeMap::<&str, Vec<usize>>::new();
    for &index in selected {
        by_path.entry(&sources[index].path).or_default().push(index);
    }
    let start = thread_cpu();
    let mut merged = shared::ReaderPlan::default();
    for (binding, needed) in needed.into_iter().enumerate() {
        if !needed { continue; }
        let entry = (|| {
            let program_id = program_of[binding];
            let program = programs[program_id].as_ref().map_err(Clone::clone)?;
            let tables = host(binding, &mut run)?;
            clocks.requirement_bindings += 1;
            let requirements = reader_requirements.entry(program_id)
                .or_insert_with(|| {
                    let (clauses, computed) = program.requirements(reader_id, || {
                        reader_run::file_requirements(&program.typed, &language)
                            .pop().expect("prepared program has an emit").1
                    });
                    clocks.requirement_programs += usize::from(computed);
                    let requirements = Rc::new(BoundRequirements::bind(&clauses, &mut prerequisite_clauses, |word| run.sym(word)));
                    if let Some((known, &id)) = requirement_ids.get_key_value(&requirements) {
                        return (known.clone(), id);
                    }
                    let id = requirement_ids.len();
                    requirement_ids.insert(requirements.clone(), id);
                    (requirements, id)
                })
                .clone();
            let layout = layouts.0[binding].map_or_else(Vec::new, |id| {
                let layout = layouts.1[id].as_ref().expect("live project layout");
                layout.declare(&mut run, &format!("\0layout/{id}"))
            });
            let mut b = bind(&mut registry, program, parser, &mut run, tables,
                cut_of[binding].as_deref(), layout, requirements)?;
            merged.add(&mut b, &mut run);
            Ok(b)
        })();
        bound.index[binding] = bound.entries.len();
        bound.entries.push(entry);
    }
    drop(language);
    let has_layout = bound.entries.iter().flatten().any(|bound| !bound.layout.is_empty());
    let (plan, supplied) = merged.finish();
    let mut exec = Exec::new(&plan);
    clocks.prepare += thread_cpu() - start;
    let mut carried_symbols = rustc_hash::FxHashMap::<u32, u32>::default();
    if let Some(dictionary) = reader_id.and_then(|id| world.project_words.get(id)) {
        carried_symbols.extend(unknown.iter().map(|(reason, &word)| (word, dictionary[reason])));
    }
    for group in by_path.values() {
        let file_start = clocks.measure_file_wall.then(std::time::Instant::now);
        let t = thread_cpu();
        let versions = versions(
            group,
            sources,
            answers,
            aliases,
            &mut clocks.reads,
            &mut state.bytes,
            |src| src.project || src.bindings.iter().any(|(binding, _)| cut_of[binding].is_some()),
        );
        let elapsed = thread_cpu() - t;
        clocks.read_versions += elapsed;
        clocks.read_parse += elapsed;
        for Version {
            origin: index,
            project_slots,
            data: bytes,
            jobs,
            failed,
        } in versions
        {
            clocks.unique_pairs += jobs.len();
            let source = &sources[index];
            let id = FileId(index.try_into().expect("file ID"));
            let t = thread_cpu();
            let demand = || {
                let mut demand = FileDemand::default();
                let mut named = BTreeMap::new();
                for (binding, _) in jobs.iter() {
                    match &bound[&binding] {
                        Ok(b) => {
                            demand.has_plan = true;
                            demand.closed |= cut_of[binding].is_some();
                            let accepted_only = programs[program_of[binding]].as_ref().is_ok_and(|p| p.accepted_only);
                            for &schema in &b.named {
                                named.insert(schema.name, schema);
                            }
                            if !accepted_only {
                                for (word, bits) in demand.rejected_facts.iter_mut().zip(b.fact_mask) { *word |= bits; }
                            }
                            demand.executable |= b.executable;
                            demand.native |= !b.named.is_empty() || programs[program_of[binding]].as_ref().is_ok_and(|p| p.renders);
                            for (word, bits) in demand.facts.iter_mut().zip(b.fact_mask) { *word |= bits; }
                            demand.words |= b.projects_words;
                            demand.fields |= b.reads_fields;
                        }
                        Err(reason) => demand.refused.push((binding, reason.clone())),
                    }
                }
                demand.named = named.into_values().collect();
                demand
            };
            let temporary;
            let demand = if let Some(key) = jobs.common_id() {
                &*demands.entry(key).or_insert_with(demand)
            } else {
                temporary = demand();
                &temporary
            };
            for (binding, reason) in &demand.refused {
                answers.set(jobs.lookup(*binding).expect("admitted binding"), Err(reason.clone()));
            }
            let have_plan = demand.has_plan;
            let required = demand.facts;
            let unknown_permission = demand.executable && source.provided.is_some()
                && source.permission == Permission::Unknown;
            let needs_native = demand.native;
            let projects_words = demand.words;
            let projects_fields = demand.fields;
            clocks.prepare += thread_cpu() - t;
            requested_facts(required, &mut requested);
            if project_slots.is_empty() && !have_plan {
                continue;
            }
            let t = thread_cpu();
            run.supplied_permission = source.provided.as_ref().map(|_| (id.0, source.permission));
            run.read_failure = failed.clone().map(|reason| (id.0, reason));
            run.root_dir.clear();
            run.root_dir.push_str(&source.root);
            // A relation read in this file's visit, from its own notes, asks for them as the project phase's relations do.
            let visit_notes = jobs.iter().any(|(binding, _)| {
                layouts.0[binding].and_then(|layout| layouts.1[layout].as_ref()).is_some_and(project::Layout::reads_in_visit)
            });
            let needs_notes = visit_notes || (!project_slots.is_empty() && contributions.previous(index).and_then(Option::as_ref).is_none());
            let request = l::ParseRequest {
                file: id,
                path: SourcePath(&source.path),
                bytes: ByteSlice(&bytes),
                fields: if projects_fields {
                    l::FieldRequest::Wanted
                } else {
                    l::FieldRequest::Unwanted
                },
                notes: if !needs_notes {
                    l::NoteRequest::Unwanted
                } else {
                    l::NoteRequest::Wanted
                },
            };
            if projects_words { requested.push(l::Relation::SyntaxTree); }
            let mut consume = |loan: Option<(l::NumberedFile<'_>, &mut l::Symbols)>| {
                let (scoped_tree, mut scoped_facts, mut scoped_symbols, mut fact_words, words, fields, coverage, notes) = match loan {
                    Some((file, symbols)) => {
                        use ryo_execution_domain::domain::coverage::types::Recovery;
                        use l::SyntaxCompleteness::Incomplete;
                        let coverage = match (file.syntax, file.lexical) {
                            (Incomplete, Incomplete) => Recovery::SyntaxErrors,
                            (Incomplete, _) => Recovery::LexicallyCompleteSyntaxErrors,
                            _ => Recovery::Whole,
                        };
                        (Some(file.tree), Some(file.facts), Some(symbols), Some(file.fact_words), file.words,
                            Some(file.fields), Some(coverage), Some(file.notes))
                    }
                    None => (None, None, None, None, &[][..], None, None, None),
                };
                clocks.old_route += usize::from(scoped_tree.is_none());
                let held = &mut scratch;
                let mut tree = &mut held.tree;
            // A file no registered reader claims still has its host populations and no source reader contributes syntax or
            // semantic rows: no numbered file is lent for it.
            clocks.files += usize::from(scoped_tree.is_some());
            if !project_slots.is_empty() {
                let c = contributions.previous(index).and_then(Option::as_ref).cloned().unwrap_or_else(|| {
                    let manifest = (failed.is_none() && manifests
                        .contains(source.path.rsplit('/').next().unwrap()))
                        .then(|| l::ManifestText(String::from_utf8_lossy(&bytes).into_owned().into()));
                    Rc::new(project::Contribution { reader: reader_id, notes: notes.cloned().unwrap_or_default(), manifest })
                });
                for &slot in &project_slots {
                    contributions.set(slot, Some(c.clone()));
                }
                if jobs.iter().all(|(b, _)| bound[&b].is_err()) {
                    clocks.read_parse += thread_cpu() - t;
                    return;
                }
            }
            let mut provider = reader::ReaderFacts::new(id.0, &unknown);
            let mut resolved_words = reader_run::packet::FileSymbols::default();
            provider.projects_words = projects_words;
            provider.projects_fields = projects_fields;
            if let Some(fields) = fields {
                if projects_fields && !fixed_fields_ready {
                    let names = fixed_fields.unwrap_or(fields);
                    run.field_sym = names.0.iter().map(|name| run.sym(name.0.as_bytes())).collect();
                    fixed_fields_ready = fixed_fields.is_some();
                }
                if projects_words {
                    let dict = scoped_symbols.as_deref().expect("scoped dictionary");
                    provider.text.extend(words.iter().map(|&word| {
                        resolved_words.resolve(&mut run, unknown, dict, &bytes, word)
                    }));
                }
            }
            clocks.read_parse += thread_cpu() - t;
            let t = thread_cpu();
            let tree = scoped_tree.unwrap_or(tree);
            let contents = &mut held.contents;
            contents.reset();
            use ryo_execution_domain::domain::coverage::service::partial_of;
            let recovery = coverage.unwrap_or(ryo_execution_domain::domain::coverage::types::Recovery::Whole);
            let rejected = l::node_table::acceptance_of(tree) == l::SyntaxAcceptance::Rejected;
            distinct.get_mut().clear();
            let mut active = std::mem::take(&mut active_buffer);
            active.clear();
            active.reserve(jobs.len());
            for (binding, slot) in jobs.iter() {
                let Ok(b) = &bound[&binding] else { continue };
                if programs[program_of[binding]].as_ref().is_ok_and(|p| p.accepted_only)
                    && rejected
                {
                    if let Some(cut) = &cut_of[binding] {
                        let b = bound[&binding].as_mut().unwrap();
                        world.insert_empty(slot, &source.path, b, &mut run, cut);
                    }
                    answers.set(slot, Ok(Reported::new(Vec::new(), Vec::new(), vec![source.path.clone()])));
                    clocks.skipped += 1;
                    continue;
                }
                let numbered_complete = parser.is_some()
                    && recovery == ryo_execution_domain::domain::coverage::types::Recovery::Whole;
                let excluded = failed.is_none() && cut_of[binding].is_none()
                    && contents.excludes(b.test, &b.requirements, |out| {
                        out.populate(
                            tree,
                            &mut run,
                            Demand {
                                kinds: numbered_complete,
                                fields: numbered_complete && provider.projects_fields,
                                words: numbered_complete && provider.projects_words,
                            },
                            |words| words.extend(distinct.get_mut().words(&provider.text).iter().copied()),
                        )
                    });
                if excluded && b.facts.is_empty() && b.named.is_empty() && !unknown_permission {
                    let whole = recovery == ryo_execution_domain::domain::coverage::types::Recovery::Whole;
                    if whole || {
                        let rels: Vec<_> = contracts[binding].needs.reads.iter()
                            .map(|r| r.relation()).collect();
                        partial_of(&[], recovery, &rels).is_empty()
                    } {
                        answers.set_empty(slot, || Ok(Reported::default()));
                        clocks.skipped += 1;
                        continue;
                    }
                }
                active.push((binding, slot, excluded));
            }
            clocks.screen += thread_cpu() - t;
            let t = thread_cpu();
            // The content screen above only retires programs with no provider dependencies.
            // Syntax acceptance selects the remaining dependency set for this admission.
            requested_facts(if rejected { demand.rejected_facts } else { demand.facts }, &mut facts);
            packet.clear();
            // A file whose every relation is a complete answer fills a relation the first time a step loads it,
            // except the node-table relations and the ones the screen asks about before it loads a column.
            let lazy_fills = scoped_facts.as_deref_mut().is_some_and(|supplier| supplier.fills_are_complete());
            let missing: Vec<_> = if lazy_fills {
                let asked = screen.asked_first();
                // A relation the language never answers is recorded up front, as the fill of every relation up front records it
                // (`NotYetRead` in the file's partial coverage, any other cause as no facts): the fill on first use never meets one.
                facts.iter().copied().filter(|&rel| asked.contains(&rel)
                    || parser.as_ref().is_some_and(|p| p.absent(rel))
                    || ryo_relations::domain::catalog::service::fact_form(rel) == ryo_relations::domain::relations::types::FactForm::NodeTable).collect()
            } else {
                facts.clone()
            };
            if let Some(supplier) = scoped_facts.as_deref_mut()
                && !missing.is_empty()
            {
                let fact_start = thread_cpu();
                let filled = packet.fill_numbered(supplier, &missing, fact_words.as_deref_mut().expect("file word dictionary"));
                fact_cpu += thread_cpu() - fact_start;
                if let Err(reason) = filled {
                    for (_, slot) in jobs.iter() {
                        answers.set(slot, Err(reason.clone()));
                    }
                    return;
                }
            }
            let partials = std::mem::take(&mut packet.partials);
            use ryo_values::domain::stored_facts::types::{
                PartialAt, PartialCause, PartialScope, ReadInPart,
            };
            let mut said: Vec<_> = partials
                .iter()
                .map(|(relation, why, lacking)| ReadInPart {
                    relation: *relation,
                    cause: PartialCause(why.as_str().into()),
                    lacking: *lacking,
                    at: PartialScope::OneFile(PartialAt(id.0)),
                })
                .collect();
            // A file the host could not read has no relation but the host's own: every other relation the programs read is read only in
            // part, for the reason the host gives.
            if let Some(reason) = &failed {
                let mut relations = BTreeSet::new();
                for (binding, _) in jobs.iter() {
                    relations.extend(contracts[binding].needs.reads.iter().map(|read| read.relation()));
                }
                for relation in relations {
                    if matches!(relation, l::Relation::FilesystemFile | l::Relation::FilesystemReadFailure) {
                        continue;
                    }
                    said.push(ReadInPart {
                        relation,
                        cause: PartialCause(reason.as_str().into()),
                        lacking: ryo_relations::domain::relations::types::Missing::Any,
                        at: PartialScope::OneFile(PartialAt(id.0)),
                    });
                }
            }
            packet.lazy_fills = lazy_fills;
            let supplier_cell = scoped_facts.take().map(std::cell::RefCell::new);
            let lazy = supplier_cell.as_ref().filter(|_| lazy_fills).map(reader_run::packet::Lazy::new);
            if unknown_permission {
                said.push(ReadInPart {
                    relation: l::Relation::FilesystemExecutable,
                    cause: PartialCause("input edge did not supply executable permission metadata".into()),
                    lacking: ryo_relations::domain::relations::types::Missing::Any,
                    at: PartialScope::OneFile(PartialAt(id.0)),
                });
            }
            packet.index(&facts);
            let mut certified = HashMap::new();
            let version = std::cell::OnceCell::new();
            clocks.read_parse += thread_cpu() - t;
            let t = thread_cpu();
            if recovery == ryo_execution_domain::domain::coverage::types::Recovery::Whole
            {
                held.literals.get_mut().clear();
                held.proof_atoms.clear();
                let scanned = held.literals.get_mut().scanned();
                let patterns = crate::pair_checks::Predicates { cache: pattern_cache, scope };
                screen.exclude(&positions, &mut active, &mut bound, &mut run, tree, provider.projects_words.then_some(provider.text.as_slice()), &distinct, &held.literals, patterns, &mut held.proof_atoms,
                    &bytes, &source.path, id, scoped_symbols.as_deref().unwrap_or(&unclaimed_symbols), fact_words.as_deref_mut(), lazy.as_ref(), &mut witness_tables, &facts, &packet, unknown, cut_of, &said, layouts.1, &mut clocks.witnesses);
                for &(binding, slot, excluded) in active.iter() {
                        if excluded { continue; }
                        let (any, all) = screen.exact_roots[positions[binding]];
                        clocks.exact_inputs[0] += 1;
                        clocks.exact_inputs[1] += usize::from(any);
                        if !all { continue; }
                        let b = bound[&binding].as_ref().unwrap();
                        clocks.exact_inputs[2] += 1;
                        clocks.exact_inputs[3] += usize::from(provider.projects_words);
                        clocks.exact_inputs[4] += usize::from(said.iter().any(|part| b.facts.contains(&part.relation)));
                        clocks.exact_inputs[5] += usize::from(!b.named.is_empty());
                        clocks.exact_inputs[6] += usize::from(cut_of[binding].is_some());
                        clocks.exact_inputs[7] += usize::from(!said.is_empty());
                        if !said.is_empty() || !provider.projects_words || cut_of[binding].is_some() || !b.layout.is_empty() { continue; }
                        if let Some(locations) = screen.exact_locations(positions[binding]) {
                            let mut rows = Vec::new();
                            let result = b.plan.emits.iter().zip(locations).enumerate().try_for_each(
                                |(emit, ((rule, _, subject, keys), (columns, mask)))| finish::session::publish_locations(
                                    &mut exec, &mut run, (binding, emit), rule, subject, keys, columns, mask, tree, id.0, b.plan.every_row, &mut rows));
                            certified.insert(slot, result.map(|()| Reported::from(rows)));
                        }
                }
                clocks.literal_positions += held.literals.get_mut().scanned() - scanned;
            }
            let proof_time = thread_cpu() - t;
            clocks.screen += proof_time;
            clocks.bound_proofs += proof_time;
            let extent = refusal::Extent {
                provider: parser.as_ref().map_or("", |p| p.name()),
                home: id.0,
                elements: l::node_table::element_count(tree).0 as u64,
                bytes: bytes.len() as u64,
            };
            let mut named_tables = Vec::new();
            let mut named_errors = BTreeMap::new();
            let mut named_partials = BTreeMap::new();
            if needs_native && let Some(supplier) = &supplier_cell {
                // Only a program that survived the screen reads its named tables.
                let mut surviving = BTreeMap::new();
                for &(binding, _, excluded) in &active {
                    if let (false, Ok(b)) = (excluded, &bound[&binding]) {
                        surviving.extend(b.named.iter().map(|&schema| (schema.name, schema)));
                    }
                }
                let schemas: Vec<_> = surviving.into_values().collect();
                let schemas = &schemas;
                if !schemas.is_empty() {
                    let named_start = thread_cpu();
                    clocks.named_counts[0] += 1;
                    let answers: Vec<_> = supplier.borrow_mut().named(schemas, scoped_symbols.as_deref_mut().expect("scoped dictionary"))
                        .into_iter().map(|result| result.map_err(|e| e.to_string())).collect();
                    if answers.len() != schemas.len() {
                        for schema in schemas {
                            named_errors.insert(schema.name, "provider returned the wrong number of named tables".to_owned());
                        }
                    } else {
                        for (schema, answer) in schemas.into_iter().zip(answers) {
                            let table = answer.and_then(|rows| {
                                if let Some(reason) = rows.partial {
                                    named_partials.insert(schema.name, reason.0);
                                }
                                named::table(&mut empty_named, rows.columns, schema, id, scoped_symbols.as_deref().unwrap_or(&unclaimed_symbols), &bytes, &mut run, unknown, &mut resolved_words, extent)
                            });
                            match table {
                                Ok(table) => {
                                    clocks.named_counts[1] += 1;
                                    clocks.named_counts[2] += table.n;
                                    clocks.named_counts[3] += usize::from(table.n == 0);
                                    named_tables.push((schema.name.to_owned(), table));
                                }
                                Err(error) => { named_errors.insert(schema.name, error); }
                            }
                        }
                    }
                    let elapsed = thread_cpu() - named_start;
                    clocks.named_cpu += elapsed;
                    clocks.read_parse += elapsed;
                }
            }
            let dictionary = fact_words.map(std::cell::RefCell::new);
            let mut render = |request: l::RenderRequest| {
                let Some(supplier) = &supplier_cell else { return Err("no numbered file to render from".to_owned()); };
                supplier.borrow_mut().render(request).map_err(|e| e.to_string())
            };
            let loan = Some(packet.operator_columns(id, extent, dictionary.as_ref(), lazy.as_ref(),
                &mut run));
            // The frame owns the reader's projected word buffer for this visit.
            let mut frame_words = provider.projects_words.then(|| std::mem::take(&mut provider.text));
            let mut facts_view = packet.view(&mut provider, scoped_symbols.as_deref().unwrap_or(&unclaimed_symbols), &bytes, dictionary.as_ref(), lazy.as_ref());
            facts_view.resolved = resolved_words;
            facts_view.renderer = Some(&mut render);
            let t = thread_cpu();
            let lines = active
                .iter()
                .any(|(b, _, _)| bound[b].as_ref().unwrap().lines)
                .then(|| {
                    let name = relations::service::name(l::Relation::FilesystemLineCount).0;
                    let (_, columns, types) =
                        run.catalog(name).expect("host line-count declaration");
                    let mut table = Table::with_rows(columns, types, 1);
                    table.cols[0].push(u64::from(id.0));
                    table.cols[1].push(
                        bytes.iter().filter(|&&b| b == b'\n').count() as u64
                            + u64::from(bytes.last().is_some_and(|&b| b != b'\n')),
                    );
                    table.n = 1;
                    (name.to_owned(), Rc::new(table))
                });
            let mut project_tables = rustc_hash::FxHashMap::default();
            if has_layout {
                for &(binding, _, excluded) in &active {
                    if excluded { continue; }
                    let bound = bound[&binding].as_ref().unwrap();
                    if !bound.layout.is_empty() {
                        let id_of_layout = layouts.0[binding].expect("bound layout index");
                        if !project_tables.contains_key(&id_of_layout) {
                            let layout = layouts.1[id_of_layout].as_ref().expect("live project layout");
                            // The relations that need the file's notes are read now, from this file's own: the visit asked for them.
                            let read = if layout.reads_in_visit() {
                                let none = l::Bytes::default();
                                match layout.read_file(parser, unknown, &mut run, &bound.layout, &source.path, id, notes.unwrap_or(&none)) {
                                    Ok(read) => {
                                        clocks.visit_projects += 1;
                                        for (relation, cause, lacking) in &read.partials {
                                            said.push(ReadInPart {
                                                relation: *relation,
                                                cause: PartialCause(cause.as_str().into()),
                                                lacking: *lacking,
                                                at: PartialScope::OneFile(PartialAt(id.0)),
                                            });
                                        }
                                        Some(read)
                                    }
                                    Err(reason) => {
                                        for (_, slot) in jobs.iter() {
                                            answers.set(slot, Err(reason.clone()));
                                        }
                                        return;
                                    }
                                }
                            } else {
                                None
                            };
                            let tables = layout.slice(&mut run, &bound.layout, &source.path, id, read.as_ref());
                            project_tables.insert(id_of_layout, tables);
                        }
                    }
                }
            }
            let mut coordinates = l::contract::FindingCoordinates::default();
            let named_unread = |report: Reported| match &failed {
                Some(reason) => report.with_unread(vec![(source.path.clone(), reason.clone())]),
                None => report,
            };
            let mut locate = |mut report: Reported| -> Result<Reported> {
                report = named_unread(report);
                for row in &mut report.rows { row.locate(&source.path, Some(tree), &bytes, &mut coordinates)?; }
                Ok(report)
            };
            let mut frame = None;
            let mut shared = demand.closed.then(|| {
                // Syntax rejection can remove closed programs that require an accepted tree.
                // The other screens retain their slots for closure, even for zero rows.
                let columns = demand.closure_columns[usize::from(rejected)].get_or_init(||
                    frontier::FileInputs::column_demand(&active, &bound, programs, program_of, cut_of)
                );
                frontier::FileInputs::new(columns)
            });
            for (binding, slot, excluded) in active.drain(..) {
                use ryo_execution_domain::domain::trust::{service::matters, types::Bearing};
                let contract = &contracts[binding];
                let b = bound[&binding].as_mut().unwrap();
                if let Some(reason) = b.named.iter().find_map(|schema| named_errors.get(schema.name)) {
                    answers.set(slot, Err(reason.clone()));
                    clocks.refused += 1;
                    continue;
                }
                let complete_base = |name: &str| {
                    use finish::session::CompleteBase;
                    if let Some((_, rows)) = named_tables.iter().find(|(n, _)| n == name) {
                        return (!named_partials.contains_key(name)).then_some(CompleteBase::Count(rows.n));
                    }
                    let rel = relations::service::relation_named(NameRef(name))?;
                    let reader = parser?;
                    if !reader.available(rel) || reader.expands(rel)
                        || ryo_relations::domain::catalog::service::node_extent(rel)
                            != Some(ryo_relations::domain::relations::types::NodeExtent::Own)
                        || partial_of(&said, recovery, &[rel]).iter().any(|p| p.relation == rel)
                    { return None; }
                    Some(CompleteBase::InFrame)
                };
                let mut local_coverage = BTreeMap::new();
                let mut issue_bases = Vec::new();
                let mut unknowns = if said.is_empty()
                    && recovery == ryo_execution_domain::domain::coverage::types::Recovery::Whole
                {
                    Vec::new()
                } else {
                    let rels: Vec<_> = contract.needs.reads.iter().map(|r| r.relation()).collect();
                    let partial = partial_of(&said, recovery, &rels);
                    for each in &partial {
                        let name = relations::service::name(each.relation).0;
                        if contract
                            .needs
                            .per_file
                            .iter()
                            .any(|r| matters(r, each) == Bearing::Matters)
                        {
                            local_coverage
                                .entry(name.to_owned())
                                .or_insert_with(|| vec![Some(id.0)]);
                        }
                        if contract
                            .needs
                            .cross
                            .iter()
                            .any(|r| matters(r, each) == Bearing::Matters)
                        {
                            let file = world.file(&source.path);
                            world
                                .unavailable
                                .entry(slot)
                                .or_default()
                                .insert((name.to_owned(), file));
                        }
                    }
                    contract.report(&partial, &source.path, |base| {
                        if cut_of[binding].is_some() { issue_bases.push(base); }
                    })
                };
                for schema in &b.named {
                    if let Some(reason) = named_partials.get(schema.name) {
                        local_coverage.insert(schema.name.to_owned(), vec![Some(id.0)]);
                        if cut_of[binding].is_some() {
                            issue_bases.push(schema.name);
                            let origin = world.file(&source.path);
                            world.unavailable.entry(slot).or_default().insert((schema.name.to_owned(), origin));
                        }
                        unknowns.push((source.path.clone(), reason.to_string()));
                    }
                }
                if excluded && unknowns.is_empty() {
                    if let Some(cut) = &cut_of[binding] {
                        world.insert_empty(slot, &source.path, b, &mut run, cut);
                    }
                    answers.set_empty(slot, || Ok(Reported::default()));
                    clocks.skipped += 1;
                    continue;
                }
                // Named suppliers have now answered, including coverage. A partial
                // population still goes through the scoped evaluator.
                if unknowns.is_empty() && let Some(result) = certified.remove(&slot) {
                    let result = result.and_then(&mut locate);
                    clocks.certified += 1;
                    clocks.productive += usize::from(result.as_ref().is_ok_and(|r| !r.rows.is_empty()));
                    clocks.empty += usize::from(result.as_ref().is_ok_and(|r| r.rows.is_empty()));
                    clocks.refused += usize::from(result.is_err());
                    answers.set(slot, result);
                    continue;
                }
                clocks.evaluated += 1;
                let version = version.get_or_init(|| {
                    finish::FileVersion::new(&tree, &bytes, id.0, frame_words.take())
                });
                let make_frame = || finish::session::frame(
                    version,
                    &plan,
                    &supplied,
                    loan.as_ref().map(|l| l as &dyn finish::fact_view::FileFacts),
                    lines.iter().chain(&named_tables).chain(project_tables.values().flatten()),
                );
                let mut restricted;
                let current = if local_coverage.is_empty() {
                    frame.get_or_insert_with(make_frame)
                } else {
                    restricted = make_frame();
                    &mut restricted
                };
                let _profile = super::finish::operators::scope(program_of[binding], "file");
                if let Some(cut) = &cut_of[binding] {
                    let value = shared.as_mut().expect("project output was bound").contribute(b, &mut exec, current, &mut run, id.0, &mut facts_view, cut,
                        &local_coverage, world, &mut carried_symbols, reader_id, unknown, slot);
                    if value.is_ok() && !unknowns.is_empty()
                        && let Some(reachable) = finish::session::reachable_partials(
                            &mut exec, current, &mut run, &mut facts_view, b.plan, &b.slots, &cut.file_steps,
                            &complete_base,
                        )
                    {
                        debug_assert_eq!(unknowns.len(), issue_bases.len());
                        unknowns = unknowns.into_iter().zip(issue_bases).filter_map(|(issue, base)|
                            reachable.contains(base).then_some(issue)).collect();
                        let origin = world.file(&source.path);
                        if let Some(notes) = world.unavailable.get_mut(&slot) {
                            notes.retain(|(name, file)| *file != origin || reachable.contains(name));
                        }
                    }
                    if value.is_err() { frame = None; exec.release_request(); }
                    let answer = value.map(|()| {
                        named_unread(Reported::new(Vec::new(), unknowns, Vec::new()))
                    });
                    let productive = world.productive(slot);
                    clocks.productive += usize::from(answer.is_ok() && productive);
                    clocks.empty += usize::from(answer.is_ok() && !productive);
                    clocks.refused += usize::from(answer.is_err());
                    answers.set(slot, answer);
                    continue;
                }
                let value = finish::session::pair(&mut exec, current, &mut run, &mut facts_view, b.plan, binding, &b.slots, &local_coverage);
                if !unknowns.is_empty() && value.as_ref().is_ok_and(|rows| rows.is_empty())
                    && finish::session::covered_empty(&mut exec, current, &mut run, &mut facts_view, b.plan, &b.slots, &complete_base)
                {
                    unknowns.clear();
                }
                if value.is_err() { frame = None; exec.release_request(); }
                let answer = value
                .map(|rows| Reported::new(rows, unknowns, Vec::new())).and_then(&mut locate);
                clocks.productive += usize::from(answer.as_ref().is_ok_and(|r| !r.rows.is_empty()));
                clocks.empty += usize::from(answer.as_ref().is_ok_and(|r| r.rows.is_empty()));
                clocks.refused += usize::from(answer.is_err());
                    answers.set(slot, answer);
            }
            drop(frame);
            if lazy.as_ref().is_some_and(|lazy| lazy.failed.get()) {
                for (_, slot) in jobs.iter() {
                    answers.set(slot, Err("a relation filled on first use was not a complete answer".to_owned()));
                }
            }
            active_buffer = active;
            clocks.evaluate += thread_cpu() - t;
            };
            let mut spent_in_visit = 0.;
            let before_visit = scoped.map(|_| thread_cpu());
            let mut called = false;
            let result = {
                scoped.filter(|_| failed.is_none()).map(|supplier| {
                    ryo_language::domain::symbols::service::clear_symbols(&mut numbered_symbols);
                    numbered_reader.get_or_insert_with(|| supplier.open()).visit(
                        request,
                        &requested,
                        &demand.named,
                        source.environment.as_deref().map(|e| e as &dyn l::FileProject),
                        &mut numbered_symbols,
                        &mut |file, dictionary| {
                            assert!(!called, "a file version was supplied twice");
                            called = true;
                            let start = thread_cpu();
                            consume(Some((file, dictionary)));
                            spent_in_visit = thread_cpu() - start;
                        },
                    )
                })
            };
            match result {
                None if !called => consume(None),
                Some(Ok(())) if called => {}
                result => {
                    let reason = result.and_then(|result| result.err()).map_or_else(
                        || "provider did not deliver exactly one numbered file".to_owned(), |e| e.to_string());
                    for (_, slot) in jobs.iter() { answers.set(slot, Err(reason.clone())); }
                }
            }
            if let Some(before_visit) = before_visit {
                // The supplier's elapsed clock spans the callback. The common
                // visit has already charged its inspection and execution.
                clocks.build_file += (thread_cpu() - before_visit - spent_in_visit).max(0.);
            }
        }
        if let Some(start) = file_start {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed > clocks.largest_file_wall {
                clocks.largest_file_wall = elapsed;
                clocks.largest_file.clone_from(&sources[group[0]].path);
            }
        }
        if let Some(boundaries) = &mut world.boundaries {
            boundaries.push((group[0], world.run.paths.len(), world.run.texts.len()));
        }
    }
    let parts = clocks.readers.entry(parser.map_or("unclaimed", FileSource::name).to_owned()).or_default();
    parts.total += clocks.read_parse - before.0;
    parts.read += clocks.read_versions - before.1;
    parts.build += clocks.build_file - before.2;
    parts.parsed += clocks.files - before.3;
    parts.old_route += clocks.old_route - old_route_before;
    parts.facts += fact_cpu;
    clocks.node_runs += exec.stats.total_runs;
    if std::env::var_os("VEC_STEP_TIME").is_some() { eprintln!("SHARED_STEPS runs={} rows={}",exec.stats.total_runs,exec.stats.total_rows); }
    if std::env::var_os("VEC_PATTERN_COUNTS").is_some() {
        use std::sync::atomic::Ordering::Relaxed;
        eprintln!("PATTERN_MATCHERS first_uses={} compiled={}", crate::pair_summary::regexes::INITIALIZED.load(Relaxed), crate::pair_summary::regexes::BUILT.load(Relaxed));
    }
}
