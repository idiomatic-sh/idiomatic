//! The corpus runner's TSV is a list of invocations, not an execution schedule.
//! Only the final comparison has access to a reference digest.
#[path = "readers.rs"]
mod readers;

use super::{Hosted, Parameters, bind_tables, execute_ids, finish, read_parameters};
use crate::check_session::project::{self, Authority, Population};
use crate::{
    check_session::{self as session, Answer, Clock, Program},
    declaration, thread_cpu,
};
use ryo::languages as l;
use ryo::{
    languages::{Language, Recognition},
    linter::{Languages, RelativePath},
};
use ryo_relations::domain::{catalog, labels::types::NameRef, relations};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io::{BufRead, Write},
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Deserialize)]
struct Input {
    root: String,
    files: Vec<String>,
    inventory: Option<Vec<String>>,
    repository: Option<Authority>,
    hosted: Vec<Hosted>,
    #[serde(deserialize_with = "read_parameters")]
    parameters: Parameters,
}

enum Work {
    Project(Rc<Answer>),
    Closed {
        project: Option<usize>,
        pairs: Vec<usize>,
        binding: usize,
        inventory: Rc<Vec<String>>,
        files: Vec<String>,
    },
    Pairs(Vec<usize>),
    Refused(String),
}

struct ProjectSpec {
    paths: Rc<Vec<String>>,
    slots: Vec<usize>,
    authority: Authority,
    wanted: BTreeSet<l::Relation>,
}

#[derive(Default, Deserialize)]
struct Authorities {
    #[serde(default)]
    roots: HashMap<String, Authority>,
    default: Option<Authority>,
}

impl Authorities {
    fn from_input() -> Result<Self, String> {
        match std::env::var_os("TM_CORPUS_PROJECTS") {
            Some(path) => serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string()),
            None => Ok(Self::default()),
        }
    }
}

#[derive(Default)]
struct Batch {
    schedule: session::Schedule,
    bindings: Vec<(usize, Rc<[Hosted]>)>,
    binding_ids: HashMap<(usize, Rc<[Hosted]>), usize>,
    queries: HashMap<String, Rc<str>>,
    work: Vec<Work>,
    inventories: HashMap<PathBuf, Rc<Vec<String>>>,
    // A bench driver may supply the original closed inventory separately from
    // the admitted file set. Its key is the source root, shared by sibling rules.
    roots: HashMap<String, PathBuf>,
    authorities: Authorities,
    projects: Vec<ProjectSpec>,
    cuts: HashMap<(usize, Vec<(String, Vec<(String, String)>)>), Rc<finish::partition::Partition>>,
    cut_of: HashMap<usize, Rc<finish::partition::Partition>>,
    local_bindings: HashMap<(usize, Vec<(String, Vec<(String, String)>)>), usize>,
    project_keys: HashMap<(String, Authority, Rc<Vec<String>>), usize>,
    contracts:
        HashMap<(usize, Vec<(String, Vec<(String, String)>)>), Rc<session::completeness::Contract>>,
    bound_contracts: HashMap<usize, Rc<session::completeness::Contract>>,
    clocks: Clock,
    populations_built: usize,
    relations_built: usize,
    relations_rebuilt: usize,
}

#[derive(Default)]
struct Active<F: Default, D: Default> {
    keep: bool,
    readers: Vec<session::ReaderState<F, D>>,
    world: session::World,
    answers: Vec<Answer>,
    aliases: Vec<usize>,
    contributions: Vec<Option<Rc<project::Contribution>>>,
    manifest_failures: Vec<l::FilePath>,
    populations: HashMap<usize, (BTreeSet<l::Relation>, Result<Population, String>)>,
}

impl Batch {
    fn register(&mut self, pack: &str, req: Input, closed: Option<&Path>) -> Result<Work, String> {
        let t = thread_cpu();
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
        self.clocks.prepare += thread_cpu() - t;
        self.register_query(query, req, closed)
    }

    fn register_query(
        &mut self,
        query: Rc<str>,
        mut req: Input,
        closed: Option<&Path>,
    ) -> Result<Work, String> {
        let t = thread_cpu();
        let rule = self
            .schedule
            .prepare(query.clone(), req.parameters.clone(), || {
                let mut ctx = ryo_engine::domain::plan_lowering::types::RuleContext::default();
                ctx.parameters
                    .insert("query".into(), req.parameters.clone());
                Program::prepare("query", &query, ctx, &declaration(), Some("/"))
            });
        self.clocks.prepare += thread_cpu() - t;
        let program = self.schedule.programs[rule]
            .as_ref()
            .map_err(Clone::clone)?;
        req.hosted
            .retain(|h| program.bases.iter().any(|n| n == &h.name));
        let shape: Vec<_> = req
            .hosted
            .iter()
            .map(|t| (t.name.clone(), t.columns.clone()))
            .collect();
        let schema_key = (rule, shape);
        let contract = if let Some(c) = self.contracts.get(&schema_key) {
            Rc::clone(c)
        } else {
            use ryo_values::domain::hosted::types::{
                ColumnLabel, HostedColumn, HostedName, HostedRelation,
            };
            let hosted = req
                .hosted
                .iter()
                .map(|t| {
                    let columns = t
                        .columns
                        .iter()
                        .map(|(n, ty)| {
                            let kind = match ty.as_str() {
                                "File" => l::CellKind::File,
                                "Text" => l::CellKind::Symbol,
                                "Number" => l::CellKind::Number,
                                _ => return Err(format!("unsupported host type {ty}")),
                            };
                            Ok(HostedColumn {
                                name: ColumnLabel(n.as_str().into()),
                                kind,
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    Ok(HostedRelation {
                        name: HostedName(t.name.as_str().into()),
                        columns,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let c = Rc::new(session::completeness::describe(&query, program, hosted)?);
            self.contracts.insert(schema_key.clone(), Rc::clone(&c));
            c
        };
        let held: Vec<_> = program
            .bases
            .iter()
            .filter_map(|n| relations::service::relation_named(NameRef(n)))
            .filter(|r| catalog::service::scope_of(*r) == catalog::types::RowScope::Project)
            .collect();
        let scope_only = program.bases.iter().all(|name| {
            name == "filesystem.file"
                || held.iter().any(|r| relations::service::name(*r).0 == name)
                || req.hosted.iter().any(|h| &h.name == name)
        });
        // File-valued caller cells use the admitted request's file dictionary.
        // The same partition can consume its source subtree locally and bind those
        // cells at the join once the inventory is numbered.
        let numbered_host = req
            .hosted
            .iter()
            .any(|table| table.columns.iter().any(|(_, kind)| kind == "File"));
        let wide = contract.cross || numbered_host;
        let cut = if !scope_only && wide {
            let schema = req
                .hosted
                .iter()
                .map(|t| (t.name.clone(), t.columns.clone()))
                .collect();
            let key = (rule, schema);
            if !self.cuts.contains_key(&key) {
                let mut run = finish::Run::default();
                let declarations = req
                    .hosted
                    .iter()
                    .map(|t| Hosted {
                        rows: Rc::default(),
                        ..t.clone()
                    })
                    .collect::<Vec<_>>();
                let _ = bind_tables(&declarations, &mut run, |_| None)?;
                let scoped = finish::partition::partition(
                    &mut finish::Exec::new(&program.plan),
                    &mut run,
                    |n| {
                        held.iter().any(|r| relations::service::name(*r).0 == n)
                            || req.hosted.iter().any(|t| t.name == n)
                    },
                )?;
                self.cuts.insert(key.clone(), Rc::new(scoped));
            }
            Some(self.cuts[&key].clone())
        } else {
            None
        };
        if req.files.is_empty() && !contract.cross {
            return Ok(Work::Pairs(Vec::new()));
        }
        if scope_only || wide {
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
            let project = if held.is_empty() {
                None
            } else {
                let authority = req
                    .repository
                    .or_else(|| {
                        self.authorities
                            .roots
                            .get(&req.root)
                            .or(self.authorities.default.as_ref())
                            .cloned()
                    })
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
            let binding = self.binding(rule, req.hosted);
            self.bound_contracts.insert(binding, Rc::clone(&contract));
            let pairs = if let Some(c) = cut {
                self.cut_of.insert(binding, c);
                // Caller rows are consumed only after closure. Sources with the same
                // prepared program and caller schema produce identical local outputs.
                let local = *self.local_bindings.entry(schema_key).or_insert(binding);
                self.schedule.admit(req.root, req.files.clone(), local)?
            } else {
                Vec::new()
            };
            self.clocks.project += thread_cpu() - t;
            return Ok(Work::Closed {
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
        let binding = self.binding(rule, req.hosted);
        self.bound_contracts.insert(binding, contract);
        let pairs = self.schedule.admit(req.root, req.files, binding)?;
        Ok(Work::Pairs(pairs))
    }

    fn binding(&mut self, rule: usize, hosted: Vec<Hosted>) -> usize {
        let hosted: Rc<[Hosted]> = hosted.into();
        let host_key = hosted.clone();
        *self.binding_ids.entry((rule, host_key)).or_insert_with(|| {
            self.bindings.push((rule, hosted));
            self.bindings.len() - 1
        })
    }

    fn run(&mut self, languages: Vec<Language>) -> Vec<Answer> {
        let readers: Vec<_> = languages
            .into_iter()
            .map(session::source::Linked::from)
            .collect();
        self.run_sources(&readers)
    }

    fn run_sources<L: session::source::Source>(&mut self, readers: &[L]) -> Vec<Answer> {
        self.schedule.close();
        self.inventories.clear();
        self.roots.clear();
        self.queries.clear();
        self.binding_ids.clear();
        let mut state = Active::default();
        self.epoch(readers, &mut state);
        state.answers
    }

    fn epoch<L: session::source::Source>(
        &mut self,
        languages: &[L],
        state: &mut Active<L::File, L::Dictionary>,
    ) {
        let Active {
            keep,
            readers: reader_states,
            world,
            answers,
            aliases,
            contributions,
            manifest_failures,
            populations,
        } = state;
        if world.run.texts.is_empty() {
            world.declare_words(languages);
        }
        let cuts: Vec<_> = (0..self.bindings.len())
            .map(|i| self.cut_of.get(&i).cloned())
            .collect();
        aliases.extend(aliases.len()..self.schedule.pairs);
        answers.resize_with(self.schedule.pairs, || Err(String::new()));
        contributions.resize_with(self.schedule.sources.len(), || None);
        let pending: Vec<_> = std::mem::take(&mut self.schedule.pending)
            .into_iter()
            .collect();
        let manifests: BTreeSet<_> = languages.iter().flat_map(|l| l.manifests()).collect();
        let unclaimed = languages.len();
        reader_states.resize_with(unclaimed + 1, Default::default);
        let mut groups = vec![Vec::new(); unclaimed + 1];
        for &i in &pending {
            let src = &self.schedule.sources[i];
            if src.bindings.is_empty() && contributions[i].is_some() {
                continue;
            }
            let path = src.path.as_str();
            let mut found = languages.iter().enumerate().filter(|(_, l)| l.reads(path));
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
        let readers = languages.iter().map(Some).chain([None]);
        for (reader_id, (language, files)) in readers.zip(groups).enumerate() {
            if files.is_empty() {
                continue;
            }
            session::visit(
                &mut reader_states[reader_id],
                *keep,
                language,
                (reader_id != unclaimed).then_some(reader_id),
                &manifests,
                contributions,
                &self.schedule.programs,
                &rules,
                &self.bound_contracts,
                &cuts,
                world,
                &self.schedule.sources,
                &files,
                aliases,
                answers,
                |binding, run| {
                    Ok(bind_tables(&self.bindings[binding].1, run, |_| None)?
                        .into_iter()
                        .map(|(n, t)| (n, Rc::new(t)))
                        .collect())
                },
                &mut self.clocks,
            );
        }
        if !*keep {
            reader_states.clear();
        }
        for work in &mut self.work {
            if let Work::Pairs(pairs) | Work::Closed { pairs, .. } = work {
                for slot in pairs {
                    *slot = aliases[*slot];
                }
            }
        }
        if !*keep {
            *aliases=Vec::new();
        }
        self.schedule.delivered(&pending);
        self.finish_projects(languages, contributions, world, answers, populations, manifest_failures, *keep);
    }

    fn finish_projects<L: session::source::Source>(
        &mut self,
        languages: &[L],
        contributions: &mut Vec<Option<Rc<project::Contribution>>>,
        world: &mut session::World,
        answers: &[Answer],
        populations: &mut HashMap<usize, (BTreeSet<l::Relation>, Result<Population, String>)>,
        manifest_failures: &mut Vec<l::FilePath>,
        keep: bool,
    ) {
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
            let population = (|| {
                let sources = s
                    .slots
                    .iter()
                    .map(|&slot| {
                        contributions[slot].clone().ok_or_else(|| {
                            format!("missing project input {}", self.schedule.sources[slot].path)
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                project::collect(
                    world,
                    languages,
                    &s.authority,
                    &s.paths,
                    &sources,
                    &s.wanted.iter().copied().collect::<Vec<_>>(),
                )
            })();
            populations.insert(id, (s.wanted.clone(), population));
        }
        if !keep {
            *contributions=Vec::new();
        }
        manifest_failures.clear();
        for id in active {
            if let Ok(population) = &populations[&id].1 {
                manifest_failures.extend(population.refused.iter().map(|r| r.path.clone()));
            }
        }
        let mut readers=readers::Closed::new(&self.work,&self.cut_of,keep);
        for ((rule, _), requests) in groups {
            let plan = &self.schedule.programs[rule].as_ref().unwrap().plan;
            let mut exec = finish::Exec::new(plan);
            for i in requests {
                let Work::Closed {
                    pairs,
                    project,
                    binding,
                    inventory,
                    files,
                } = &self.work[i]
                else {
                    unreachable!()
                };
                let population = project.map(|p| populations[&p].1.as_ref()).transpose();
                let answer = if let Some(prior)=readers.duplicate[i] {
                    let Work::Project(value)=&self.work[prior] else {unreachable!("same binding evaluated in request order")};
                    value.clone()
                } else {Rc::new(population.map_err(Clone::clone).and_then(|p| {
                    let mut carried = BTreeMap::new();
                    let contract = &self.bound_contracts[binding];
                    let mut incomplete = BTreeMap::<String, Vec<Option<u32>>>::new();
                    let mut unknowns = Vec::new();
                    if let Some(p) = p {
                        use ryo_execution_domain::domain::{
                            coverage::{
                                service::{repository, unknown_message},
                                types::{CoverageScope, PartialPlace},
                            },
                            trust::{service::matters, types::Bearing},
                        };
                        for partial in &p.partials {
                            if !contract
                                .needs
                                .reads
                                .iter()
                                .any(|r| matters(r, partial) == Bearing::Matters)
                            {
                                continue;
                            }
                            let file = match partial.at {
                                PartialPlace::Repository => None,
                                PartialPlace::File(f) => Some(f.0),
                            };
                            let path = file
                                .map(|f| l::FilePath(world.run.paths[f as usize].as_str().into()));
                            let scope = path
                                .as_ref()
                                .map_or(CoverageScope::Repository, CoverageScope::File);
                            let reason = unknown_message(partial, scope).0.into_string();
                            unknowns
                                .push((path.unwrap_or_else(repository).0.into_string(), reason));
                            incomplete
                                .entry(relations::service::name(partial.relation).0.to_owned())
                                .or_default()
                                .push(file);
                        }
                    }
                    if files.is_empty() && !inventory.is_empty() && contract.frontier {
                        let mut withheld = session::Reported::skipped(inventory);
                        withheld.unknowns.extend(unknowns);
                        return Ok(withheld);
                    }
                    if let Some(cut) = self.cut_of.get(binding) {
                        for &slot in pairs {
                            if let Some(partials) = world.unavailable.get(&slot) {
                                for (relation, file) in partials {
                                    incomplete
                                        .entry(relation.clone())
                                        .or_default()
                                        .push(Some(*file));
                                }
                            }
                            unknowns.extend(
                                answers[slot]
                                    .as_ref()
                                    .map_err(Clone::clone)?
                                    .unknowns
                                    .iter()
                                    .cloned(),
                            );
                        }
                        carried=readers.inputs(i,||world.joined(pairs,&cut.inputs))?;
                        if carried.len() != cut.inputs.len() {
                            return Err("source frontier has not been closed".into());
                        }
                    }
                    let numbers: Vec<_> = inventory.iter().map(|p| world.file(p)).collect();
                    execute_ids(
                        &mut exec,
                        &mut world.run,
                        &numbers,
                        &carried,
                        inventory,
                        files,
                        &self.bindings[*binding].1,
                        p,
                        &incomplete,
                    )
                    .map(|rows| session::Reported { rows, unknowns })
                }))};
                exec.release_request();
                if let Some(finished)=readers.release(i,pairs,*project,world) {
                    populations.remove(&finished);
                }
                self.work[i] = Work::Project(answer);
            }
        }
        self.clocks.project += thread_cpu() - t;
    }

    fn answer<'a>(work: &'a Work, answers: &'a [Answer]) -> Result<Receipt<'a>, &'a str> {
        let mut receipt = Receipt::default();
        match work {
            Work::Project(result) => receipt.append(result)?,
            Work::Closed { .. } => return Err("project inputs still open"),
            Work::Refused(reason) => return Err(reason),
            Work::Pairs(slots) => {
                for &slot in slots {
                    receipt.append(&answers[slot])?;
                }
            }
        }
        receipt.unknowns.sort_unstable();
        receipt.unknowns.dedup();
        Ok(receipt)
    }
}

#[derive(Default)]
struct Receipt<'a> {
    rows: Vec<&'a finish::answer::Row>,
    unknowns: Vec<&'a (String, String)>,
}
impl<'a> Receipt<'a> {
    fn append(&mut self, answer: &'a Answer) -> Result<(), &'a str> {
        let held = answer.as_ref().map_err(|s| {
            if s.is_empty() {
                "no source reader"
            } else {
                s.as_str()
            }
        })?;
        self.rows.extend(&held.rows);
        self.unknowns.extend(&held.unknowns);
        Ok(())
    }
}

pub(super) fn findings<'a>(rows: impl IntoIterator<Item = &'a finish::answer::Row>) -> Vec<String> {
    let mut result: Vec<_> = rows.into_iter().map(finish::answer::finding).collect();
    result.sort_unstable();
    result
}

fn digest(rows: &[&finish::answer::Row]) -> String {
    let mut hash = Sha256::new();
    if rows.is_empty() {
        hash.update(b"\n");
    }
    for row in findings(rows.iter().copied()) {
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
        repository: Option<Authority>,
    }
    let mut session = Batch::default();
    session.authorities = Authorities::from_input()?;
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
        input.repository = project.repository;
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
            Ok(r) => {
                serde_json::json!({"id":id,"rows":r.rows.len(),"sha256":digest(&r.rows),"unknowns":r.unknowns})
            }
            Err(reason) => serde_json::json!({"id":id,"refused":reason}),
        };
        writeln!(out, "{result}").map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
}

pub(crate) fn batch(languages: Vec<Language>, manifest: &str, output: &str) {
    let readers: Vec<_> = languages
        .into_iter()
        .map(session::source::Linked::from)
        .collect();
    batch_with_sources(&readers, manifest, output);
}

fn batch_with_sources<L: session::source::Source>(readers: &[L], manifest: &str, output: &str) {
    let start = thread_cpu();
    let mut session = Batch::default();
    if let Some(index) = std::env::var_os("TM_CORPUS_INVENTORIES") {
        let bytes = std::fs::read(index).expect("closed input inventories");
        session.roots = serde_json::from_slice(&bytes).expect("source roots and their inventories");
    }
    session.authorities = Authorities::from_input().expect("repository declarations");
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
    let answers = session.run_sources(readers);
    let t = thread_cpu();
    let mut out = std::io::BufWriter::new(std::fs::File::create(output).unwrap());
    let results: Vec<_> = session
        .work
        .iter()
        .map(|work| Batch::answer(work, &answers).map(|r| (digest(&r.rows), r.unknowns)))
        .collect();
    let mut refusals = BTreeMap::new();
    for (i, id, expected) in invocations {
        match &results[i] {
            Ok((d, unknowns)) => {
                let status = if d == &expected && unknowns.is_empty() {
                    "exact"
                } else {
                    "different"
                };
                write!(out, "{id}\t{status}\t{d}").unwrap();
                if !unknowns.is_empty() {
                    write!(out, "\t").unwrap();
                    serde_json::to_writer(&mut out, unknowns).expect("write unknown findings");
                }
                writeln!(out).unwrap();
            }
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
    println!(
        "PROJECT_POPULATIONS inventories={} builds={} relations={} repeated_relations={}",
        session.projects.len(), session.populations_built, session.relations_built, session.relations_rebuilt,
    );
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
    input: Value,
    output: &str,
) -> Result<(), String> {
    let readers: Vec<_> = languages
        .into_iter()
        .map(session::source::Linked::from)
        .collect();
    let req = serde_json::from_value(input).map_err(|e| e.to_string())?;
    one_source(&readers, pack, req, output)
}

fn one_source<L: session::source::Source>(
    readers: &[L],
    pack: &str,
    req: Input,
    output: &str,
) -> Result<(), String> {
    let mut session = Batch::default();
    let work = session.register(pack, req, None)?;
    session.work.push(work);
    let answers = session.run_sources(readers);
    let rows = Batch::answer(&session.work[0], &answers)?;
    if !rows.unknowns.is_empty() {
        return Err("partial answer requires typed unknown output".into());
    }
    std::fs::write(output, findings(rows.rows).join("\n") + "\n").map_err(|e| e.to_string())
}

#[cfg(feature = "product-session")]
#[path = "live/calls.rs"]
pub(super) mod calls;
