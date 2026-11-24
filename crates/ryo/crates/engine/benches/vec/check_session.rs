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
#[path = "check_session/shared.rs"]
mod shared;

type Parameters =
    BTreeMap<Box<str>, ryo_values::domain::finish_plans::expressions::types::PlanConstant>;

type Result<T> = std::result::Result<T, String>;
pub(crate) type Answer = Result<Reported>;

#[derive(Default)]
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
    typed: Rc<ryo_values::domain::finish_plans::types::FinishPlan>,
    pub plan: Plan,
    pub syntax: Box<ryo_values::domain::program::types::WrittenProgram>,
    pub bases: Vec<String>,
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
        let bases = plan
            .nodes
            .iter()
            .filter_map(|n| n.base().map(str::to_owned))
            .collect();
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
        })
    }
}

#[derive(Default)]
pub(crate) struct Clock {
    pub prepare: f64,
    pub read_parse: f64,
    pub screen: f64,
    pub evaluate: f64,
    pub project: f64,
    pub assemble: f64,
    pub files: usize,
    pub reads: usize,
    pub unique_pairs: usize,
    pub pairs: usize,
    pub skipped: usize,
    pub evaluated: usize,
    pub productive: usize,
    pub refused: usize,
}

/// The root and path identify the caller's immutable effective source. This
/// distinguishes in-memory replacements staged at different roots.
pub(crate) struct Source {
    pub root: Rc<str>,
    pub path: String,
    pub bindings: BTreeMap<usize, usize>,
    known: BTreeMap<usize, usize>,
    project: bool,
    provided: Option<Rc<[u8]>>,
}

/// Per-check program identities and the inverse admission index. The caller binds
/// external relations and hands the index a stable identity for each binding.
#[derive(Default)]
pub(crate) struct Schedule {
    pub sources: Vec<Source>,
    source_ids: HashMap<(Rc<str>, Rc<str>), usize>,
    supplied_ids: HashMap<(Rc<str>, Rc<[u8]>), usize>,
    pub pending: BTreeSet<usize>,
    pub programs: Vec<Result<Program>>,
    prepared: HashMap<(Rc<str>, Parameters), usize>,
    pub pairs: usize,
}

impl Schedule {
    pub(crate) fn delivered(&mut self, ids: &[usize]) {
        for &i in ids {
            self.sources[i].bindings.clear();
            self.sources[i].project = false;
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
    fn source(&mut self, root: &Rc<str>, path: String) -> Result<usize> {
        let rel = Path::new(&path);
        if rel
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(format!("invalid relative source path {path}"));
        }
        Ok(*self
            .source_ids
            .entry((root.clone(), Rc::from(path.as_str())))
            .or_insert_with(|| {
                self.sources.push(Source {
                    root: root.clone(),
                    path,
                    bindings: BTreeMap::new(),
                    known: BTreeMap::new(),
                    project: false,
                    provided: None,
                });
                self.sources.len() - 1
            }))
    }

    /// Attach immutable caller bytes before admitting any consumer of this source root.
    pub(crate) fn provide(&mut self, root: &Rc<str>, path: String, bytes: Rc<[u8]>) -> Result<()> {
        let key = (Rc::from(path.as_str()), bytes.clone());
        let id = match self.supplied_ids.get(&key) {
            Some(&id) => id,
            None => {
                let id = self.source(root, path.clone())?;
                self.sources[id].provided = Some(bytes);
                self.supplied_ids.insert(key, id);
                id
            }
        };
        self.alias_source(root, &path, id)
    }

    pub(crate) fn listed(&mut self, root: &Rc<str>, directory: &Rc<str>, path: &str) -> Result<()> {
        let id = self.source(directory, path.to_owned())?;
        self.alias_source(root, path, id)
    }

    fn alias_source(&mut self, root: &Rc<str>, path: &str, id: usize) -> Result<()> {
        if self
            .source_ids
            .insert((root.clone(), Rc::from(path)), id)
            .is_some_and(|p| p != id)
        {
            return Err("immutable source version changed".into());
        }
        Ok(())
    }

    pub(crate) fn project_inputs(&mut self, root: &str, paths: &[String]) -> Result<Vec<usize>> {
        let root: Rc<str> = Rc::from(root);
        paths
            .iter()
            .map(|p| {
                let index = self.source(&root, p.clone())?;
                self.sources[index].project = true;
                self.pending.insert(index);
                Ok(index)
            })
            .collect()
    }
    pub(crate) fn admit(
        &mut self,
        root: String,
        files: Vec<String>,
        binding: usize,
    ) -> Result<Vec<usize>> {
        let root: Rc<str> = root.into();
        let mut pairs = Vec::with_capacity(files.len());
        for path in files {
            let src = self.source(&root, path)?;
            let source = &mut self.sources[src];
            let slot = *source.known.entry(binding).or_insert_with(|| {
                let slot = self.pairs;
                self.pairs += 1;
                source.bindings.insert(binding, slot);
                self.pending.insert(src);
                slot
            });
            pairs.push(slot);
        }
        Ok(pairs)
    }
}

/// Binding IDs are dense across the session. Only the selected reader's
/// bindings own a compiled plan entry.
struct Bindings<'p> {
    index: Vec<usize>,
    entries: Vec<Result<Bound<'p>>>,
}
impl<'p> std::ops::Index<usize> for Bindings<'p> {
    type Output = Result<Bound<'p>>;
    fn index(&self, id: usize) -> &Self::Output {
        &self.entries[self.index[id]]
    }
}
impl std::ops::IndexMut<usize> for Bindings<'_> {
    fn index_mut(&mut self, id: usize) -> &mut Self::Output {
        &mut self.entries[self.index[id]]
    }
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
    screenable: Option<bool>,
    empty_frontier: Option<frontier::EmptyFrontier>,
    plan: &'p Plan,
    slots: Vec<usize>,
    test: usize,
    facts: Vec<l::Relation>,
    fact_mask: FactMask,
    filters: BTreeMap<usize,Vec<(usize,u64)>>,
    hosted: Tables,
    shape: Vec<Vec<(bool, u32)>>,
    words: Vec<Vec<u32>>,
    never: bool,
    lines: bool,
    coverage: reader_run::coverage::Coverage,
}

fn bind<'p, L: FileSource>(
    program: &'p Program,
    parser: Option<&L>,
    run: &mut Run,
    hosted: Tables,
    cut: Option<&finish::partition::Partition>,
) -> Result<Bound<'p>> {
    let mut facts = Vec::new();
    for (at, step) in program.plan.nodes.iter().enumerate() {
        if step.op != "base" || cut.is_some_and(|c| !c.file_steps[at]) {
            continue;
        }
        let name = step.base().ok_or("base relation")?;
        if hosted.iter().any(|(n, _)| n == name) {
            continue;
        }
        let rel = relations::service::relation_named(NameRef(name))
            .ok_or_else(|| format!("undeclared {name}"))?;
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
    let (never, shape, words) = &program.requirements;
    let shape = shape
        .iter()
        .map(|clause| {
            clause
                .iter()
                .map(|v| {
                    let (kind, term) = v.split_once(':').unwrap();
                    (kind == "k", run.sym(term.as_bytes()))
                })
                .collect()
        })
        .collect();
    let words = words
        .iter()
        .map(|clause| clause.iter().map(|w| run.sym(w.as_bytes())).collect())
        .collect();
    let filters=cut.into_iter().flat_map(|c|&c.filters).map(|(&node,alts)|{
        (node,alts.iter().map(|(col,w)|(*col,u64::from(run.sym(w)))).collect())
    }).collect();
    let mut fact_mask = FactMask::default();
    for &rel in &facts {
        let id = relations::service::number_of(rel).0;
        fact_mask[id / 64] |= 1 << (id % 64);
    }
    Ok(Bound {
        fact_mask,
        screenable: None,
        empty_frontier: None,
        filters,
        plan: &program.plan,
        slots: Vec::with_capacity(program.plan.nodes.len()),
        test: 0,
        facts,
        hosted,
        shape,
        words,
        never: *never,
        lines: program
            .bases
            .iter()
            .any(|n| n == relations::service::name(l::Relation::FilesystemLineCount).0),
        coverage: Default::default(),
    })
}

struct Version {
    origin: usize,
    project_slots: Vec<usize>,
    data: Rc<[u8]>,
    jobs: BTreeMap<usize, usize>,
}

/// A relative path in two overlaid roots has the same local answers exactly when
/// the parser sees the same bytes and the host sees the same executable mode.
/// Hard-linked listed files in the immutable snapshot need only one physical read.
fn versions(
    group: &[usize],
    sources: &[Source],
    answers: &mut [Answer],
    aliases: &mut [usize],
    reads: &mut usize,
    retained: &mut HashMap<usize, (Rc<[u8]>, u32)>,
    keep: bool,
) -> Vec<Version> {
    use std::hash::{Hash, Hasher};
    use std::os::unix::fs::MetadataExt;
    let mut files = Vec::<Version>::new();
    let mut inodes = HashMap::new();
    let mut hashes = HashMap::<(u64, Option<u32>), Vec<usize>>::new();
    for &idx in group {
        let src = &sources[idx];
        let path = Path::new(&*src.root).join(&src.path);
        let found = (|| -> std::io::Result<usize> {
            let (data, inode, perm) = if let Some(bytes) = src.provided.as_ref() {
                (Rc::clone(bytes), None, None)
            } else if let Some((bytes, mode)) = retained.get(&idx) {
                (Rc::clone(bytes), None, Some(*mode))
            } else {
                let meta = std::fs::metadata(&path)?;
                let inode = (meta.dev(), meta.ino());
                if let Some(&i) = inodes.get(&inode) {
                    return Ok(i);
                }
                *reads += 1;
                (
                    Rc::from(std::fs::read(path)?),
                    Some(inode),
                    Some(meta.mode() & 0o111),
                )
            };
            if let Some(mode) = perm.filter(|_| keep) {
                retained
                    .entry(idx)
                    .or_insert_with(|| (Rc::clone(&data), mode));
            }
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            data.hash(&mut hasher);
            let equal = hashes.entry((hasher.finish(), perm)).or_default();
            let i = match equal.iter().copied().find(|&i| files[i].data == data) {
                Some(i) => i,
                None => {
                    let i = files.len();
                    files.push(Version {
                        origin: idx,
                        project_slots: Vec::new(),
                        data,
                        jobs: BTreeMap::new(),
                    });
                    equal.push(i);
                    i
                }
            };
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
                    aliases[slot] = *files[i].jobs.entry(binding).or_insert(slot);
                }
            }
            Err(e) => {
                for &slot in src.bindings.values() {
                    answers[slot] = Err(format!("{}: {e}", src.path));
                }
            }
        }
    }
    files
}

/// The host may add requests after consuming the previous answers. Source and symbol
/// identities then live across admissions. A closed batch can recycle one parser scratch.
#[derive(Default)]
pub(crate) struct ReaderState<F: Default, D: Default> {
    run: Run,
    screens: pairs::Retained,
    unknown: Option<HashMap<u32, u32>>,
    files: HashMap<usize, HeldFile<F, D>>,
    bytes: HashMap<usize, (Rc<[u8]>, u32)>,
}

#[derive(Default)]
struct Contents {
    kinds: rustc_hash::FxHashSet<u32>,
    fields: rustc_hash::FxHashSet<u32>,
    words: rustc_hash::FxHashSet<u32>,
}

#[derive(Default)]
struct HeldFile<F: Default, D: Default> {
    symbols: D,
    native: F,
    tree: l::NodeTable,
    fields: Vec<u32>,
    text: Vec<u32>,
    read: bool,
    contents: Contents,
    known: BTreeSet<l::Relation>,
    facts: l::contract::Collect,
}

/// Reader visits in a closed epoch use the same ordered source names. Path symbols
/// still belong to each reader and are interned in first-occurrence order.
#[derive(Default)]
pub(crate) struct SourceNames {
    paths: Vec<String>,
    spelling: Vec<usize>,
    first: Vec<usize>,
}

impl SourceNames {
    fn attach(&mut self, run: &mut Run, sources: &[Source]) {
        if self.paths.is_empty() {
            let mut indices = HashMap::new();
            for (i, source) in sources.iter().enumerate() {
                let next = self.first.len();
                let id = *indices.entry(source.path.as_str()).or_insert_with(|| {
                    self.first.push(i);
                    next
                });
                self.spelling.push(id);
                self.paths.push(source.path.clone());
            }
        }
        std::mem::swap(&mut self.paths, &mut run.paths);
        let words: Vec<_> = self.first.iter().map(|&i| run.sym(sources[i].path.as_bytes())).collect();
        run.path_sym.extend(self.spelling.iter().map(|&i| words[i]));
    }
}

/// Each binding gets its own prepared column positions for this reader and host
/// schema. Symbols and file IDs live in the reader session. Source trees never
/// escape the visit. Every request for the same pair refers to its answer slot.
pub(crate) fn visit<L: FileSource>(
    state: &mut ReaderState<L::File, L::Dictionary>,
    keep: bool,
    names: &mut SourceNames,
    parser: Option<&L>,
    reader_id: Option<usize>,
    manifests: &BTreeSet<&str>,
    contributions: &mut [Option<Rc<project::Contribution>>],
    programs: &[Result<Program>],
    program_of: &[usize],
    contracts: &HashMap<usize, Rc<completeness::Contract>>,
    cut_of: &[Option<Rc<finish::partition::Partition>>],
    world: &mut World,
    sources: &[Source],
    selected: &[usize],
    aliases: &mut [usize],
    answers: &mut [Answer],
    mut host: impl FnMut(usize, &mut Run) -> Result<Tables>,
    clocks: &mut Clock,
) {
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
    if keep {
        for src in &sources[run.paths.len()..] {
            run.paths.push(src.path.clone());
            let word = run.sym(src.path.as_bytes());
            run.path_sym.push(word);
        }
    } else {
        names.attach(&mut run, sources);
    }
    let t = thread_cpu();
    let mut needed = vec![false; program_of.len()];
    for &index in selected {
        for &binding in sources[index].bindings.keys() {
            needed[binding] = true;
        }
    }
    let mut checks = (!keep).then(|| pairs::Checks::new(parser, programs, program_of, &needed, cut_of));
    clocks.prepare += thread_cpu() - t;
    let mut bound = Bindings { index: vec![usize::MAX; needed.len()], entries: Vec::new() };
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
            let program = programs[program_of[binding]].as_ref().map_err(Clone::clone)?;
            let tables = host(binding, &mut run)?;
            let mut b = bind(program, parser, &mut run, tables, cut_of[binding].as_deref())?;
            merged.add(&mut b, &mut run);
            Ok(b)
        })();
        bound.index[binding] = bound.entries.len();
        bound.entries.push(entry);
    }
    let (plan, supplied, test_count) = merged.finish();
    let mut exec = Exec::new(&plan);
    let mut test_results = vec![None; test_count];
    let mut distinct = std::cell::RefCell::new(crate::pair_checks::FileSymbols::default());
    clocks.prepare += thread_cpu() - start;
    let mut carried_symbols = HashMap::new();
    if let Some(dictionary) = reader_id.and_then(|id| world.project_words.get(id)) {
        carried_symbols.extend(unknown.iter().map(|(reason, &word)| (word, dictionary[reason])));
    }
    let mut facts = Vec::new();
    let mut active = Vec::new();
    for group in by_path.values() {
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
        clocks.read_parse += thread_cpu() - t;
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
            for (&binding, &slot) in &jobs {
                if let Err(reason) = &bound[binding] {
                    answers[slot] = Err(reason.clone());
                }
            }
            clocks.prepare += thread_cpu() - t;
            let mut have_plan = false;
            let mut want_contents = keep;
            let mut want_words = keep;
            let mut mask = FactMask::default();
            for &binding in jobs.keys() {
                if let Ok(b) = &bound[binding] {
                    have_plan = true;
                    if cut_of[binding].is_none() && !b.never {
                        want_words |= !b.words.is_empty();
                        want_contents |= want_words || !b.shape.is_empty();
                    }
                    for (word, &required) in mask.iter_mut().zip(&b.fact_mask) {
                        *word |= required;
                    }
                }
            }
            requested_facts(mask, &mut facts);
            if project_slots.is_empty() && !have_plan {
                continue;
            }
            let t = thread_cpu();
            run.root_dir.clear();
            run.root_dir.push_str(&source.root);
            let held = if keep {
                state.files.entry(index).or_default()
            } else {
                &mut scratch
            };
            let mut symbols = &mut held.symbols;
            let reused = keep && held.read;
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
                    fields: l::FieldRequest::Wanted,
                    notes: if project_slots.is_empty() && !keep {
                        l::NoteRequest::Unwanted
                    } else {
                        l::NoteRequest::Wanted
                    },
                };
                parser.build(
                    request,
                    &mut elements,
                    &mut symbols,
                    if keep {
                        relations::service::ALL
                    } else {
                        &facts
                    },
                )
            } else {
                // Every registered reader declined this path. Its host populations
                // still exist and no source reader contributes syntax or semantic rows.
                Ok(())
            };
            if let Err(reason) = result {
                for &slot in jobs.values() {
                    answers[slot] = Err(reason.clone());
                }
                clocks.read_parse += thread_cpu() - t;
                continue;
            }
            if !project_slots.is_empty() || (keep && !reused) {
                let manifest = manifests
                    .contains(source.path.rsplit('/').next().unwrap())
                    .then(|| l::ManifestText(String::from_utf8_lossy(&bytes).into_owned().into()));
                let c = Rc::new(project::Contribution {
                    reader: reader_id,
                    notes: L::notes(&elements),
                    manifest,
                });
                if keep {
                    contributions[index] = Some(c.clone());
                }
                for slot in project_slots {
                    contributions[slot] = Some(c.clone());
                }
                if !keep && !have_plan {
                    clocks.read_parse += thread_cpu() - t;
                    continue;
                }
            }
            let mut provider = reader::ReaderFacts::new(id.0, &unknown);
            if reused {
                run.field_sym.clone_from(&held.fields);
                provider.text = std::mem::take(&mut held.text);
            } else {
                L::tree(
                    &elements,
                    &symbols,
                    &bytes,
                    &mut run,
                    &mut tree,
                    &mut provider,
                );
                if keep {
                    held.fields.clone_from(&run.field_sym);
                }
            }
            clocks.read_parse += thread_cpu() - t;
            let t = thread_cpu();
            let contents = &mut held.contents;
            if !reused && want_contents {
                contents.kinds.clear();
                contents.fields.clear();
                contents.words.clear();
                for (block, word) in l::node_table::kinds_present(tree).0.iter().enumerate() {
                    let mut bits = word.0;
                    while bits != 0 {
                        let code = block * 64 + bits.trailing_zeros() as usize;
                        contents.kinds.insert(run.kind_sym[code]);
                        bits &= bits - 1;
                    }
                }
                for (code, name) in run.field_sym.iter().enumerate() {
                    if !l::node_table::ids_of_field(tree, l::FieldCode(code as u16)).is_empty() {
                        contents.fields.insert(*name);
                    }
                }
                if want_words {
                    contents.words.extend(provider.text.iter().copied());
                    contents.words.insert(run.empty);
                }
                L::expanded_summary(elements, symbols, &bytes, &mut run, unknown,
                    provider.text.len(), &mut contents.kinds, &mut contents.fields, want_words.then_some(&mut contents.words));
            }
            let Contents { kinds, fields, words } = &*contents;
            use ryo_execution_domain::domain::coverage::service::partial_of;
            let recovery = L::recovery(&elements);
            test_results.fill(None);
            active.clear();
            for (&binding, &slot) in &jobs {
                let Ok(b) = &bound[binding] else { continue };
                let excluded = cut_of[binding].is_none()
                    && *test_results[b.test].get_or_insert_with(|| {
                        if b.never || (b.shape.is_empty() && b.words.is_empty()) {
                            return b.never;
                        }
                        !b.shape.iter().all(|clause| clause.iter().any(|(kind, word)| {
                            if *kind { kinds.contains(word) } else { fields.contains(word) }
                        })) || !b.words.iter().all(|clause| clause.iter().any(|v| words.contains(v)))
                    });
                if excluded && b.facts.is_empty() {
                    let whole = recovery == ryo_execution_domain::domain::coverage::types::Recovery::Whole;
                    if whole || {
                        let rels: Vec<_> = contracts[&binding].needs.reads.iter()
                            .map(|r| r.relation()).collect();
                        partial_of(&[], recovery, &rels).is_empty()
                    } {
                        answers[slot] = Ok(Reported::default());
                        clocks.skipped += 1;
                        continue;
                    }
                }
                active.push((binding, slot, excluded));
            }
            clocks.screen += thread_cpu() - t;
            let t = thread_cpu();
            // Coarse screening removes only bindings with no provider facts.
            // The pre-parse union is therefore also the packet's exact request.
            packet.clear();
            if keep {
                std::mem::swap(&mut held.facts, &mut packet.rows);
            }
            let retained_missing;
            let missing = if keep {
                retained_missing = facts.iter().copied()
                    .filter(|rel| !held.known.contains(rel)).collect::<Vec<_>>();
                retained_missing.as_slice()
            } else {
                facts.as_slice()
            };
            if let Some(parser) = &parser
                && !missing.is_empty()
            {
                if let Err(reason) = parser.fill(
                    &mut elements,
                    &mut symbols,
                    &bytes,
                    id,
                    missing,
                    &mut packet.rows,
                ) {
                    for &slot in jobs.values() {
                        answers[slot] = Err(reason.clone());
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
            let version = std::cell::OnceCell::new();
            clocks.read_parse += thread_cpu() - t;
            let t = thread_cpu();
            if recovery == ryo_execution_domain::domain::coverage::types::Recovery::Whole
            {
                distinct.get_mut().clear();
                let mut evaluate = |screen: &mut pairs::Checks, jobs: &mut [(usize, usize, bool)], root: fn(usize) -> usize| {
                    screen.exclude(root, jobs, &mut bound, &mut run, tree, &provider.text, &distinct,
                        &bytes, &source.path, id, L::symbols(&symbols), &facts, &packet, unknown, cut_of, &said);
                };
                if let Some(screen) = &mut checks {
                    evaluate(screen, &mut active, |binding| binding);
                } else if selected.len() > 1 {
                    // A one-source open admission cannot amortize preparing a reader screen.
                    // Its coarse requirements and ordinary executor still apply.
                    let prepare = state.screens.groups(parser, programs, program_of, cut_of, &mut active,
                        |screen, jobs| evaluate(screen, jobs, |_| 0));
                    clocks.prepare += prepare;
                    clocks.screen -= prepare;
                }
            }
            clocks.screen += thread_cpu() - t;
            let mut facts_view = packet.view(&mut provider, L::symbols(&symbols), &bytes);
            let t = thread_cpu();
            let lines = active
                .iter()
                .any(|(b, _, _)| bound[*b].as_ref().unwrap().lines)
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
            let mut frame = None;
            let mut shared = frontier::FileInputs::new(&active,&bound,programs,program_of,cut_of);
            for (binding, slot, excluded) in active.drain(..) {
                use ryo_execution_domain::domain::trust::{service::matters, types::Bearing};
                let contract = &contracts[&binding];
                let b = bound[binding].as_mut().unwrap();
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
                    answers[slot] = Ok(Reported::default());
                    clocks.skipped += 1;
                    continue;
                }
                clocks.evaluated += 1;
                let version = version.get_or_init(|| {
                    finish::FileVersion::new(&tree, &bytes, id.0, facts_view.syntax_text())
                });
                let make_frame = || finish::session::frame(version, &plan, &supplied, lines.as_ref());
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
                    answers[slot] = value.map(|()| {
                        Reported {
                            rows: Vec::new(),
                            unknowns,
                        }
                    });
                    clocks.productive += usize::from(world.productive(slot));
                    clocks.refused += usize::from(answers[slot].is_err());
                    continue;
                }
                let value = finish::session::pair(&mut exec, current, &mut run, &mut facts_view, b.plan, &b.slots, &local_coverage);
                if value.is_err() { frame = None; exec.release_request(); }
                answers[slot] = value
                .map(|rows| Reported { rows, unknowns });
                clocks.productive += usize::from(answers[slot].as_ref().is_ok_and(|r| !r.rows.is_empty()));
                clocks.refused += usize::from(answers[slot].is_err());
            }
            if keep {
                held.read = true;
                held.text = provider.text;
                packet.rows.partials = partials;
                packet.rows.partials_at = partials_at;
                std::mem::swap(&mut held.facts, &mut packet.rows);
            }
            clocks.evaluate += thread_cpu() - t;
        }
    }
    if std::env::var_os("VEC_STEP_TIME").is_some() { eprintln!("SHARED_STEPS runs={} rows={}",exec.stats.total_runs,exec.stats.total_rows); }
    if keep {
        state.run = run;
    } else {
        std::mem::swap(&mut names.paths, &mut run.paths);
    }
}
