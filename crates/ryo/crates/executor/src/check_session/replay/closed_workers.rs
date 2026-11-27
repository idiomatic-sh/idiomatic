//! Closed programs can borrow the same immutable inputs after the last file.
use super::*;
use std::sync::Arc;

pub enum Inputs {
    Withheld(session::Reported),
    Ready {
        carried: finish::population::Inputs,
        incomplete: BTreeMap<String, Vec<Option<u32>>>,
        unknowns: Vec<(String, String)>,
        skipped: Vec<String>,
    },
}

/// The listed files of a program over the host's inventory that are not there, and so have no `filesystem.file` row. A program with a
/// frontier is told by its files' own visits; for one without, a read some other program made of the file says so, and the disk is
/// asked only of a file no read has reached.
pub fn absent_files(
    schedule: &session::Schedule,
    contract: &session::completeness::Contract,
    has_cut: bool,
    root: &Arc<str>,
    files: &[String],
) -> BTreeSet<String> {
    if has_cut || !contract.needs.reads.iter().any(|read| read.relation() == l::Relation::FilesystemFile) {
        return BTreeSet::new();
    }
    let ids = schedule.source_ids.get(root);
    files
        .iter()
        .filter(|path| match ids.and_then(|ids| ids.get(path.as_str())) {
            Some(&id) => schedule.sources[id].is_absent(),
            None => std::fs::metadata(std::path::Path::new(&**root).join(path))
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        })
        .cloned()
        .collect()
}

pub fn accepted_files<'a>(files: &'a [String], rejected: &[String]) -> std::borrow::Cow<'a, [String]> {
    if rejected.is_empty() { return std::borrow::Cow::Borrowed(files); }
    std::borrow::Cow::Owned(files.iter().filter(|p| rejected.binary_search(p).is_err()).cloned().collect())
}

pub fn inputs(
    run: &finish::Run,
    contract: &session::completeness::Contract,
    unavailable: &HashMap<usize, BTreeSet<(String, u32)>>,
    pairs: &[usize],
    inventory: &[String],
    files: &[String],
    missing: &BTreeSet<String>,
    cut: Option<&finish::partition::Partition>,
    p: Option<&Population>,
    answers: &ReportSlots,
    frontier: impl FnOnce(&[usize], &[usize]) -> Result<finish::population::Inputs, crate::check_session::refusal::Refusal>,
) -> Result<Inputs, crate::check_session::refusal::Refusal> {
    let mut carried = BTreeMap::new();
    let mut incomplete = BTreeMap::<String, Vec<Option<u32>>>::new();
    let mut unknowns = Vec::new();
    let mut skipped = Vec::new();
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
                .map(|f| l::FilePath(run.paths[f as usize].as_str().into()));
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
        withheld.extend_unknowns(unknowns);
        return Ok(Inputs::Withheld(withheld));
    }
    if let Some(cut) = cut {
        for &slot in pairs {
            if let Some(partials) = unavailable.get(&slot) {
                for (relation, file) in partials {
                    incomplete
                        .entry(relation.clone())
                        .or_default()
                        .push(Some(*file));
                }
            }
            let answer = answers[slot].as_ref().map_err(Clone::clone)?;
            unknowns.extend(answer.unknowns.iter().cloned());
            skipped.extend(answer.rejected_syntax.iter().cloned());
        }
        skipped.sort_unstable();
        skipped.dedup();
        if !files.is_empty() && files.iter().all(|p| skipped.binary_search(p).is_ok()) {
            unknowns.extend(session::Reported::skipped(&skipped).unknowns.iter().cloned());
            return Ok(Inputs::Withheld(session::Reported::new(Vec::new(), unknowns, skipped)));
        }
        // A pair whose file could not be read has no frontier: the file is unread, and the rule's other files are joined.
        // A pair whose file could not be read was run as a host-only file: it has its frontier, like every other pair.
        let readable: Vec<usize> = pairs.to_vec();
        carried = frontier(&readable, &cut.inputs)?;
        if carried.len() != cut.inputs.len() {
            return Err("source frontier has not been closed".into());
        }
    }
    // The host's inventory has no row for a listed file the visit found not there: a program that reads it says so for that file.
    if contract.needs.reads.iter().any(|read| read.relation() == l::Relation::FilesystemFile) {
        for path in files.iter().filter(|path| missing.contains(*path)) {
            if !unknowns.iter().any(|(at, _)| at == path) {
                unknowns.push((path.clone(), "the file is not there: it has no row in the file inventory".to_owned()));
            }
        }
    }
    Ok(Inputs::Ready { carried, incomplete, unknowns, skipped })
}

pub struct SharedTable<'a> {
    pub name: &'a str,
    pub names: &'a [String],
    pub types: &'a [finish::Ty],
    pub columns: &'a [Vec<u64>],
    pub rows: usize,
}

pub struct SharedPopulation<'a> {
    pub tables: Vec<SharedTable<'a>>,
    pub partials: &'a [ryo_execution_domain::domain::coverage::types::Partial],
}
impl<'a> SharedPopulation<'a> {
    pub fn from(p: &'a Population) -> Self {
        Self {
            tables: p.tables.iter().map(|(name, table)| SharedTable {
                name,
                names: &table.names,
                types: &table.tys,
                columns: &table.cols,
                rows: table.n,
            }).collect(),
            partials: &p.partials,
        }
    }

    // Provider tables are small and shared by every program in this worker.
    // The large file frontier shares its compressed columns by ownership.
    pub fn local(&self) -> Population {
        Population {
            tables: self.tables.iter().map(|table| {
                let data = finish::Table {
                    names: Rc::new(table.names.to_vec()),
                    tys: Rc::new(table.types.to_vec()),
                    cols: table.columns.to_vec(),
                    n: table.rows,
                };
                (table.name.to_owned(), Rc::new(data))
            }).collect(),
            partials: self.partials.to_vec(),
            refused: Vec::new(),
            consulted: Vec::new(),
            observations: Vec::new(),
        }
    }
}

pub struct Job<'a> {
    pub slot: usize,
    pub program: usize,
    pub project: Option<usize>,
    pub pairs: &'a [usize],
    pub inventory: &'a [String],
    pub inventory_key: usize,
    pub files: &'a [String],
    pub absent: BTreeSet<String>,
    pub hosted: &'a [Hosted],
    pub contract: &'a session::completeness::Contract,
    pub cut: Option<&'a finish::partition::Partition>,
}

pub fn run(
    count: usize,
    batch: &mut Batch,
    world: &mut session::World,
    populations: &HashMap<usize, (BTreeSet<l::Relation>, Result<Population, crate::check_session::refusal::Refusal>)>,
    answers: &ReportSlots,
    groups: impl Iterator<Item = Vec<usize>>,
    readers: &mut readers::Closed,
    reader_states: &[session::ReaderState],
) -> f64 {
    let mut programs = Vec::new();
    for slots in groups {
        let mut jobs = Vec::new();
        let mut plan = None;
        for slot in slots {
            if readers.duplicate[slot].is_some() {
                continue;
            }
            let Work::Closed { root, pairs, project, binding, inventory, files } = &batch.work[slot]
                else { unreachable!("closed program task") };
            let inventory_key = readers.inventory_id(slot);
            readers.numbers(slot, inventory, world);
            plan = Some(&batch.schedule.programs[batch.program_of[*binding]].as_ref().unwrap().plan);
            jobs.push(Job {
                slot,
                program: batch.program_of[*binding],
                project: *project,
                pairs,
                inventory,
                inventory_key,
                files,
                absent: absent_files(&batch.schedule, &batch.bound_contracts[*binding], batch.cut_of[*binding].is_some(), root, files),
                hosted: &batch.bindings[*binding],
                contract: &batch.bound_contracts[*binding],
                cut: batch.cut_of[*binding].as_deref(),
            });
        }
        if let Some(plan) = plan {
            programs.push((plan, jobs));
        }
    }
    let numbers = readers.numbered_inventories();
    let frontier = session::SharedFrontier::new(world, programs.iter().flat_map(|(_, jobs)| {
        jobs.iter().filter(|job| job.cut.is_some()).flat_map(|job| job.pairs.iter().copied())
    }));
    let available = &world.unavailable;
    let paths = &world.run.paths;
    let words = world.run.share_words();
    let empty = world.run.empty;
    let absent_word = world.run.absent_word;
    let unresolved_words = &world.run.unresolved_words;
    let link_sym = world.run.link_sym;
    let populations: HashMap<_, _> = populations.iter().map(|(&id, (_, population))| {
        (id, population.as_ref().map(SharedPopulation::from))
    }).collect();

    let file_catalog = &batch.file_catalog;
    let run_shard = |shard: usize| {
        let mut run = finish::Run::default();
        file_catalog.declare(&mut run);
        run.paths = Arc::clone(paths);
        run.empty = empty;
        run.absent_word = absent_word;
        run.unresolved_words.clone_from(unresolved_words);
        run.link_sym = link_sym;
        words.install(&mut run);
        let mut local_blocks = HashMap::new();
        let mut schemas = finish::population::LocalSchemas::default();
        let mut local_populations = HashMap::new();
        let mut output = Vec::new();
        for (plan, jobs) in programs.iter().skip(shard).step_by(count) {
            let mut exec = finish::Exec::new(plan);
            for job in jobs {
                let _profile = finish::operators::scope(job.program, "closed");
                let answer = (|| {
                    let p = match job.project {
                        Some(id) => {
                            let source = populations[&id].as_ref().map_err(|e| (*e).clone())?;
                            Some(&*local_populations.entry(id).or_insert_with(|| source.local()))
                        }
                        None => None,
                    };
                    let delivered = finish::operators::enter(usize::MAX, 0);
                    let ready = inputs(&run, job.contract, available, job.pairs, job.inventory, job.files, &job.absent,
                        job.cut, p, answers, |readable, steps| frontier.joined(readable, steps, &mut local_blocks, &mut schemas));
                    if let Some(start) = delivered {
                        finish::operators::leave(start, "closed-deliver", 0, true);
                    }
                    match ready?
                    {
                        Inputs::Withheld(answer) => Ok(answer),
                        Inputs::Ready { carried, incomplete, unknowns, skipped } => {
                            let evaluated = finish::operators::enter(usize::MAX - 1, 0);
                            let rows = execute_ids(&mut exec, &mut run, numbers[job.inventory_key].as_deref().expect("closed inventory assigned"), &carried,
                                job.inventory, job.files, &job.absent, job.hosted, p, &incomplete, None);
                            if let Some(start) = evaluated {
                                finish::operators::leave(start, "closed-execute", rows.as_ref().map_or(0, Vec::len), true);
                            }
                            rows.map(|rows| session::Reported::new(rows, unknowns, skipped))
                        }
                    }
                })();
                exec.release_request();
                output.push((job.slot, answer));
            }
        }
        let _ = finish::operators::flush();
        output
    };

    let (parts, cpu) = session::jobs::run(0..count.min(programs.len()), run_shard);
    let results = parts.into_iter().flatten();
    drop(programs);
    let mut coordinates = HashMap::new();
    for (slot, answer) in results {
        let Work::Closed { root, .. } = &batch.work[slot] else { unreachable!("closed result") };
        let placed = answer.and_then(|mut report| {
            batch.schedule.locate_closed(root, reader_states, &mut report, &mut coordinates)?;
            Ok(report)
        });
        batch.work[slot] = Work::Project(Rc::new(placed));
    }
    for (slot, prior) in readers.duplicate.iter().enumerate() {
        if let Some(prior) = prior {
            let Work::Project(answer) = &batch.work[*prior] else { unreachable!("closed request owner") };
            batch.work[slot] = Work::Project(answer.clone());
        }
    }
    cpu
}
