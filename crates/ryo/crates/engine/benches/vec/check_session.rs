//! Schedule all consumers of a source before walking it. Keep the emitted rows per
//! bound program and source version. Parser scratch is released after its last consumer.
use super::{finish, first, lower, reader, reader_run, thread_cpu};
use finish::{Exec, Plan, Provider, Run, Table};
use ryo::languages::{self as l, ByteSlice, FileId, SourcePath};
use ryo_relations::domain::{catalog, labels::types::NameRef, relations};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
    rc::Rc,
    sync::Arc,
};

#[path = "check_session/completeness.rs"]
pub(crate) mod completeness;
#[path = "check_session/pairs.rs"]
mod pairs;
#[path = "check_session/project.rs"]
pub(crate) mod project;

#[path = "check_session/source.rs"]
pub(crate) mod source;
use source::Source as FileSource;
#[path = "check_session/frontier.rs"]
mod frontier;
pub(crate) use frontier::World;
#[cfg(feature = "product-session")]
pub(crate) use frontier::SharedFrontier;
#[path = "check_session/shared.rs"]
mod shared;

#[path = "check_session/retention.rs"]
mod retention;
pub(crate) use retention::Retention;

#[path="check_session/stored_block.rs"]
pub(crate) mod stored_block;

type Parameters =
    BTreeMap<Box<str>, ryo_values::domain::finish_plans::expressions::types::PlanConstant>;

type Result<T> = std::result::Result<T, String>;
pub(crate) type Answer = Result<Reported>;

#[derive(Clone, Default)]
pub(crate) struct Reported {
    pub rows: Vec<finish::answer::Row>,
    pub unknowns: Vec<(String, String)>,
}
impl Reported {
    pub(crate) fn skipped(paths: &[String]) -> Self {
        let error = ryo_execution_domain::domain::operating::errors::OperatingError::ObservationIncomplete {
            reason: "every source partition was explicitly skipped".into(),
        };
        let reason = format!("project phase diagnostic emit: {error}");
        Self {
            rows: Vec::new(),
            unknowns: paths.iter().map(|p| (p.clone(), reason.clone())).collect(),
        }
    }
}
impl From<Vec<finish::answer::Row>> for Reported {
    fn from(rows: Vec<finish::answer::Row>) -> Self {
        Self {
            rows,
            ..Default::default()
        }
    }
}
pub(crate) type Tables = Vec<(String, Rc<Table>)>;

pub(crate) struct Program {
    typed: Arc<ryo_values::domain::finish_plans::types::FinishPlan>,
    pub plan: Plan,
    pub syntax: Box<ryo_values::domain::program::types::WrittenProgram>,
    pub bases: Vec<String>,
    base_steps: Vec<(usize, Option<l::Relation>)>,
    requirements: (bool, Vec<Vec<String>>, Vec<Vec<String>>),
}

impl Program {
    pub(crate) fn prepare(
        alias: &str,
        text: &str,
        context: ryo_engine::domain::plan_lowering::types::RuleContext,
        declaration: &first::Language,
        capture_separator: Option<&str>,
    ) -> Result<Self> {
        let (plan, pack) = lower::query_parts(alias, text, context, true, capture_separator)?;
        let typed = pack.typed.take().expect("prepared query");
        let requirements = reader_run::file_requirements(&typed, declaration)
            .pop()
            .ok_or("missing query emit")?
            .1;
        let (bases, base_steps) = plan.nodes.iter().enumerate().filter_map(|(offset, step)| {
            step.base().map(|name| {
                (name.to_owned(), (offset, relations::service::relation_named(NameRef(name))))
            })
        }).unzip();
        let syntax = match pack
            .programs
            .rules
            .into_iter()
            .next()
            .ok_or("missing parsed query")?
            .program
        {
            ryo_engine::domain::plan_lowering::types::RuleProgram::Parsed(root) => root,
            _ => return Err("unread query".into()),
        };
        Ok(Self {
            plan,
            syntax,
            typed,
            requirements,
            bases,
            base_steps,
        })
    }
}

#[derive(Default)]
pub(crate) struct Clock {
    pub(crate) readers: BTreeMap<String, ReaderParts>,
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
    pub(crate) read_versions: f64,
    pub(crate) build_file: f64,
    pub screen: f64,
    pub bound_proofs: f64,
    pub literal_positions: usize,
    pub requirement_programs: usize,
    pub requirement_bindings: usize,
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
    pub productive: usize,
    pub refused: usize,
}

#[derive(Default)]
pub(crate) struct ReaderParts {
    pub total: f64,
    pub read: f64,
    pub build: f64,
    pub tree: f64,
    pub facts: f64,
    pub parsed: usize,
}

/// The root and path identify the caller's immutable effective source. This
/// distinguishes in-memory replacements staged at different roots.
pub(crate) struct Source {
    pub root: Arc<str>,
    pub path: String,
    pub bindings: BTreeMap<usize, usize>,
    known: BTreeMap<usize, usize>,
    pub(crate) project: bool,
    pub(crate) provided: Option<Arc<[u8]>>,
}

type PathIds = HashMap<Rc<str>, usize>;

/// Per-check program identities and the inverse admission index. The caller binds
/// external relations and hands the index a stable identity for each binding.
#[derive(Default)]
pub(crate) struct Schedule {
    pub sources: Vec<Source>,
    source_ids: HashMap<Arc<str>, PathIds>,
    supplied_ids: HashMap<(Rc<str>, Arc<[u8]>), usize>,
    pub pending: BTreeSet<usize>,
    pub programs: Vec<Result<Program>>,
    pub(crate) prepared: HashMap<(Rc<str>, Parameters), usize>,
    pub pairs: usize,
}

impl Schedule {
    pub(crate) fn delivered(&mut self, ids: &[usize], keep: bool) {
        for &i in ids {
            let source = &mut self.sources[i];
            if keep && source.known.is_empty() {
                source.known = std::mem::take(&mut source.bindings);
            } else if keep {
                source.known.extend(std::mem::take(&mut source.bindings));
            } else {
                source.bindings.clear();
            }
            source.project = false;
        }
    }
    pub(crate) fn close(&mut self) {
        self.prepared = HashMap::new();
        self.source_ids = HashMap::new();
        self.supplied_ids = HashMap::new();
        for source in &mut self.sources {
            source.known.clear();
        }
    }
    pub(crate) fn prepare(
        &mut self,
        text: Rc<str>,
        parameters: Parameters,
        make: impl FnOnce() -> Result<Program>,
    ) -> usize {
        *self.prepared.entry((text, parameters)).or_insert_with(|| {
            self.programs.push(make());
            self.programs.len() - 1
        })
    }
    fn source_in(
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
            path: path.to_owned(),
            bindings: BTreeMap::new(),
            known: BTreeMap::new(),
            project: false,
            provided: None,
        });
        Ok(id)
    }

    /// Attach immutable caller bytes before admitting any consumer of this source root.
    pub(crate) fn provide(
        &mut self,
        root: &Arc<str>,
        path: &str,
        bytes: &Arc<[u8]>,
    ) -> Result<Arc<[u8]>> {
        let key = (Rc::from(path), bytes.clone());
        let (id, canonical) = match self.supplied_ids.get_key_value(&key) {
            Some(((_, held), &id)) => (id, held.clone()),
            None => {
                let ids = self.source_ids.entry(root.clone()).or_default();
                let id = Self::source_in(&mut self.sources, ids, root, path)?;
                self.sources[id].provided = Some(bytes.clone());
                self.supplied_ids.insert(key, id);
                (id, bytes.clone())
            }
        };
        self.alias_source(root, path, id)?;
        Ok(canonical)
    }

    pub(crate) fn listed(&mut self, root: &Arc<str>, directory: &Arc<str>, path: &str) -> Result<()> {
        let ids = self.source_ids.entry(directory.clone()).or_default();
        let id = Self::source_in(&mut self.sources, ids, directory, path)?;
        self.alias_source(root, path, id)
    }

    fn alias_source(&mut self, root: &Arc<str>, path: &str, id: usize) -> Result<()> {
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

    pub(crate) fn project_inputs(&mut self, root: &str, paths: &[String]) -> Result<Vec<usize>> {
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
    pub(crate) fn admit_paths(
        &mut self,
        root: &Arc<str>,
        files: &[String],
        binding: usize,
        known_binding: bool,
        mut resolved: impl FnMut(usize),
    ) -> Result<Vec<usize>> {
        let ids = self.source_ids.entry(root.clone()).or_default();
        files.iter().map(|path| {
            let src = Self::source_in(&mut self.sources, ids, root, path)?;
            resolved(src);
            Ok(Self::slot(&mut self.sources[src], src, binding, known_binding, &mut self.pairs, &mut self.pending))
        }).collect()
    }

    pub(crate) fn admit_ids(&mut self, sources: &[usize], binding: usize, known_binding: bool) -> Vec<usize> {
        sources.iter().map(|&src| {
            Self::slot(&mut self.sources[src], src, binding, known_binding, &mut self.pairs, &mut self.pending)
        }).collect()
    }

    fn slot(
        source: &mut Source,
        src: usize,
        binding: usize,
        known_binding: bool,
        pairs: &mut usize,
        pending: &mut BTreeSet<usize>,
    ) -> usize {
        if known_binding && let Some(&slot) = source.known.get(&binding) {
            return slot;
        }
        let first = source.bindings.is_empty();
        *source.bindings.entry(binding).or_insert_with(|| {
            let slot = *pairs;
            *pairs += 1;
            if first {
                pending.insert(src);
            }
            slot
        })
    }
}

/// Admission assigns dense binding IDs. Only bindings used by this reader own an executor.
struct Bindings<'p> { index: Vec<usize>, entries: Vec<Result<Bound<'p>>> }

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

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
struct SummaryColumns {
    kinds: bool,
    fields: bool,
    words: bool,
}

type FactMask = [u64; relations::service::ALL.len().div_ceil(64)];

fn requested_facts(mask: FactMask, out: &mut Vec<l::Relation>) {
    out.clear();
    for (group, mut bits) in mask.into_iter().enumerate() {
        while bits != 0 {
            let id = group * 64 + bits.trailing_zeros() as usize;
            out.push(relations::service::ALL[id]);
            bits &= bits - 1;
        }
    }
}

struct Bound<'p> {
    fact_mask: FactMask,
    requirements: Rc<BoundRequirements>,
    projects_words: bool,
    reads_fields: bool,
    screenable: Option<bool>,
    empty_frontier: Option<frontier::EmptyFrontier>,
    plan: &'p Plan,
    slots: Vec<usize>,
    test: usize,
    absent: Option<(u32, Vec<bool>)>,
    outputs: HashMap<usize,usize>,
    projections: BTreeMap<usize,finish::session::contribution::Projection>,
    facts: Vec<l::Relation>,
    filters: BTreeMap<usize,Vec<(usize,u64)>>,
    hosted: Tables,

    lines: bool,
    coverage: reader_run::coverage::Coverage,
}

#[derive(Eq, Hash, PartialEq)]
struct BoundRequirements {
    never: bool,
    shape: Vec<Vec<(bool, u32)>>,
    words: Vec<Vec<u32>>,
    summary: SummaryColumns,
}

impl BoundRequirements {
    fn new(program: &Program, run: &mut Run) -> Self {
        let (never, shape, words) = &program.requirements;
        let summary = SummaryColumns {
            kinds: shape.iter().flatten().any(|v| v.starts_with("k:")),
            fields: shape.iter().flatten().any(|v| v.starts_with("f:")),
            words: !words.is_empty(),
        };
        let shape = shape.iter().map(|clause| {
            clause.iter().map(|value| {
                let (kind, term) = value.split_once(':').expect("shape term");
                (kind == "k", run.sym(term.as_bytes()))
            }).collect()
        }).collect();
        let words = words.iter().map(|clause| {
            clause.iter().map(|word| run.sym(word.as_bytes())).collect()
        }).collect();
        Self { never: *never, shape, words, summary }
    }
}

fn bind<'p, L: FileSource>(
    registry: &mut finish::session::contribution::Registry,
    program: &'p Program,
    parser: Option<&L>,
    run: &mut Run,
    hosted: Tables,
    cut: Option<&finish::partition::Partition>,
    (requirements, test): (Rc<BoundRequirements>, usize),
) -> Result<Bound<'p>> {
    let mut facts = Vec::new();
    let mut projects_words = requirements.summary.words;
    let mut reads_fields = requirements.summary.fields;
    for (name, (at, relation)) in program.bases.iter().zip(&program.base_steps) {
        if cut.is_some_and(|c| !c.file_steps[*at]) || hosted.iter().any(|(n, _)| n == name) {
            continue;
        }
        let rel = relation.ok_or_else(|| format!("undeclared {name}"))?;
        let name = name.as_str();
        projects_words |= matches!(rel, l::Relation::SyntaxTree | l::Relation::SemanticSyntax | l::Relation::TokenLink);
        reads_fields |= matches!(rel, l::Relation::SemanticChild | l::Relation::SyntaxChild);
        if relations::service::class_of(rel) == relations::types::Class::Engine {
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
        if parser.is_some_and(|p| p.expands(rel)) {
            run.reader_tables.insert(name.to_owned());
        }
        if catalog::service::fact_form(rel) != relations::types::FactForm::NodeTable
            || run.reader_tables.contains(name)
        {
            if parser.is_some_and(|p| !p.available(rel)) {
                return Err(format!("provider does not read {name}"));
            }
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
        fact_mask,
        requirements,
        reads_fields,
        projects_words,
        screenable: None,
        absent: None,
        empty_frontier: None,
        outputs,
        projections,
        filters,
        plan: &program.plan,
        slots: Vec::with_capacity(program.plan.nodes.len()),
        test,
        facts,
        hosted,
        lines: program
            .bases
            .iter()
            .any(|n| n == relations::service::name(l::Relation::FilesystemLineCount).0),
        coverage: Default::default(),
    })
}

struct Version<'s> {
    origin: usize,
    project_slots: Vec<usize>,
    data: Arc<[u8]>,
    jobs: std::borrow::Cow<'s, BTreeMap<usize, usize>>,
}

/// A relative path in two overlaid roots has the same local answers exactly when
/// the parser sees the same bytes and the host sees the same executable mode.
/// Hard-linked listed files in the immutable snapshot need only one physical read.
fn versions<'s>(
    group: &[usize],
    sources: &'s [Source],
    answers: &mut (impl Slots<Answer> + ?Sized),
    aliases: &mut (impl Slots<usize> + ?Sized),
    reads: &mut usize,
    retained: &mut HashMap<usize, (Arc<[u8]>, u32)>,
    keep: bool,
) -> Vec<Version<'s>> {
    use std::os::unix::fs::MetadataExt;
    if let [idx] = *group && !keep {
        let src = &sources[idx];
        let data = if let Some(bytes) = &src.provided {
            Ok(Arc::clone(bytes))
        } else if let Some((bytes, _)) = retained.get(&idx) {
            Ok(Arc::clone(bytes))
        } else {
            *reads += 1;
            std::fs::read(Path::new(&*src.root).join(&src.path)).map(Arc::from)
        };
        return match data {
            Ok(data) => vec![Version {
                origin: idx,
                project_slots: src.project.then_some(idx).into_iter().collect(),
                data,
                jobs: std::borrow::Cow::Borrowed(&src.bindings),
            }],
            Err(error) => {
                for &slot in src.bindings.values() {
                    answers.set(slot, Err(format!("{}: {error}", src.path)));
                }
                Vec::new()
            }
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
            } else if let Some((bytes, mode)) = retained.get(&idx) {
                (Arc::clone(bytes), None, Some(*mode))
            } else {
                let meta = std::fs::metadata(&path)?;
                let inode = (meta.dev(), meta.ino());
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
            if let Some(mode) = perm.filter(|_| keep) {
                retained
                    .entry(idx)
                    .or_insert_with(|| (Arc::clone(&data), mode));
            }
            // Length and executable mode usually decide before the byte comparison.
            // The ordered key also handles different files of equal length exactly.
            let key = (data.len(), perm, Arc::clone(&data));
            let i = *contents.entry(key).or_insert_with(|| {
                let i = files.len();
                files.push(Version {
                    origin: idx,
                    project_slots: Vec::new(),
                    data,
                    jobs: std::borrow::Cow::Owned(BTreeMap::new()),
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
                for (&binding, &slot) in &src.bindings {
                    let origin = *files[i].jobs.to_mut().entry(binding).or_insert(slot);
                    if origin != slot {
                        aliases.set(slot, origin);
                    }
                }
            }
            Err(e) => {
                for &slot in src.bindings.values() {
                    answers.set(slot, Err(format!("{}: {e}", src.path)));
                }
            }
        }
    }
    files
}

/// The host may add requests after consuming the previous answers. Source and symbol
/// identities then live across admissions. A closed batch can recycle one parser scratch.
#[derive(Default)]
pub(crate) struct ReaderState {
    run: Run,
    screens: pairs::Retained,
    patterns: crate::pair_checks::SymbolMatches,
    requirements: HashMap<usize, (Rc<BoundRequirements>, usize)>,
    requirement_ids: HashMap<Rc<BoundRequirements>, usize>,
    previous_plan: Option<shared::PreviousPlan>,
    unknown: Option<HashMap<u32, u32>>,
    bytes: HashMap<usize, (Arc<[u8]>, u32)>,
}

#[derive(Default)]
struct Contents {
    ready: SummaryColumns,
    answers: crate::pair_checks::AtomAnswers,
    kinds: rustc_hash::FxHashSet<u32>,
    fields: rustc_hash::FxHashSet<u32>,
    words: rustc_hash::FxHashSet<u32>,
}

#[derive(Default)]
struct HeldFile<F: Default, D: Default> {
    symbols: D,
    native: F,
    lent: Option<std::sync::Arc<l::NodeTable>>,
    tree: l::NodeTable,
    fields: Vec<u32>,
    text: Vec<u32>,
    recovery: Option<ryo_execution_domain::domain::coverage::types::Recovery>,
    contents: Contents,
    literals: std::cell::RefCell<crate::pair_checks::LiteralPositions>,
    proof_atoms: crate::pair_checks::AtomAnswers,
    known: BTreeSet<l::Relation>,
    facts: l::contract::Collect,
}

/// Each binding gets its own prepared column positions for this reader and host
/// schema. Symbols and file IDs live in the reader session. Source trees never
/// escape the visit. Every request for the same pair refers to its answer slot.
#[path = "check_session/worker.rs"]
pub(crate) mod worker;
use worker::Slots;

pub(crate) fn visit<L: FileSource>(
    state: &mut ReaderState,
    retention: &mut Retention<L::File, L::Dictionary>,
    keep: bool,
    parser: Option<&L>,
    reader_id: Option<usize>,
    manifests: &BTreeSet<&str>,
    contributions: &mut (impl Slots<Option<Rc<project::Contribution>>> + ?Sized),
    programs: &[Result<Program>],
    program_of: &[usize],
    contracts: &[Arc<completeness::Contract>],
    cut_of: &[Option<Arc<finish::partition::Partition>>],
    world: &mut World,
    sources: &[Source],
    selected: &[usize],
    aliases: &mut (impl Slots<usize> + ?Sized),
    answers: &mut (impl Slots<Answer> + ?Sized),
    mut host: impl FnMut(usize, &mut Run) -> Result<Tables>,
    clocks: &mut Clock,
) {
    let before = (clocks.read_parse, clocks.read_versions, clocks.build_file, clocks.files);
    let mut tree_cpu = 0.;
    let mut fact_cpu = 0.;
    let mut run = std::mem::take(&mut state.run);
    let unknown = state.unknown.get_or_insert_with(|| {
        run.file_field_codes = true;
        run.empty = run.sym(b"");
        run.links();
        let public =
            reader::public_words(&mut run, &parser.map_or_else(Vec::new, FileSource::words));
        if let Some(p) = &parser {
            run.kind_sym = p.kinds().iter().map(|k| run.sym(k.as_bytes())).collect();
        }
        public
    });
    if run.paths.len() < sources.len() {
        let start = run.paths.len();
        Arc::make_mut(&mut run.paths).extend(sources[start..].iter().map(|src| src.path.clone()));
    }
    let t = thread_cpu();
    let mut needed = vec![false; program_of.len()];
    for &index in selected {
        for &binding in sources[index].bindings.keys() {
            needed[binding] = true;
        }
    }
    let mut checks = (!keep).then(|| pairs::Checks::new(parser, programs, program_of, &needed, cut_of));
    let mut local_requirements = HashMap::new();
    let mut local_ids = HashMap::new();
    let (reader_requirements, requirement_ids) = if keep {
        (&mut state.requirements, &mut state.requirement_ids)
    } else {
        (&mut local_requirements, &mut local_ids)
    };
    let local_patterns = crate::pair_checks::SymbolMatches::default();
    let pattern_cache = if keep { &state.patterns } else { &local_patterns };
    clocks.prepare += thread_cpu() - t;
    let mut bound = Bindings { index: vec![usize::MAX; program_of.len()], entries: Vec::new() };
    let mut registry = finish::session::contribution::Registry::default();
    let mut requested = Vec::new();
    let mut facts = Vec::new();
    let mut distinct = std::cell::RefCell::new(crate::pair_checks::FileSymbols::default());
    let mut scratch = HeldFile::default();
    let mut packet = reader_run::packet::Packet::new();
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
                    clocks.requirement_programs += 1;
                    let requirements = Rc::new(BoundRequirements::new(program, &mut run));
                    if let Some((known, &id)) = requirement_ids.get_key_value(&requirements) {
                        return (known.clone(), id);
                    }
                    let id = requirement_ids.len();
                    requirement_ids.insert(requirements.clone(), id);
                    (requirements, id)
                })
                .clone();
            let mut b = bind(&mut registry, program, parser, &mut run, tables,
                cut_of[binding].as_deref(), requirements)?;
            merged.add(&mut b, &mut run, program_id, keep);
            Ok(b)
        })();
        bound.index[binding] = bound.entries.len();
        bound.entries.push(entry);
    }
    let (plan, supplied) = merged.finish(keep.then_some(&mut state.previous_plan), &mut bound);
    let mut exec = Exec::new(&plan);
    let mut read_roots = Vec::new();
    clocks.prepare += thread_cpu() - start;
    let mut carried_symbols = HashMap::new();
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
            keep,
        );
        let elapsed = thread_cpu() - t;
        clocks.read_versions += elapsed;
        clocks.read_parse += elapsed;
        for Version {
            origin: index,
            project_slots,
            data: bytes,
            jobs,
        } in versions
        {
            clocks.unique_pairs += jobs.len();
            let source = &sources[index];
            let id = FileId(index.try_into().expect("file ID"));
            let t = thread_cpu();
            let mut required = FactMask::default();
            let mut have_plan = false;
            let mut projects_words = keep;
            let mut projects_fields = keep;
            let mut summary = SummaryColumns::default();
            for (&binding, &slot) in jobs.iter() {
                match &bound[&binding] {
                    Ok(b) => {
                        have_plan = true;
                        for (word, bits) in required.iter_mut().zip(b.fact_mask) { *word |= bits; }
                        projects_words |= b.projects_words;
                        projects_fields |= b.reads_fields;
                        if cut_of[binding].is_none() && !b.requirements.never {
                            summary.kinds |= b.requirements.summary.kinds;
                            summary.fields |= b.requirements.summary.fields;
                            summary.words |= b.requirements.summary.words;
                        }
                    }
                    Err(reason) => answers.set(slot, Err(reason.clone())),
                }
            }
            clocks.prepare += thread_cpu() - t;
            requested_facts(required, &mut requested);
            if project_slots.is_empty() && !have_plan {
                continue;
            }
            let t = thread_cpu();
            run.root_dir.clear();
            run.root_dir.push_str(&source.root);
            let session_open = keep;
            let start = keep.then(|| retention.clock()).flatten();
            let mut cached = start.and_then(|_| retention.take(index));
            // An opaque provider dictionary and its projected rows are one version.
            // Rebuild them together if a later binding asks for additional facts.
            if let Some((credit, file)) = cached.as_mut()
                && !requested.iter().all(|rel| file.known.contains(rel))
            {
                *file = HeldFile::default();
                *credit = 0;
            }
            let keep = cached.is_some();
            let held = cached.as_mut().map_or(&mut scratch, |(_,file)| file);
            let mut symbols = &mut held.symbols;
            let reused = keep && held.recovery.is_some();
            let mut elements = &mut held.native;
            let mut tree = &mut held.tree;
            let result = if reused {
                Ok(())
            } else if let Some(parser) = &parser {
                clocks.files += 1;
                let request = l::ParseRequest {
                    file: id,
                    path: SourcePath(&source.path),
                    bytes: ByteSlice(&bytes),
                    // Later calls can widen the relations asked of a retained file.
                    fields: if session_open || projects_fields {
                        l::FieldRequest::Wanted
                    } else {
                        l::FieldRequest::Unwanted
                    },
                    notes: if project_slots.is_empty() && !session_open {
                        l::NoteRequest::Unwanted
                    } else {
                        l::NoteRequest::Wanted
                    },
                };

                if !session_open && projects_words { requested.push(l::Relation::SyntaxTree); }
                let build_start = thread_cpu();
                let result = parser.build(
                    request,
                    &mut elements,
                    &mut symbols,
                    if session_open {
                        relations::service::ALL
                    } else {
                        &requested
                    },
                );
                clocks.build_file += thread_cpu() - build_start;
                result
            } else {
                // Every registered reader declined this path. Its host populations
                // still exist and no source reader contributes syntax or semantic rows.
                Ok(())
            };
            if let Err(reason) = result {
                for &slot in jobs.values() {
                    answers.set(slot, Err(reason.clone()));
                }
                clocks.read_parse += thread_cpu() - t;
                continue;
            }
            if !project_slots.is_empty() || (session_open && !reused) {
                let c = contributions.previous(index).and_then(Option::as_ref).cloned().unwrap_or_else(|| {
                    let manifest = manifests
                        .contains(source.path.rsplit('/').next().unwrap())
                        .then(|| l::ManifestText(String::from_utf8_lossy(&bytes).into_owned().into()));
                    Rc::new(project::Contribution { reader: reader_id, notes: L::notes(&elements), manifest })
                });
                if session_open {
                    contributions.set(index, Some(c.clone()));
                }
                for slot in project_slots {
                    contributions.set(slot, Some(c.clone()));
                }
                if !session_open && jobs.keys().all(|b| bound[b].is_err()) {
                    clocks.read_parse += thread_cpu() - t;
                    continue;
                }
            }
            let mut provider = reader::ReaderFacts::new(id.0, &unknown);
            provider.projects_words = projects_words;
            provider.projects_fields = projects_fields;
            if reused {
                run.field_sym.clone_from(&held.fields);
                provider.text = std::mem::take(&mut held.text);
            } else {
                let tree_start = thread_cpu();
                L::tree(
                    &elements,
                    &symbols,
                    &bytes,
                    &mut run,
                    &mut tree,
                    &mut provider,
                );
                tree_cpu += thread_cpu() - tree_start;
                if keep {
                    held.fields.clone_from(&run.field_sym);
                }
            }
            clocks.read_parse += thread_cpu() - t;
            let t = thread_cpu();
            let native = if reused { held.lent.take() } else { L::lent(elements) };
            let tree = native.as_deref().unwrap_or(tree);
            let contents = &mut held.contents;
            if !reused {
                contents.ready = SummaryColumns::default();
                contents.answers.clear();
            }
            summary.kinds &= !contents.ready.kinds;
            summary.fields &= !contents.ready.fields;
            summary.words &= !contents.ready.words;
            if summary.kinds || summary.fields || summary.words {
                contents.ready.kinds |= summary.kinds;
                contents.ready.fields |= summary.fields;
                contents.ready.words |= summary.words;
                if summary.kinds {
                    contents.kinds.clear();
                    contents.kinds.extend(l::node_table::kind_codes(tree).iter()
                        .map(|&k| run.kind_sym[usize::from(k)]));
                }
                if summary.fields {
                    contents.fields.clear();
                    contents.fields.extend(l::node_table::field_codes(tree).iter()
                        .map(|&f| run.field_sym[usize::from(f)]));
                }
                if summary.words {
                    contents.words.clear();
                    contents.words.extend(provider.text.iter().copied());
                    contents.words.insert(run.empty);
                }
                L::expanded_summary(elements, symbols, &bytes, &mut run, unknown,
                    l::node_table::kind_codes(tree).len(), summary.kinds.then_some(&mut contents.kinds),
                    summary.fields.then_some(&mut contents.fields), summary.words.then_some(&mut contents.words));
            }
            let Contents { kinds, fields, words, answers: coarse, .. } = contents;
            use ryo_execution_domain::domain::coverage::service::partial_of;
            let recovery = held.recovery.unwrap_or_else(|| L::recovery(&elements));
            let mut active = Vec::new();
            for (&binding, &slot) in jobs.iter() {
                let Ok(b) = &bound[&binding] else { continue };
                let excluded = cut_of[binding].is_none()
                    && coarse.get(b.test).unwrap_or_else(|| {
                        let impossible = b.requirements.never
                            || !b.requirements.shape.iter().all(|c| {
                                c.iter().any(|(k, v)| if *k { kinds.contains(v) } else { fields.contains(v) })
                            })
                            || !b.requirements.words.iter().all(|c| c.iter().any(|v| words.contains(v)));
                        coarse.set(b.test, impossible);
                        impossible
                    });
                if excluded && b.facts.is_empty() {
                    let whole = recovery == ryo_execution_domain::domain::coverage::types::Recovery::Whole;
                    if whole || {
                        let rels: Vec<_> = contracts[binding].needs.reads.iter()
                            .map(|r| r.relation()).collect();
                        partial_of(&[], recovery, &rels).is_empty()
                    } {
                        answers.set(slot, Ok(Reported::default()));
                        clocks.skipped += 1;
                        continue;
                    }
                }
                active.push((binding, slot, excluded));
            }
            clocks.screen += thread_cpu() - t;
            let t = thread_cpu();
            let mut required = FactMask::default();
            for (binding, _, _) in &active {
                for (word, bits) in required.iter_mut().zip(bound[binding].as_ref().unwrap().fact_mask) { *word |= bits; }
            }
            requested_facts(required, &mut facts);
            packet.clear();
            if keep {
                std::mem::swap(&mut held.facts, &mut packet.rows);
            }
            let missing: Vec<_> = facts
                .iter()
                .filter(|rel| !keep || !held.known.contains(rel))
                .copied()
                .collect();
            if let Some(parser) = &parser
                && !missing.is_empty()
            {
                let fact_start = thread_cpu();
                let filled = parser.fill(
                    &mut elements,
                    &mut symbols,
                    &bytes,
                    id,
                    &missing,
                    &mut packet.rows,
                );
                fact_cpu += thread_cpu() - fact_start;
                if let Err(reason) = filled {
                    for &slot in jobs.values() {
                        answers.set(slot, Err(reason.clone()));
                    }
                    if keep {
                        held.known.clear();
                        held.facts.clear();
                    }
                    continue;
                }
            }
            let partials = std::mem::take(&mut packet.rows.partials);
            let partials_at = std::mem::take(&mut packet.rows.partials_at);
            use ryo_values::domain::stored_facts::types::{
                PartialAt, PartialCause, PartialScope, ReadInPart,
            };
            let said: Vec<_> = partials
                .iter()
                .map(|(relation, why, lacking)| ReadInPart {
                    relation: *relation,
                    cause: PartialCause(why.as_str().into()),
                    lacking: *lacking,
                    at: PartialScope::OneFile(PartialAt(id.0)),
                })
                .chain(partials_at.iter().map(|(relation, file, why)| ReadInPart {
                    relation: *relation,
                    cause: PartialCause(why.as_str().into()),
                    lacking: ryo_relations::domain::relations::types::Missing::Any,
                    at: PartialScope::OneFile(PartialAt(file.0)),
                }))
                .collect();
            if keep {
                held.known.extend(facts.iter().copied());
                packet.index(&held.known.iter().copied().collect::<Vec<_>>());
            } else {
                packet.index(&facts);
            }
            clocks.read_parse += thread_cpu() - t;
            let t = thread_cpu();
            if recovery == ryo_execution_domain::domain::coverage::types::Recovery::Whole
            {
                distinct.get_mut().clear();
                if !reused {
                    held.literals.get_mut().clear();
                    held.proof_atoms.clear();
                }
                let scanned = held.literals.get_mut().scanned();
                let mut evaluate = |scope, screen: &mut pairs::Checks, jobs: &mut [(usize, usize, bool)], root: fn(usize) -> usize| {
                    let patterns = crate::pair_checks::Predicates { cache: pattern_cache, scope };
                    screen.exclude(root, jobs, &mut bound, &mut run, tree, provider.projects_words.then_some(provider.text.as_slice()), &distinct, &held.literals, patterns, &mut held.proof_atoms,
                        &bytes, &source.path, id, L::symbols(&symbols), &facts, &packet, unknown, cut_of, &said);
                };
                if let Some(screen) = &mut checks {
                    evaluate(0, screen, &mut active, |binding| binding);
                } else if selected.len() > 1 {
                    // A one-source open admission cannot amortize preparing a reader screen.
                    // Its coarse requirements and ordinary executor still apply.
                    let prepare = state.screens.groups(parser, programs, program_of, cut_of, &mut active,
                        |index, screen, jobs| evaluate(index, screen, jobs, |_| 0));
                    clocks.prepare += prepare;
                    clocks.bound_proofs -= prepare;
                    clocks.screen -= prepare;
                }
                clocks.literal_positions += held.literals.get_mut().scanned() - scanned;
            }
            let proof_time = thread_cpu() - t;
            clocks.screen += proof_time;
            clocks.bound_proofs += proof_time;
            let mut facts_view = packet.view(&provider, L::symbols(&symbols), &bytes);
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
            let indexed = facts_view.indexed();
            let mut version = finish::FileVersion::new(&tree, &bytes, id.0, facts_view.syntax_text());
            let mut frame = None;
            let mut shared = frontier::FileInputs::new(&active,&bound,programs,program_of,cut_of);
            version.bind_packet(Some(&indexed));
            version.supply(supplied.iter().chain(lines.as_ref()).cloned());
            read_roots.clear();
            let incomplete = !said.is_empty() || recovery != ryo_execution_domain::domain::coverage::types::Recovery::Whole;
            for &(binding, _, excluded) in &active {
                let b = bound[&binding].as_ref().unwrap();
                if incomplete {
                    read_roots.extend(&b.slots);
                } else if let Some(cut) = &cut_of[binding] {
                    read_roots.extend(cut.inputs.iter().map(|&node| b.slots[node]));
                } else if !excluded {
                    read_roots.extend(b.plan.emits.iter().map(|emit| b.slots[emit.1]));
                }
            }
            if let Err(reason) = exec.open_file(&version, &mut run, &mut facts_view, read_roots.iter().copied(), &[], None) {
                for (_, slot, _) in active { answers.set(slot, Err(reason.clone())); }
                continue;
            }
            for (binding, slot, excluded) in active {
                use ryo_execution_domain::domain::trust::{service::matters, types::Bearing};
                let contract = &contracts[binding];
                let b = bound[&binding].as_mut().unwrap();
                let mut local_coverage = BTreeMap::new();
                let unknowns = if said.is_empty()
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
                    contract.report(&partial, &source.path)
                };
                if excluded && unknowns.is_empty() {
                    if let Some(cut) = &cut_of[binding] {
                        world.insert_empty(slot, &source.path, b, &mut run, cut);
                    }
                    answers.set(slot, Ok(Reported::default()));
                    clocks.skipped += 1;
                    continue;
                }
                clocks.evaluated += 1;
                let make_frame = || finish::session::frame(&version, &plan);
                let mut restricted;
                let current = if local_coverage.is_empty() {
                    frame.get_or_insert_with(make_frame)
                } else {
                    restricted = make_frame();
                    &mut restricted
                };
                if let Some(cut) = &cut_of[binding] {
                    let value = shared.contribute(b, &mut exec, current, &mut run, id.0, &mut facts_view, cut,
                        &local_coverage, world, &mut carried_symbols, reader_id, unknown, slot);
                    if value.is_err() { frame = None; exec.release_request(); }
                    let answer = value.map(|()| {
                        Reported {
                            rows: Vec::new(),
                            unknowns,
                        }
                    });
                    clocks.productive += usize::from(world.productive(slot));
                    clocks.refused += usize::from(answer.is_err());
                    answers.set(slot, answer);
                    continue;
                }
                let value = finish::session::pair(&mut exec, current, &mut run, &mut facts_view, b.plan, binding, &b.slots, &local_coverage);
                if value.is_err() { frame = None; exec.release_request(); }
                let answer = value
                .map(|rows| Reported { rows, unknowns });
                clocks.productive += usize::from(answer.as_ref().is_ok_and(|r| !r.rows.is_empty()));
                clocks.refused += usize::from(answer.is_err());
                    answers.set(slot, answer);
            }
            drop(frame);
            let _widened = exec.close_file(&version);
            drop(version);
            if keep {
                held.recovery = Some(recovery);
                held.lent = native;
                held.native = L::File::default();
                held.text = provider.text;
                packet.rows.partials = partials;
                packet.rows.partials_at = partials_at;
                std::mem::swap(&mut held.facts, &mut packet.rows);
            }
            if let Some((previous, file)) = cached
                && let (Some(start), Some(end)) = (start, retention.clock()) {
                // All allocations on this worker during the visit conservatively bound
                // the new storage, including provider-owned opaque parser allocations.
                retention.put(index, previous.saturating_add(end.wrapping_sub(start)), file);
            }
            clocks.evaluate += thread_cpu() - t;
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
    parts.tree += tree_cpu;
    parts.facts += fact_cpu;
    if std::env::var_os("VEC_STEP_TIME").is_some() { eprintln!("SHARED_STEPS runs={} rows={}",exec.stats.total_runs,exec.stats.total_rows); }
    if keep {
        state.run = run;
    }
}
