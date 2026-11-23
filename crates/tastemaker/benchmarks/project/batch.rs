//! The corpus runner's TSV is a list of invocations, not an execution schedule.
//! Only the final comparison has access to a reference digest.
use super::{Hosted, bind_tables, execute_bound, finish};
use crate::{
    check_session::{self as session, Answer, Clock, Program},
    declaration, thread_cpu,
};
use ryo::{
    languages::{Language, Recognition},
    linter::{Languages, RelativePath},
};
use serde::Deserialize;
use ryo_values::domain::finish_plans::expressions::types::PlanConstant;
use std::sync::Arc;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    io::{BufRead, Write},
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Deserialize)]
struct Input {
    workers: Option<std::num::NonZeroUsize>,
    root: String,
    files: Vec<String>,
    inventory: Option<Vec<String>>,
    hosted: Vec<Hosted>,
    #[serde(deserialize_with = "crate::json_input::constants")]
    parameters: BTreeMap<String, PlanConstant>,
}

enum Work {
    Project(Result<finish::json_output::Records, String>),
    Closed {
        binding: usize,
        inventory: Rc<Vec<String>>,
        files: Vec<String>,
    },
    Pairs(Vec<usize>),
    Refused(String),
}

#[derive(Default)]
struct Batch {
    workers: Option<std::num::NonZeroUsize>,
    schedule: session::Schedule,
    bindings: Vec<(usize, Arc<[Hosted]>)>,
    binding_ids: HashMap<(usize, Arc<[Hosted]>), usize>,
    queries: HashMap<String, Rc<str>>,
    work: Vec<Work>,
    inventories: HashMap<PathBuf, Rc<Vec<String>>>,
    // A bench driver may supply the original closed inventory separately from
    // the admitted file set. Its key is the source root, shared by sibling rules.
    roots: HashMap<String, PathBuf>,
    clocks: Clock,
}

impl Batch {
    fn register(&mut self, pack: &str, req: Input, closed: Option<&Path>) -> Result<Work, String> {
        let t = thread_cpu();
        if let Some(workers) = req.workers {
            if self.workers.is_some_and(|count| count != workers) { return Err("conflicting check-session worker counts".into()); }
            self.workers = Some(workers);
        }
        let query = match self.queries.get(pack) {
            Some(q) => q.clone(),
            None => {
                let q: Rc<str> =
                    std::fs::read_to_string(Path::new(pack).join("rules/query/query.ryoql"))
                        .map_err(|e| e.to_string())?
                        .into();
                self.queries.insert(pack.to_owned(), q.clone());
                q
            }
        };
        let rule = self.schedule.prepare(
            query.clone(),
            req.parameters.clone(),
            || {
                let mut ctx = ryo_engine::domain::plan_lowering::types::RuleContext::default();
                ctx.parameters.insert(
                    "query".into(),
                    req.parameters
                        .iter()
                        .map(|(k, v)| (k.as_str().into(), v.clone()))
                        .collect(),
                );
                Program::prepare("query", &query, ctx, &declaration(), Some("/"))
            },
        );
        self.clocks.prepare += thread_cpu() - t;
        let program = self.schedule.programs[rule]
            .as_ref()
            .map_err(Clone::clone)?;
        let scope_only = program
            .bases
            .iter()
            .all(|name| name == "filesystem.file" || req.hosted.iter().any(|h| &h.name == name));
        if scope_only {
            let t = thread_cpu();
            // Closed fixture inventories accompany the production input stream. If
            // only typed scalar host cells are read, no file identity needs binding.
            let closed = closed.or_else(|| self.roots.get(&req.root).map(PathBuf::as_path));
            let snapshot = if let Some(path) = closed {
                if !self.inventories.contains_key(path) {
                    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                    self.inventories.insert(
                        path.to_owned(),
                        Rc::new({
                            let mut paths: Vec<String> =
                                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                            paths.sort_unstable();
                            paths.dedup();
                            paths
                        }),
                    );
                }
                self.inventories.get(path).cloned()
            } else {
                None
            };
            let inventory = req
                .inventory
                .map(|mut paths| {
                    paths.sort_unstable();
                    paths.dedup();
                    Rc::new(paths)
                })
                .or(snapshot)
                .or_else(|| {
                    let has_file = req
                        .hosted
                        .iter()
                        .any(|h| h.columns.iter().any(|(_, k)| k == "File"));
                    let needs_inventory = program.bases.iter().any(|n| n == "filesystem.file");
                    (!has_file && (!req.files.is_empty() || !needs_inventory)).then(|| {
                        let mut paths = req.files.clone();
                        paths.sort_unstable();
                        Rc::new(paths)
                    })
                })
                .ok_or("incomplete project inventory")?;
            let binding = self.binding(rule, req.hosted);
            self.clocks.project += thread_cpu() - t;
            return Ok(Work::Closed {
                binding,
                inventory,
                files: req.files,
            });
        }
        if req.files.is_empty() {
            return Err("no complete source contribution for query".into());
        }
        if req
            .hosted
            .iter()
            .any(|h| h.columns.iter().any(|(_, k)| k == "File"))
        {
            return Err("file-valued caller table requires a numbered source inventory".into());
        }
        let binding = self.binding(rule, req.hosted);
        let pairs = self.schedule.admit(req.root, req.files, binding)?;
        Ok(Work::Pairs(pairs))
    }

    fn binding(&mut self, rule: usize, hosted: Vec<Hosted>) -> usize {
        let hosted: Arc<[Hosted]> = hosted.into();
        *self.binding_ids.entry((rule, hosted.clone())).or_insert_with(|| {
            self.bindings.push((rule, hosted));
            self.bindings.len() - 1
        })
    }

    fn run(&mut self, languages: Vec<Language>) -> Vec<Answer> {
        self.schedule.close();
        self.inventories.clear();
        self.roots.clear();
        self.queries.clear();
        self.binding_ids.clear();
        let mut aliases: Vec<_> = (0..self.schedule.pairs).collect();
        let mut answers: Vec<Answer> = (0..self.schedule.pairs)
            .map(|_| Err(String::new()))
            .collect();
        let linked: Vec<_> = languages.into_iter().map(|l| Languages(vec![l])).collect();
        let unclaimed = linked.len();
        let mut groups = vec![Vec::new(); linked.len() + 1];
        for (i, src) in self.schedule.sources.iter().enumerate() {
            let path = RelativePath(src.path.as_str().into());
            let mut found = linked
                .iter()
                .enumerate()
                .filter(|(_, l)| l.reads(&path) == Recognition::Reads);
            match (found.next(), found.next()) {
                (Some((index, _)), None) => groups[index].push(i),
                (None, None) => groups[unclaimed].push(i),
                _ => {
                    for &slot in src.bindings.values() {
                        answers[slot] = Err(format!("multiple source readers for {}", src.path));
                    }
                }
            }
        }
        let rules: Vec<_> = self.bindings.iter().map(|(r, _)| *r).collect();
        self.clocks.pairs = self.schedule.pairs;
        let readers = linked
            .into_iter()
            .map(|l| l.0.into_iter().next())
            .chain([None]);
        let paths: Arc<[String]> = self.schedule.sources.iter().map(|s| s.path.clone()).collect();
        for (language, files) in readers.zip(groups) {
            if files.is_empty() {
                continue;
            }
            let bindings = &self.bindings;
            session::visit(
                self.workers, language,
                &self.schedule.programs,
                &rules,
                &self.schedule.sources,
                &paths,
                &files,
                &mut aliases,
                &mut answers,
                |binding, run| {
                    Ok(bind_tables(&bindings[binding].1, run, |_| None)?
                        .into_iter()
                        .map(|(n, t)| (n, Rc::new(t)))
                        .collect())
                },
                &mut self.clocks,
            );
        }
        for work in &mut self.work {
            if let Work::Pairs(pairs) = work {
                for slot in pairs {
                    *slot = aliases[*slot];
                }
            }
        }
        self.finish_projects();
        answers
    }

    fn finish_projects(&mut self) {
        let t = thread_cpu();
        // Bound values select a different population. Only changed column declarations
        // require another prepared layout for the same lowered program.
        let mut groups = BTreeMap::new();
        for (i, work) in self.work.iter().enumerate() {
            if let Work::Closed { binding, .. } = work {
                let (rule, tables) = &self.bindings[*binding];
                let mut schema: Vec<_> = tables.iter().map(|t| (&t.name, &t.columns)).collect();
                schema.sort_unstable();
                groups
                    .entry((*rule, schema))
                    .or_insert_with(Vec::new)
                    .push(i);
            }
        }
        let mut project_run = finish::Run::default();
        project_run.empty = project_run.sym(b"");
        for ((rule, _), requests) in groups {
            let plan = &self.schedule.programs[rule].as_ref().unwrap().plan;
            let mut exec = finish::Exec::new(plan);
            for i in requests {
                let Work::Closed {
                    binding,
                    inventory,
                    files,
                } = &self.work[i]
                else {
                    unreachable!()
                };
                let answer = execute_bound(
                    &mut exec,
                    &mut project_run,
                    inventory,
                    files,
                    &self.bindings[*binding].1,
                );
                self.work[i] = Work::Project(answer);
            }
        }
        self.clocks.project += thread_cpu() - t;
    }

    fn answer<'a>(work: &'a Work, answers: &'a [Answer]) -> Result<Vec<&'a str>, &'a str> {
        match work {
            Work::Project(r) => r
                .as_ref()
                .map(finish::json_output::Records::ordered_lines)
                .map_err(String::as_str),
            Work::Refused(e) => Err(e),
            Work::Closed { .. } => Err("project inputs still open"),
            Work::Pairs(pairs) => {
                let mut rows = Vec::new();
                for &p in pairs {
                    rows.extend(
                        answers[p]
                            .as_ref()
                            .map_err(|e| {
                                if e.is_empty() {
                                    "no source reader"
                                } else {
                                    e.as_str()
                                }
                            })?
                            .iter()
                            .map(String::as_str),
                    );
                }
                rows.sort_unstable();
                Ok(rows)
            }
        }
    }
}

fn digest(rows: &[&str]) -> String {
    let mut hash = Sha256::new();
    if rows.is_empty() {
        hash.update(b"\n");
    }
    for row in rows {
        hash.update(row.as_bytes());
        hash.update(b"\n");
    }
    format!("{:x}", hash.finalize())
}

/// The committed benchmark passes the numbered inventory on stdin and source
/// roots in its adjacent manifest. Both streams describe inputs to one session.
pub(crate) fn packets(
    languages: Vec<Language>,
    manifest: &str,
    output: &str,
) -> Result<(), String> {
    #[derive(Deserialize)]
    struct Entry {
        id: String,
        pack: String,
        request: PathBuf,
    }
    #[derive(Deserialize)]
    struct Inventory {
        id: String,
        inventory: Vec<String>,
    }
    let mut session = Batch::default();
    let mut ids = Vec::new();
    let source = std::fs::File::open(manifest).map_err(|e| e.to_string())?;
    let stdin = std::io::stdin();
    let mut projects = stdin.lock().lines();
    for line in std::io::BufReader::new(source).lines() {
        let item: Entry =
            serde_json::from_str(&line.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let mapped = projects
            .next()
            .ok_or("missing closed inventory")?
            .map_err(|e| e.to_string())?;
        let project: Inventory = serde_json::from_str(&mapped).map_err(|e| e.to_string())?;
        if project.id != item.id {
            return Err("inventory and admitted request differ".into());
        }
        let bytes = std::fs::read(item.request).map_err(|e| e.to_string())?;
        let mut input: Input = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        input.inventory = Some(project.inventory);
        let work = session
            .register(&item.pack, input, None)
            .unwrap_or_else(Work::Refused);
        session.work.push(work);
        ids.push(item.id);
    }
    if projects.next().is_some() {
        return Err("extra closed inventory".into());
    }
    let answers = session.run(languages);
    let mut out =
        std::io::BufWriter::new(std::fs::File::create(output).map_err(|e| e.to_string())?);
    for (id, work) in ids.into_iter().zip(&session.work) {
        let result = match Batch::answer(work, &answers) {
            Ok(rows) => serde_json::json!({"id":id,"rows":rows.len(),"sha256":digest(&rows)}),
            Err(reason) => serde_json::json!({"id":id,"refused":reason}),
        };
        writeln!(out, "{result}").map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
}

pub(crate) fn batch(languages: Vec<Language>, manifest: &str, output: &str, workers: Option<std::num::NonZeroUsize>) {
    let start = thread_cpu();
    let mut session = Batch { workers, ..Default::default() };
    if let Some(index) = std::env::var_os("TM_CORPUS_INVENTORIES") {
        let bytes = std::fs::read(index).expect("closed input inventories");
        session.roots = serde_json::from_slice(&bytes).expect("source roots and their inventories");
    }
    let file = std::fs::File::open(manifest).expect("input manifest");
    let mut invocations = Vec::new();
    let mut inputs = HashMap::new();
    for l in std::io::BufReader::new(file).lines() {
        let line = l.expect("manifest line");
        let mut fields = line.split('\t');
        let (id, pack, req, oracle) = (
            fields.next().unwrap(),
            fields.next().unwrap(),
            fields.next().unwrap(),
            fields.next().unwrap(),
        );
        let closed = fields.next().map(Path::new);
        let key = (
            PathBuf::from(pack),
            PathBuf::from(req),
            closed.map(Path::to_owned),
        );
        let idx = *inputs.entry(key).or_insert_with(|| {
            let result = std::fs::read(req)
                .map_err(|e| e.to_string())
                .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|e| e.to_string()))
                .and_then(|input| session.register(pack, input, closed))
                .unwrap_or_else(Work::Refused);
            session.work.push(result);
            session.work.len() - 1
        });
        invocations.push((idx, id.to_owned(), oracle.to_owned()));
    }
    drop(inputs);
    let registration = thread_cpu() - start - session.clocks.prepare - session.clocks.project;
    let answers = session.run(languages);
    let t = thread_cpu();
    let mut out = std::io::BufWriter::new(std::fs::File::create(output).unwrap());
    let results: Vec<_> = session
        .work
        .iter()
        .map(|work| Batch::answer(work, &answers).map(|rows| digest(&rows)))
        .collect();
    let mut refusals = BTreeMap::new();
    for (i, id, expected) in invocations {
        match &results[i] {
            Ok(d) => writeln!(
                out,
                "{id}\t{}\t{d}",
                if d == &expected { "exact" } else { "different" }
            )
            .unwrap(),
            Err(reason) => {
                *refusals.entry(*reason).or_insert(0usize) += 1;
                writeln!(out, "{id}\trefused\t").unwrap();
            }
        }
    }
    out.flush().unwrap();
    for (reason, n) in refusals {
        println!("REFUSAL\t{n}\t{reason}");
    }
    let c = &mut session.clocks;
    c.assemble += thread_cpu() - t;
    println!(
        "CHECK_SESSION programs={} bindings={} sources={} reads={} parses={} pairs={} distinct_pairs={} skipped={} evaluated={} prep={:.6} read_parse={:.6} first={:.6} second={:.6} project={:.6} assembly={:.6} admission={:.6}",
        session.schedule.programs.len(),
        session.bindings.len(),
        session.schedule.sources.len(),
        c.reads,
        c.files,
        c.pairs,
        c.unique_pairs,
        c.skipped,
        c.evaluated,
        c.prepare,
        c.read_parse,
        c.screen,
        c.evaluate,
        c.project,
        c.assemble,
        registration,
    );
}

/// Standalone scenario entry, using the same scheduler and full typed output.
pub(crate) fn one(
    languages: Vec<Language>,
    pack: &str,
    input: &[u8],
    output: &str,
) -> Result<(), String> {
    let mut session = Batch::default();
    let req = crate::finish::json_input::decode(input).map_err(|e| e.to_string())?;
    let work = session.register(pack, req, None)?;
    session.work.push(work);
    let answers = session.run(languages);
    let rows = Batch::answer(&session.work[0], &answers)?;
    std::fs::write(output, rows.join("\n") + "\n").map_err(|e| e.to_string())
}
