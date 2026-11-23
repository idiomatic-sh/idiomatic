//! Schedule all consumers of a source before walking it. Keep the emitted rows per
//! bound program and source version. Parser scratch is released after its last consumer.
use super::{finish, first, lower, reader, reader_run, thread_cpu};
use finish::{Exec, Plan, Run, Table};
use ryo::languages::{self as l, ByteSlice, FileId, SourcePath};
use ryo_relations::domain::{catalog, labels::types::NameRef, relations};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
    rc::Rc,
};

type Result<T> = std::result::Result<T, String>;
pub(crate) type Answer = Result<Vec<String>>;
pub(crate) type Tables = Vec<(String, Rc<Table>)>;

pub(crate) struct Program {
    pub plan: Plan,
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
        let (mut plan, pack) = lower::query_parts(alias, text, context, true)?;
        {
            use ryo_values::domain::finish_plans::types::FinishStep;
            let typed = pack.typed.borrow();
            for (node, step) in plan.nodes.iter_mut().zip(&typed.as_ref().unwrap().steps) {
                if let FinishStep::Extend(args) = step {
                    node.j["args"]["value-order"] = serde_json::json!(
                        args.values.iter().map(|v| &v.name.0).collect::<Vec<_>>()
                    );
                }
                if node.op == "capture"
                    && node.j["args"]["separator"].is_null()
                    && let Some(separator) = capture_separator
                {
                    node.j["args"]["separator"] = separator.into();
                }
            }
        }
        let requirements = reader_run::file_requirements(&pack, declaration)
            .pop()
            .ok_or("missing query emit")?
            .1;
        let bases = plan
            .nodes
            .iter()
            .filter(|n| n.op == "base")
            .map(|n| n.j["rel"].as_str().unwrap().to_owned())
            .collect();
        Ok(Self {
            plan,
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
}

/// The root and path identify the caller's immutable effective source. This
/// distinguishes in-memory replacements staged at different roots.
pub(crate) struct Source {
    pub root: Rc<str>,
    pub path: String,
    pub bindings: BTreeMap<usize, usize>,
}

/// Per-check program identities and the inverse admission index. The caller binds
/// external relations and hands the index a stable identity for each binding.
#[derive(Default)]
pub(crate) struct Schedule {
    pub sources: Vec<Source>,
    source_ids: HashMap<(Rc<str>, Rc<str>), usize>,
    pub programs: Vec<Result<Program>>,
    prepared: HashMap<(Rc<str>, String), usize>,
    pub pairs: usize,
}

impl Schedule {
    pub(crate) fn close(&mut self) {
        self.prepared = HashMap::new();
        self.source_ids = HashMap::new();
    }
    pub(crate) fn prepare(
        &mut self,
        text: Rc<str>,
        parameters: String,
        make: impl FnOnce() -> Result<Program>,
    ) -> usize {
        *self.prepared.entry((text, parameters)).or_insert_with(|| {
            self.programs.push(make());
            self.programs.len() - 1
        })
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
            let rel = Path::new(&path);
            if rel
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return Err(format!("invalid relative source path {path}"));
            }
            let src = *self
                .source_ids
                .entry((root.clone(), Rc::from(path.as_str())))
                .or_insert_with(|| {
                    self.sources.push(Source {
                        root: root.clone(),
                        path,
                        bindings: BTreeMap::new(),
                    });
                    self.sources.len() - 1
                });
            let slot = *self.sources[src]
                .bindings
                .entry(binding)
                .or_insert_with(|| {
                    let slot = self.pairs;
                    self.pairs += 1;
                    slot
                });
            pairs.push(slot);
        }
        Ok(pairs)
    }
}

struct Bound<'p> {
    exec: Exec<'p>,
    facts: Vec<l::Relation>,
    hosted: Tables,
    shape: Vec<Vec<(bool, u32)>>,
    words: Vec<Vec<u32>>,
    never: bool,
    coverage: reader_run::coverage::Coverage,
}

fn bind<'p>(
    program: &'p Program,
    parser: &dyn l::contract::ParseEngine,
    run: &mut Run,
    hosted: Tables,
) -> Result<Bound<'p>> {
    let mut facts = Vec::new();
    for name in &program.bases {
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
        if parser.includes_expansions(rel) {
            run.reader_tables.insert(name.clone());
        }
        if catalog::service::fact_form(rel) != relations::types::FactForm::NodeTable
            || run.reader_tables.contains(name)
        {
            if parser.absence_of(rel) == Some(l::AbsenceCause::NotYetRead) {
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
    let exec = Exec::new(&program.plan);
    Ok(Bound {
        exec,
        facts,
        hosted,
        shape,
        words,
        never: *never,
        coverage: Default::default(),
    })
}

struct Version {
    origin: usize,
    data: Vec<u8>,
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
) -> Vec<Version> {
    use std::hash::{Hash, Hasher};
    use std::os::unix::fs::MetadataExt;
    let mut files = Vec::<Version>::new();
    let mut inodes = HashMap::new();
    let mut hashes = HashMap::<(u64, u32), Vec<usize>>::new();
    for &idx in group {
        let src = &sources[idx];
        let path = Path::new(&*src.root).join(&src.path);
        let found = (|| -> std::io::Result<usize> {
            let meta = std::fs::metadata(&path)?;
            if let Some(&i) = inodes.get(&(meta.dev(), meta.ino())) {
                return Ok(i);
            }
            let data = std::fs::read(path)?;
            *reads += 1;
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            data.hash(&mut hasher);
            let equal = hashes
                .entry((hasher.finish(), meta.mode() & 0o111))
                .or_default();
            let i = match equal.iter().copied().find(|&i| files[i].data == data) {
                Some(i) => i,
                None => {
                    let i = files.len();
                    files.push(Version {
                        origin: idx,
                        data,
                        jobs: BTreeMap::new(),
                    });
                    equal.push(i);
                    i
                }
            };
            inodes.insert((meta.dev(), meta.ino()), i);
            Ok(i)
        })();
        match found {
            Ok(i) => {
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

/// Each binding gets its own prepared column positions for this reader and host
/// schema. Symbols and file IDs live in the reader session. Source trees never
/// escape the visit. Every request for the same pair refers to its answer slot.
pub(crate) fn visit(
    language: l::Language,
    programs: &[Result<Program>],
    program_of: &[usize],
    sources: &[Source],
    selected: &[usize],
    aliases: &mut [usize],
    answers: &mut [Answer],
    mut host: impl FnMut(usize, &mut Run) -> Result<Tables>,
    clocks: &mut Clock,
) {
    let parser = language.parse_engine();
    let mut run = Run::default();
    run.file_field_codes = true;
    run.empty = run.sym(b"");
    run.links();
    let unknown = reader::public_words(&mut run, parser.unknown_reasons());
    run.kind_sym = parser
        .kind_names()
        .0
        .iter()
        .map(|k| run.sym(k.0.as_bytes()))
        .collect();
    // A local path may have many source versions. The IDs here identify the version.
    for src in sources {
        run.paths.push(src.path.clone());
        let word = run.sym(src.path.as_bytes());
        run.path_sym.push(word);
    }
    let mut bound = HashMap::new();
    let mut elements = l::Elements::default();
    let mut symbols = l::Symbols::default();
    let mut tree = l::NodeTable::default();
    let mut rows = l::contract::Collect::default();
    let mut by_path = BTreeMap::<&str, Vec<usize>>::new();
    for &index in selected {
        by_path.entry(&sources[index].path).or_default().push(index);
    }
    for group in by_path.values() {
        let t = thread_cpu();
        let versions = versions(group, sources, answers, aliases, &mut clocks.reads);
        clocks.read_parse += thread_cpu() - t;
        for Version {
            origin: index,
            data: bytes,
            jobs,
        } in versions
        {
            clocks.unique_pairs += jobs.len();
            let source = &sources[index];
            let id = FileId(index.try_into().expect("file ID"));
            let t = thread_cpu();
            for (&binding, &slot) in &jobs {
                let program = &programs[program_of[binding]];
                let entry = bound.entry(binding).or_insert_with(|| {
                    let p = program.as_ref().map_err(Clone::clone)?;
                    let tables = host(binding, &mut run)?;
                    bind(p, &*parser, &mut run, tables)
                });
                if let Err(reason) = entry {
                    answers[slot] = Err(reason.clone());
                }
            }
            clocks.prepare += thread_cpu() - t;
            let wanted: BTreeSet<_> = jobs
                .keys()
                .filter_map(|b| bound[b].as_ref().ok())
                .flat_map(|b| b.facts.iter().copied())
                .collect();
            if jobs.keys().all(|b| bound[b].is_err()) {
                continue;
            }
            let t = thread_cpu();
            run.root_dir.clear();
            run.root_dir.push_str(&source.root);
            let result = {
                let request = l::ParseRequest {
                    file: id,
                    path: SourcePath(&source.path),
                    bytes: ByteSlice(&bytes),
                    fields: l::FieldRequest::Wanted,
                    notes: l::NoteRequest::Unwanted,
                };
                parser
                    .build_file(
                        request,
                        &mut elements,
                        &mut symbols,
                        &wanted.into_iter().collect::<Vec<_>>(),
                    )
                    .map_err(|e| format!("{e:?}"))
            };
            clocks.files += 1;
            if let Err(reason) = result {
                for &slot in jobs.values() {
                    answers[slot] = Err(reason.clone());
                }
                clocks.read_parse += thread_cpu() - t;
                continue;
            }
            let mut provider = reader::ReaderFacts::new(id.0, &unknown);
            reader::project_tree(
                &elements,
                &symbols,
                &bytes,
                &mut run,
                &mut tree,
                &mut provider,
            );
            clocks.read_parse += thread_cpu() - t;
            let t = thread_cpu();
            use rustc_hash::FxHashSet;
            let mut kinds: FxHashSet<_> = l::node_table::kind_codes(&tree)
                .iter()
                .map(|&k| run.kind_sym[usize::from(k)])
                .collect();
            let mut fields: FxHashSet<_> = l::node_table::field_codes(&tree)
                .iter()
                .map(|&f| run.field_sym[usize::from(f)])
                .collect();
            let mut words: FxHashSet<_> = provider.text.iter().copied().collect();
            words.insert(run.empty);
            // Expanded semantic elements can extend beyond the concrete node table.
            for i in provider.text.len() as u32..l::end_of(&elements).0 {
                let at = l::Position(i);
                kinds.insert(run.kind_sym[usize::from(l::kind_at(&elements, at).0)]);
                fields.insert(reader::symbol(
                    &mut run,
                    &unknown,
                    &symbols,
                    &bytes,
                    l::field_name_at(&elements, at),
                ));
                words.insert(reader::symbol(
                    &mut run,
                    &unknown,
                    &symbols,
                    &bytes,
                    l::text_at(&elements, at),
                ));
            }
            use ryo_execution_domain::domain::coverage::{service::partial_of, types::Recovery};
            use ryo_language::domain::{
                diagnostics::types::SyntaxCompleteness,
                elements::service::{has_incomplete_lexical_stream, has_incomplete_syntax},
            };
            let recovery = if has_incomplete_syntax(&elements) == SyntaxCompleteness::Incomplete {
                if has_incomplete_lexical_stream(&elements) == SyntaxCompleteness::Incomplete {
                    Recovery::SyntaxErrors
                } else {
                    Recovery::LexicallyCompleteSyntaxErrors
                }
            } else {
                Recovery::Whole
            };
            let mut active = Vec::new();
            for (&binding, &slot) in &jobs {
                let Ok(b) = &bound[&binding] else { continue };
                let p = programs[program_of[binding]].as_ref().unwrap();
                if recovery != Recovery::Whole {
                    let rels: Vec<_> = p
                        .bases
                        .iter()
                        .filter_map(|n| relations::service::relation_named(NameRef(n)))
                        .collect();
                    if let Some(partial) = partial_of(&[], recovery, &rels).first() {
                        answers[slot] = Err(format!(
                            "{} is read only in part in {}: {}",
                            relations::service::name(partial.relation).0,
                            source.path,
                            partial.cause.0
                        ));
                        continue;
                    }
                }
                if b.never
                    || !b.shape.iter().all(|c| {
                        c.iter().any(|(k, v)| {
                            if *k {
                                kinds.contains(v)
                            } else {
                                fields.contains(v)
                            }
                        })
                    })
                    || !b.words.iter().all(|c| c.iter().any(|v| words.contains(v)))
                {
                    answers[slot] = Ok(Vec::new());
                    clocks.skipped += 1;
                    continue;
                }
                active.push((binding, slot));
            }
            clocks.screen += thread_cpu() - t;
            let t = thread_cpu();
            let wanted: BTreeSet<_> = active
                .iter()
                .flat_map(|(b, _)| bound[b].as_ref().unwrap().facts.iter().copied())
                .collect();
            let facts: Vec<_> = wanted.into_iter().collect();
            rows.clear();
            if !facts.is_empty() {
                parser.walk_file(
                    &mut elements,
                    &mut symbols,
                    ByteSlice(&bytes),
                    id,
                    &facts,
                    &mut rows,
                );
            }
            let partials = std::mem::take(&mut rows.partials);
            provider.receive(&mut run, &facts, &rows, &symbols, &bytes);
            clocks.read_parse += thread_cpu() - t;
            let t = thread_cpu();
            for (binding, slot) in active {
                let p = programs[program_of[binding]].as_ref().unwrap();
                let b = bound.get_mut(&binding).unwrap().as_mut().unwrap();
                if let Some((_, reason, _)) = partials
                    .iter()
                    .find(|(r, _, m)| b.facts.contains(r) && b.coverage.needed(&p.plan, *r, *m))
                {
                    answers[slot] = Err(reason.clone());
                    continue;
                }
                clocks.evaluated += 1;
                answers[slot] = finish::session::pair(
                    &mut b.exec,
                    &mut run,
                    id.0,
                    &tree,
                    &bytes,
                    &mut provider,
                    &b.hosted,
                );
            }
            clocks.evaluate += thread_cpu() - t;
        }
    }
}
