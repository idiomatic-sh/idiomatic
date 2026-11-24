//! Closed programs can borrow the same immutable inputs after the last file.
use super::*;
#[cfg(feature = "product-session")]
use std::sync::Arc;

pub(super) enum Inputs {
    Withheld(session::Reported),
    Ready {
        carried: finish::population::Inputs,
        incomplete: BTreeMap<String, Vec<Option<u32>>>,
        unknowns: Vec<(String, String)>,
    },
}

pub(super) fn inputs(
    run: &finish::Run,
    contract: &session::completeness::Contract,
    unavailable: &HashMap<usize, BTreeSet<(String, u32)>>,
    pairs: &[usize],
    inventory: &[String],
    files: &[String],
    cut: Option<&finish::partition::Partition>,
    p: Option<&Population>,
    answers: &[Answer],
    frontier: impl FnOnce(&[usize]) -> Result<finish::population::Inputs, String>,
) -> Result<Inputs, String> {
    let mut carried = BTreeMap::new();
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
        withheld.unknowns.extend(unknowns);
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
            unknowns.extend(
                answers[slot]
                    .as_ref()
                    .map_err(Clone::clone)?
                    .unknowns
                    .iter()
                    .cloned(),
            );
        }
        carried = frontier(&cut.inputs)?;
        if carried.len() != cut.inputs.len() {
            return Err("source frontier has not been closed".into());
        }
    }
    Ok(Inputs::Ready { carried, incomplete, unknowns })
}

#[cfg(feature = "product-session")]
struct SharedTable<'a> {
    name: &'a str,
    names: &'a [String],
    types: &'a [finish::Ty],
    columns: &'a [Vec<u64>],
    rows: usize,
}

#[cfg(feature = "product-session")]
struct SharedPopulation<'a> {
    tables: Vec<SharedTable<'a>>,
    partials: &'a [ryo_execution_domain::domain::coverage::types::Partial],
}
#[cfg(feature = "product-session")]
impl<'a> SharedPopulation<'a> {
    fn from(p: &'a Population) -> Self {
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
    fn local(&self) -> Population {
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

#[cfg(feature = "product-session")]
struct Job<'a> {
    slot: usize,
    project: Option<usize>,
    pairs: &'a [usize],
    inventory: &'a [String],
    inventory_key: usize,
    files: &'a [String],
    hosted: &'a [Hosted],
    contract: &'a session::completeness::Contract,
    cut: Option<&'a finish::partition::Partition>,
}

#[cfg(feature = "product-session")]
pub(super) fn run(
    count: usize,
    batch: &mut Batch,
    world: &mut session::World,
    populations: &HashMap<usize, (BTreeSet<l::Relation>, Result<Population, String>)>,
    answers: &[Answer],
    groups: impl Iterator<Item = Vec<usize>>,
    duplicate: &[Option<usize>],
) -> f64 {
    let mut programs = Vec::new();
    let mut numbers = HashMap::<usize, Vec<u32>>::new();
    for slots in groups {
        let mut jobs = Vec::new();
        let mut plan = None;
        for slot in slots {
            if duplicate[slot].is_some() {
                continue;
            }
            let Work::Closed { pairs, project, binding, inventory, files } = &batch.work[slot]
                else { unreachable!("closed program task") };
            let inventory_key = Rc::as_ptr(inventory) as usize;
            numbers.entry(inventory_key).or_insert_with(|| {
                inventory.iter().map(|path| world.file(path)).collect()
            });
            plan = Some(&batch.schedule.programs[batch.program_of[*binding]].as_ref().unwrap().plan);
            jobs.push(Job {
                slot,
                project: *project,
                pairs,
                inventory,
                inventory_key,
                files,
                hosted: &batch.bindings[*binding],
                contract: &batch.bound_contracts[binding],
                cut: batch.cut_of[*binding].as_deref(),
            });
        }
        if let Some(plan) = plan {
            programs.push((plan, jobs));
        }
    }
    let frontier = session::SharedFrontier::new(world, programs.iter().flat_map(|(_, jobs)| {
        jobs.iter().filter(|job| job.cut.is_some()).flat_map(|job| job.pairs.iter().copied())
    }));
    let available = &world.unavailable;
    let paths = &world.run.paths;
    let words: Vec<&[u8]> = world.run.texts.iter().map(|word| word.as_ref()).collect();
    let empty = world.run.empty;
    let absent_word = world.run.absent_word;
    let unresolved_words = &world.run.unresolved_words;
    let link_sym = world.run.link_sym;
    let populations: HashMap<_, _> = populations.iter().map(|(&id, (_, population))| {
        (id, population.as_ref().map(SharedPopulation::from))
    }).collect();

    let run_shard = |shard: usize| {
        let start = thread_cpu();
        let mut run = finish::Run::default();
        run.paths = Arc::clone(paths);
        run.empty = empty;
        run.absent_word = absent_word;
        run.unresolved_words.clone_from(unresolved_words);
        run.link_sym = link_sym;
        for text in &words {
            run.sym(text);
        }
        let mut local_blocks = HashMap::new();
        let mut local_populations = HashMap::new();
        let mut output = Vec::new();
        for (plan, jobs) in programs.iter().skip(shard).step_by(count) {
            let mut exec = finish::Exec::new(plan);
            for job in jobs {
                let answer = (|| {
                    let p = match job.project {
                        Some(id) => {
                            let source = populations[&id].as_ref().map_err(|e| (*e).clone())?;
                            Some(&*local_populations.entry(id).or_insert_with(|| source.local()))
                        }
                        None => None,
                    };
                    match inputs(&run, job.contract, available, job.pairs, job.inventory, job.files,
                        job.cut, p, answers, |steps| frontier.joined(job.pairs, steps, &mut local_blocks))?
                    {
                        Inputs::Withheld(answer) => Ok(answer),
                        Inputs::Ready { carried, incomplete, unknowns } => {
                            execute_ids(&mut exec, &mut run, &numbers[&job.inventory_key], &carried,
                                job.inventory, job.files, job.hosted, p, &incomplete)
                                .map(|rows| session::Reported { rows, unknowns })
                        }
                    }
                })();
                exec.release_request();
                output.push((job.slot, answer));
            }
        }
        (output, thread_cpu() - start)
    };

    let (results, cpu) = std::thread::scope(|scope| {
        let workers: Vec<_> = (1..count.min(programs.len()))
            .map(|shard| scope.spawn(move || run_shard(shard)))
            .collect();
        let (mut output, _) = run_shard(0);
        let mut cpu = 0.;
        for worker in workers {
            let (part, used) = worker.join().expect("closed program worker panicked");
            output.extend(part);
            cpu += used;
        }
        (output, cpu)
    });
    drop(programs);
    for (slot, answer) in results {
        batch.work[slot] = Work::Project(Rc::new(answer));
    }
    for (slot, prior) in duplicate.iter().enumerate() {
        if let Some(prior) = prior {
            let Work::Project(answer) = &batch.work[*prior] else { unreachable!("closed request owner") };
            batch.work[slot] = Work::Project(answer.clone());
        }
    }
    cpu
}
