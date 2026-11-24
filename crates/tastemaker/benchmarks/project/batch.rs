//! The corpus runner's TSV is a list of invocations, not an execution schedule.
//! Only the final comparison has access to a reference digest.
#[path = "readers.rs"]
mod readers;
#[path="stored.rs"]
mod stored;

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
use crate::{json_input, json_output};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io::{BufRead, Write},
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Deserialize)]
pub(crate) struct Input {
    root: String,
    #[serde(deserialize_with = "read_paths")]
    files: Rc<Vec<String>>,
    #[serde(default, deserialize_with = "read_inventory")]
    inventory: Option<Rc<Vec<String>>>,
    repository: Option<Authority>,
    hosted: Vec<Hosted>,
    #[serde(deserialize_with = "read_parameters")]
    parameters: Parameters,
}

fn read_paths<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Rc<Vec<String>>, D::Error> {
    Vec::<String>::deserialize(input).map(Rc::new)
}

fn read_inventory<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Option<Rc<Vec<String>>>, D::Error> {
    Ok(Option::<Vec<String>>::deserialize(input)?.map(canonical_inventory))
}

fn canonical_inventory(mut paths: Vec<String>) -> Rc<Vec<String>> {
    paths.sort_unstable();
    paths.dedup();
    Rc::new(paths)
}

enum Work {
    Project(Rc<Answer>),
    Closed {
        project: Option<usize>,
        pairs: Vec<usize>,
        binding: usize,
        inventory: Rc<Vec<String>>,
        files: Rc<Vec<String>>,
    },
    Pairs(Vec<usize>),
    Refused(String),
    Invalid(String),
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
            Some(path) => json_input::decode(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string()),
            None => Ok(Self::default()),
        }
    }
}

type HostSchema = Vec<(String, Vec<(String, String)>)>;

struct Admission {
    id: usize,
    contract: Rc<session::completeness::Contract>,
    project_bases: Vec<l::Relation>,
    closed: bool,
    file_cells: bool,
    file_inventory: bool,
    cut: Option<Rc<finish::partition::Partition>>,
}

#[derive(Default)]
struct Batch {
    workers: Option<std::num::NonZeroUsize>,
    schedule: session::Schedule,
    program_of: Vec<usize>,
    bindings: Vec<Rc<[Hosted]>>,
    host_rows: HashMap<std::sync::Arc<[std::sync::Arc<[super::HostCell]>]>, usize>,
    binding_ids: HashMap<(usize, Vec<(usize, Option<String>)>), usize>,
    queries: HashMap<String, Rc<str>>,
    work: Vec<Work>,
    inventories: HashMap<PathBuf, Rc<Vec<String>>>,
    // A bench driver may supply the original closed inventory separately from
    // the admitted file set. Its key is the source root, shared by sibling rules.
    roots: HashMap<String, PathBuf>,
    authorities: Authorities,
    projects: Vec<ProjectSpec>,
    cut_of: Vec<Option<Rc<finish::partition::Partition>>>,
    local_bindings: HashMap<usize, usize>,
    project_keys: HashMap<(String, Authority, Rc<Vec<String>>), usize>,
    admissions: HashMap<(usize, HostSchema), Rc<Admission>>,
    bound_contracts: HashMap<usize, Rc<session::completeness::Contract>>,
    clocks: Clock,
    populations_built: usize,
    relations_built: usize,
    relations_rebuilt: usize,
}

#[derive(Default)]
struct Active<F: Default, D: Default> {
    keep: bool,
    retention: session::Retention<F,D>,
    readers: Vec<session::ReaderState>,
    world: session::World,
    answers: Vec<Answer>,
    aliases: Vec<usize>,
    contributions: Vec<Option<Rc<project::Contribution>>>,
    manifest_failures: Vec<l::FilePath>,
    populations: HashMap<usize, (BTreeSet<l::Relation>, Result<Population, String>)>,
}

impl Admission {
    fn prepare(id: usize, query: &str, program: &Program, hosted: &[Hosted]) -> Result<Self, String> {
        use ryo_values::domain::hosted::types::{
            ColumnLabel, HostedColumn, HostedName, HostedRelation,
        };
        let relations = hosted
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
        let contract = Rc::new(session::completeness::describe(query, program, relations)?);
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
        let numbered_host = hosted
            .iter()
            .any(|table| table.columns.iter().any(|(_, kind)| kind == "File"));
        let wide = contract.cross || numbered_host;
        let cut = if !scope_only && wide {
            let mut run = finish::Run::default();
            let declarations = hosted
                .iter()
                .map(|t| Hosted {
                    rows: Default::default(),
                    name: t.name.clone(),
                    columns: t.columns.clone(),
                    partial: t.partial.clone(),
                })
                .collect::<Vec<_>>();
            let _ = bind_tables(&declarations, &mut run, |_| None)?;
            let scoped = finish::partition::partition(
                &mut finish::Exec::new(&program.plan),
                &mut run,
                |n| {
                    held.iter().any(|r| relations::service::name(*r).0 == n)
                        || hosted.iter().any(|t| t.name == n)
                },
            )?;
            Some(Rc::new(scoped))
        } else {
            None
        };
        Ok(Self {
            id,
            contract,
            project_bases: held,
            closed: scope_only || wide,
            file_cells: numbered_host,
            file_inventory: program.bases.iter().any(|name| name == "filesystem.file"),
            cut,
        })
    }
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

    fn prepared_rule(&mut self,query:Rc<str>,parameters:&Parameters)->usize{
        let t = thread_cpu();
        let rule = self
            .schedule
            .prepare(query.clone(), parameters.clone(), || {
                let mut ctx = ryo_engine::domain::plan_lowering::types::RuleContext::default();
                ctx.parameters
                    .insert("query".into(), parameters.clone());
                Program::prepare("query", &query, ctx, &declaration(), Some("/"))
            });
        self.clocks.prepare += thread_cpu() - t;
        rule
    }
    fn register_query(
        &mut self,
        query: Rc<str>,
        mut req: Input,
        closed: Option<&Path>,
    ) -> Result<Work, String> {
        let rule=self.prepared_rule(query.clone(),&req.parameters);
        let program=match &self.schedule.programs[rule]{Ok(p)=>p,Err(reason)=>return Ok(Work::Invalid(reason.clone()))};
        req.hosted
            .retain(|h| program.bases.iter().any(|n| n == &h.name));
        let shape: Vec<_> = req
            .hosted
            .iter()
            .map(|t| (t.name.clone(), t.columns.clone()))
            .collect();
        let schema_key = (rule, shape);
        let class = match self.admissions.get(&schema_key) {
            Some(class) => class.clone(),
            None => {
                let class = Rc::new(Admission::prepare(self.admissions.len(), &query, program, &req.hosted)?);
                self.admissions.insert(schema_key, class.clone());
                class
            }
        };
        let held = &class.project_bases;
        let contract = &class.contract;
        if req.files.is_empty() && !contract.cross {
            return Ok(Work::Pairs(Vec::new()));
        }
        if class.closed {
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
                                json_input::decode(&bytes).map_err(|e| e.to_string())?;
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
                .or(snapshot)
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
            let binding = self.binding(rule, class.id, req.hosted);
            self.bound_contracts.insert(binding, Rc::clone(contract));
            let pairs = if let Some(c) = &class.cut {
                self.cut_of[binding] = Some(c.clone());
                // Caller rows are consumed only after closure. Sources with the same
                // prepared program and caller schema produce identical local outputs.
                let local = *self.local_bindings.entry(class.id).or_insert(binding);
                self.schedule.admit(req.root, &req.files, local)?
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
        let binding = self.binding(rule, class.id, req.hosted);
        self.bound_contracts.insert(binding, contract.clone());
        let pairs = self.schedule.admit(req.root, &req.files, binding)?;
        Ok(Work::Pairs(pairs))
    }

    fn binding(&mut self, rule: usize, class: usize, mut hosted: Vec<Hosted>) -> usize {
        let mut payloads = Vec::with_capacity(hosted.len());
        for table in &mut hosted {
            let id = if let Some((rows, &id)) = self.host_rows.get_key_value(&table.rows) {
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
        *self.binding_ids.entry((class, payloads)).or_insert_with(|| {
            self.program_of.push(rule);
            self.cut_of.push(None);
            self.bindings.push(hosted.into());
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
        // The closed, store-disabled capture cannot register another request.
        // Release the admission-only maps before the reader and join heaps grow.
        self.project_keys = Default::default();
        self.authorities = Default::default();
        self.admissions = Default::default();
        self.local_bindings = Default::default();
        self.release_admission();
        let mut state = Active::default();
        self.epoch(readers, &mut state);
        state.answers
    }

    fn release_admission(&mut self) {
        self.host_rows.clear();
        self.schedule.close();
        self.inventories.clear();
        self.roots.clear();
        self.queries.clear();
        self.binding_ids.clear();
    }

    fn epoch<L: session::source::Source>(&mut self,languages:&[L],state:&mut Active<L::File,L::Dictionary>) {
        self.epoch_with_store(languages,state,None)
    }
    fn epoch_with_store<L: session::source::Source>(
        &mut self,
        languages: &[L],
        state: &mut Active<L::File, L::Dictionary>,
        mut store:Option<&mut stored::Store>,
    ) {
        let Active {
            keep,
            retention,
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
        aliases.extend(aliases.len()..self.schedule.pairs);
        answers.resize_with(self.schedule.pairs, || Err(String::new()));
        contributions.resize_with(self.schedule.sources.len(), || None);
        if let Some(s)=store.as_deref_mut() {
            let started = thread_cpu();
            s.admit(self,world,answers,contributions);
            eprintln!("store_admit_cpu={:.6}", thread_cpu()-started);
            if !*keep {self.release_admission();}
        }
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
        self.clocks.pairs = self.schedule.pairs;
        let cuts: Vec<_> = self.cut_of.iter().map(|cut| cut.as_deref()).collect();
        let contracts = self.bound_contracts.iter().map(|(&id, c)| (id, c.as_ref())).collect();
        let hosted: Vec<_> = self.bindings.iter().map(|t| t.as_ref()).collect();
        let paths = (!*keep).then(|| std::sync::Arc::new(self.schedule.sources.iter().map(|src|src.path.clone()).collect()));
        let readers = languages.iter().map(Some).chain([None]);
        for (reader_id, (language, files)) in readers.zip(groups).enumerate() {
            if files.is_empty() {
                continue;
            }
            session::worker::dispatch(
                self.workers,
                languages,
                paths.as_ref(),
                &mut reader_states[reader_id],
                retention,
                *keep,
                language,
                (reader_id != unclaimed).then_some(reader_id),
                &manifests,
                contributions,
                &self.schedule.programs,
                &self.program_of,
                &contracts,
                &cuts,
                world,
                &self.schedule.sources,
                &files,
                aliases,
                answers,
                |binding, run| {
                    Ok(bind_tables(hosted[binding], run, |_| None)?
                        .into_iter()
                        .map(|(n, t)| (n, Rc::new(t)))
                        .collect())
                },
                &mut self.clocks,
            );
            if !*keep && store.is_none() {
                // Other readers have disjoint files. Answers and project contributions
                // already own everything the closed phase will read from this group.
                self.schedule.delivered(&files, *keep);
                for i in files {
                    self.schedule.sources[i].provided = None;
                }
            }
        }
        if let Some(s)=store.as_deref_mut() {
            let started = thread_cpu();
            s.local(self,world,answers,aliases,contributions);
            eprintln!("store_local_cpu={:.6}", thread_cpu()-started);
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
        self.schedule.delivered(&pending, *keep);
        self.finish_projects(languages, contributions, world, answers, populations, manifest_failures, *keep,store.as_deref_mut());

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
        mut store:Option<&mut stored::Store>,
    ) {
        let t = thread_cpu();
        // Bound values select a different population. Only changed column declarations
        // require another prepared layout for the same lowered program.
        let mut groups = BTreeMap::new();
        for (i, work) in self.work.iter().enumerate() {
            if let Work::Closed { binding, .. } = work {
                let rule = self.program_of[*binding];
                let tables = &self.bindings[*binding];
                let mut schema: Vec<_> = tables.iter().map(|t| (&t.name, &t.columns)).collect();
                schema.sort_unstable();
                groups
                    .entry((rule, schema))
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
                    if let Some(cut) = &self.cut_of[*binding] {
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
                        &self.bindings[*binding],
                        p,
                        &incomplete,
                    )
                    .map(|rows| session::Reported { rows, unknowns })
                }))};
                exec.release_request();
                if let Some(s)=store.as_deref_mut() {
                    let pop=project.and_then(|id| populations[&id].1.as_ref().ok());
                    s.closed(i,&answer,pop,project.map(|id| (id, &self.projects[id].authority)));
                }
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
            Work::Refused(reason) | Work::Invalid(reason) => return Err(reason),
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

pub(super) fn digest(rows: &[&finish::answer::Row]) -> String {
    let mut hash = Sha256::new();
    if rows.is_empty() { hash.update(b"\n"); }
    finish::json_output::ordered_rows(rows.iter().copied(), |line| {
        hash.update(line);
        hash.update(b"\n");
    });
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
            json_input::decode_line(&line.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let mapped = projects
            .next()
            .ok_or("missing closed inventory")?
            .map_err(|e| e.to_string())?;
        let project: Inventory = json_input::decode_line(&mapped).map_err(|e| e.to_string())?;
        if project.id != item.id {
            return Err("inventory and admitted request differ".into());
        }
        let bytes = std::fs::read(item.request).map_err(|e| e.to_string())?;
        let mut input: Input = json_input::decode(&bytes).map_err(|e| e.to_string())?;
        input.inventory = Some(canonical_inventory(project.inventory));
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
                json_output::packet_receipt(&id, Ok((r.rows.len(), digest(&r.rows), r.unknowns)))
            }
            Err(reason) => json_output::packet_receipt(&id, Err(reason)),
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
    batch_with_workers(readers, manifest, output, None);
}
fn batch_with_workers<L: session::source::Source>(
    readers: &[L],
    manifest: &str,
    output: &str,
    workers: Option<std::num::NonZeroUsize>,
) {
    batch_mode_with_workers(readers, manifest, output, None, workers, || {})
}
fn batch_mode<L:session::source::Source>(readers:&[L],manifest:&str,output:&str,root:Option<&Path>){
    batch_mode_with_workers(readers, manifest, output, root, None, || {})
}
fn batch_mode_with_workers<L:session::source::Source>(readers:&[L], manifest:&str, output:&str, root:Option<&Path>, workers:Option<std::num::NonZeroUsize>, completed: impl FnOnce()){
    let start = thread_cpu();
    let mut session = Batch { workers, ..Default::default() };
    if let Some(index) = std::env::var_os("TM_CORPUS_INVENTORIES") {
        let bytes = std::fs::read(index).expect("closed input inventories");
        session.roots = json_input::decode(&bytes).expect("source roots and their inventories");
    }
    session.authorities = Authorities::from_input().expect("repository declarations");
    let mut store=root.map(|p|stored::Store::open(p).expect("open request store"));
    let authority=store.as_mut().map(|s| {
        ["TM_CORPUS_INVENTORIES","TM_CORPUS_PROJECTS"].map(|name|std::env::var_os(name).map(|p|
            s.contract_file(Path::new(&p)).expect("read declared session inputs")))
    });
    enum Entry { Stored(stored::Key), Evaluate(usize,Option<stored::Key>) }
    let mut entries=Vec::new();
    let mut inherited_inventories=Vec::new();
    let file = std::fs::File::open(manifest).expect("input manifest");
    let mut invocations = Vec::new();
    let mut expected_text = std::collections::HashSet::<Rc<str>>::new();
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
            let contract=store.as_mut().and_then(|s| {
                let input=s.contract_file(Path::new(req))?;
                let query=s.contract_file(&Path::new(pack).join("rules/query/query.ryoql"))?;
                let inventory=match closed{Some(p)=>Some(s.contract_file(p)?),None=>None};
                Some(stored::key((input,query,inventory,authority)))
            });
            if let (Some(s),Some(k))=(&mut store,contract) && let Some((answer,_))=s.lookup(k) {
                entries.push(Entry::Stored(answer));
            }else {
                let result=std::fs::read(req).map_err(|e|e.to_string())
                    .and_then(|bytes|json_input::decode::<Input>(&bytes).map_err(|e|e.to_string()))
                    .and_then(|input|{
                        let inherited=if closed.is_none() && store.is_some(){session.roots.get(&input.root).cloned()}else{None};
                        let work=session.register(pack,input,closed)?;
                        if let (Some(p),Some(s))=(inherited,&mut store) && matches!(work,Work::Closed{..}|Work::Project(_)){
                            let expected=s.contract_file(&p);
                            inherited_inventories.push((session.work.len(),p,expected));
                        }
                        Ok(work)
                    }).unwrap_or_else(Work::Refused);
                let idx = session.work.len();
                session.work.push(result);
                if store.is_none() {
                    return idx;
                }
                entries.push(Entry::Evaluate(idx,contract));
            }
            entries.len()-1
        });
        // Expected receipt text is opaque until the completed results are compared.
        let expected = if let Some(existing) = expected_text.get(oracle) {
            existing.clone()
        } else {
            let text = Rc::<str>::from(oracle);
            expected_text.insert(text.clone());
            text
        };
        invocations.push((idx, Box::<str>::from(id), expected));
    }
    drop(inputs);
    drop(expected_text);
    let invocations = invocations.into_boxed_slice();
    let registration = thread_cpu() - start - session.clocks.prepare - session.clocks.project;
    let answers=if let Some(s)=&mut store {
        let mut state=Active::default();
        if !session.work.is_empty(){session.epoch_with_store(readers,&mut state,Some(s));}
        for (i,path,expected) in inherited_inventories {s.inventory(i,path,expected);}
        state.answers
    }else{session.run_sources(readers)};
    let t=thread_cpu();
    let mut out=std::io::BufWriter::new(std::fs::File::create(output).unwrap());
    // Project closure shares complete answers and file closure shares ordered slots.
    // Reuse evidence only for those exact identities in this immutable replay.
    #[derive(Hash, Eq, PartialEq)]
    enum Source<'a> {
        Project(*const Answer),
        Files(&'a [usize]),
    }
    let mut known = HashMap::new();
    let mut indices = Vec::with_capacity(session.work.len());
    enum Output<'a> { Packet(stored::Key), Computed(std::borrow::Cow<'a, str>, Vec<&'a (String, String)>) }
    let mut results: Vec<Result<Output<'_>, String>> = Vec::new();
    let empty = digest(&[]);
    let mut gathered = 0.0;
    let mut nonempty_digests = 0;
    for entry in entries.into_iter().chain(
        (0..if store.is_none() { session.work.len() } else { 0 }).map(|i| Entry::Evaluate(i, None))
    ) {
        let source = match (&entry, &store) {
            (Entry::Evaluate(i, _), None) => match &session.work[*i] {
                Work::Project(answer) if Rc::strong_count(answer) > 1 => Some(Source::Project(Rc::as_ptr(answer))),
                Work::Pairs(slots) => Some(Source::Files(slots)),
                _ => None,
            },
            _ => None,
        };
        let next = results.len();
        let index = source.map_or(next, |source| *known.entry(source).or_insert(next));
        indices.push(index);
        if index != next { continue; }
        let result = (|| {
            let packet=match entry {
                Entry::Stored(k)=>k,
                Entry::Evaluate(i,id)=>{
                    if let Work::Invalid(reason)=&session.work[i] {
                        if let (Some(s),Some(k))=(&mut store,id){s.decline(k,reason);}
                        return Err(reason.clone())
                    }
                    let start = thread_cpu();
                    let r=Batch::answer(&session.work[i],&answers).map_err(str::to_owned)?;
                    gathered += thread_cpu() - start;
                    if let (Some(s),Some(k))=(&mut store,id) && let Some(k)=s.save(k,i,&r){k}
                    else {
                        let digest = if r.rows.is_empty() {
                            std::borrow::Cow::Borrowed(empty.as_str())
                        } else {
                            nonempty_digests += 1;
                            std::borrow::Cow::Owned(digest(&r.rows))
                        };
                        return Ok(Output::Computed(digest, r.unknowns));
                    }
                }
            };
            Ok(Output::Packet(packet))
        })();
        results.push(result);
    }
    drop(known);
    let mut fragments = json_output::UnknownWriter::default();
    if let Some(s)=&mut store{
        let pub_start=thread_cpu();
        let written=s.publish().expect("publish session snapshot");
        eprintln!("publication_cpu={:.6}",thread_cpu()-pub_start);s.costs();
        eprintln!("retained request_hits={} pair_hits={} closed_hits={} evaluated_pairs={} evaluated_closed={} written={written}",s.request_hits,s.file_hits,s.closed_hits,s.evaluated_pairs,s.evaluated_closed);
    }
    let mut refusals = BTreeMap::new();
    for (i, id, expected) in invocations {
        let receipt = match &results[indices[i]] {
            Ok(Output::Packet(k)) => {
                let packet = store.as_ref().unwrap().packet(k);
                match &packet.refusal {
                    Some(reason) => Err(reason),
                    None => Ok((packet.digest.as_str(), packet.unknowns.as_slice(), &[][..])),
                }
            }
            Ok(Output::Computed(d, unknowns)) => Ok((d.as_ref(), &[][..], unknowns.as_slice())),
            Err(reason) => Err(reason),
        };
        match receipt {
            Ok((d, stored_unknowns, computed_unknowns)) => {
                let empty = stored_unknowns.is_empty() && computed_unknowns.is_empty();
                let status = if d == expected.as_ref() && empty {
                    "exact"
                } else {
                    "different"
                };
                write!(out, "{id}\t{status}\t{d}").unwrap();
                if !empty {
                    write!(out, "\t").unwrap();
                    fragments.write(&mut out, stored_unknowns.iter().chain(computed_unknowns.iter().copied()));
                }
                writeln!(out).unwrap();
            }
            Err(reason) => {
                *refusals.entry(reason.as_str()).or_insert(0usize) += 1;
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
    println!("RECEIPT_EVIDENCE registrations={} gathers={} nonempty_digests={} unknown_encodes={} assembly_sdk={gathered:.6} assembly_evidence={:.6}",
        session.work.len(), results.len(), nonempty_digests, fragments.len(), c.assemble - gathered);
    for (name, r) in &c.readers {
        println!("CHECK_READER {name} parsed={} total={:.6} read={:.6} build={:.6} tree={:.6} facts={:.6} other={:.6}",
            r.parsed, r.total, r.read, r.build, r.tree, r.facts, r.total - r.read - r.build - r.tree - r.facts);
    }
    println!(
        "CHECK_SESSION programs={} bindings={} sources={} reads={} parses={} pairs={} distinct_pairs={} skipped={} evaluated={} productive={} empty={} refused={} prep={:.6} read_parse={:.6} first={:.6} second={:.6} project={:.6} assembly={:.6} admission={:.6} read_versions={:.6} reader_build={:.6} reader_projection_facts={:.6} inventory={:.6} join={:.6} bound_proofs={:.6}",
        session.schedule.programs.len(),
        session.bindings.len(),
        session.schedule.sources.len(),
        c.reads,
        c.files,
        c.pairs,
        c.unique_pairs,
        c.skipped,
        c.evaluated, c.productive, c.evaluated-c.productive-c.refused, c.refused,
        c.prepare,
        c.read_parse,
        c.screen,
        c.evaluate,
        c.project,
        c.assemble,
        registration,
        c.read_versions,
        c.build_file,
        c.read_parse - c.read_versions - c.build_file,
        c.inventory,
        c.join,
        c.bound_proofs,
    );
    completed();
}

/// Standalone scenario entry, using the same scheduler and full typed output.
pub(crate) fn one(
    languages: Vec<Language>,
    pack: &str,
    input: Input,
    output: &str,
) -> Result<(), String> {
    let readers: Vec<_> = languages
        .into_iter()
        .map(session::source::Linked::from)
        .collect();
    one_source(&readers, pack, input, output)
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
    let mut stream = Vec::new();
    if rows.rows.is_empty() { stream.push(b'\n'); }
    finish::json_output::ordered_rows(rows.rows, |line| {
        stream.extend_from_slice(line);
        stream.push(b'\n');
    });
    std::fs::write(output, stream).map_err(|e| e.to_string())
}

#[cfg(feature = "product-session")]
#[path = "live/calls.rs"]
pub(super) mod calls;
