//! Dispatch all versions of a relative path together. Workers return answers and
//! immutable project contributions by the original admission slot.
use super::*;
use std::num::NonZeroUsize;
pub(crate) trait Slots<T> {
    fn previous(&self, _slot: usize) -> Option<&T> { None }
    fn set(&mut self, slot: usize, value: T);
}

impl<T> Slots<T> for [T] {
    fn previous(&self, slot: usize) -> Option<&T> { self.get(slot) }
    fn set(&mut self, slot: usize, value: T) {
        self[slot] = value;
    }
}

impl<T> Slots<T> for Vec<(usize, T)> {
    fn set(&mut self, slot: usize, value: T) {
        self.push((slot, value));
    }
}

/// Unwritten slots are their own result. Allocate only for pages containing
/// two admitted slots proven to share the same source and binding.
#[derive(Default)]
pub(crate) struct PairAliases {
    pages: Vec<Option<Box<[usize; 256]>>>,
    writes: usize,
}
impl PairAliases {
    pub(crate) fn lookup(&self, id: usize) -> usize {
        self.pages.get(id / 256).and_then(Option::as_deref)
            .map_or(id, |page| page[id % 256])
    }

    pub(crate) fn is_empty(&self) -> bool { self.pages.is_empty() }

    pub(crate) fn counts(&self) -> (usize, usize) {
        (self.writes, self.pages.iter().flatten().count())
    }
}
impl Slots<usize> for PairAliases {
    fn set(&mut self, slot: usize, target: usize) {
        if target == slot { return; }
        let index = slot / 256;
        if self.pages.len() <= index { self.pages.resize_with(index + 1, || None); }
        let page = self.pages[index].get_or_insert_with(|| Box::new(std::array::from_fn(|i| index * 256 + i)));
        page[slot % 256] = target;
        self.writes += 1;
    }
}

pub(crate) fn dispatch<L: FileSource>(
    request: Option<NonZeroUsize>,
    languages: &[L],
    paths: Option<&Arc<Vec<String>>>,
    state: &mut ReaderState,
    retention: &mut Retention<L::File, L::Dictionary>,
    keep: bool,
    parser: Option<&L>,
    reader_id: Option<usize>,
    manifests: &BTreeSet<&str>,
    contributions: &mut [Option<Rc<project::Contribution>>],
    programs: &[Result<Program>],
    rules: &[usize],
    contracts: &[Arc<completeness::Contract>],
    cuts: &[Option<Arc<finish::partition::Partition>>],
    world: &mut World,
    sources: &[Source],
    selected: &[usize],
    aliases: &mut PairAliases,
    answers: &mut (impl Slots<Answer> + ?Sized),
    host: impl Fn(usize, &mut Run) -> Result<Tables> + Sync,
    clocks: &mut Clock,
) {
    let count = request
        .or_else(|| std::thread::available_parallelism().ok())
        .map_or(1, NonZeroUsize::get)
        .min(selected.len());
    // Reuse the open reader for small SDK calls and when a finite cache budget
    // requires a single global admission order.
    let parallel_open = keep && paths.is_some() && selected.len() >= 128 && !retention.has_limit();
    if (keep && !parallel_open) || count <= 1 {
        if !keep && let Some(paths) = paths {
            state.run.paths = paths.clone();
        }
        visit(
            state,
            retention,
            keep,
            parser,
            reader_id,
            manifests,
            contributions,
            programs,
            rules,
            contracts,
            cuts,
            world,
            sources,
            selected,
            aliases,
            answers,
            host,
            clocks,
        );
        return;
    }
    let inventory_wall = std::time::Instant::now();
    let inventory_start = thread_cpu();
    let mut by_path = BTreeMap::<&str, Vec<usize>>::new();
    for &id in selected {
        by_path.entry(&sources[id].path).or_default().push(id);
    }
    let mut jobs: Vec<_> = by_path
        .into_values()
        .map(|group| {
            let src = &sources[group[0]];
            let size = src.provided.as_ref().map_or_else(
                || {
                    std::fs::metadata(Path::new(src.root.as_ref()).join(&src.path))
                        .map_or(0, |m| m.len())
                },
                |bytes| bytes.len() as u64,
            );
            let weight = size.max(1).saturating_mul(src.bindings.len().max(1) as u64);
            (std::cmp::Reverse(weight), group)
        })
        .collect();
    jobs.sort_unstable();
    let mut shards = vec![Vec::new(); count.min(jobs.len())];
    let mut loads = std::collections::BinaryHeap::from_iter(
        (0..shards.len()).map(|id| std::cmp::Reverse((0u64, id))),
    );
    for (std::cmp::Reverse(weight), ids) in jobs {
        let mut next = loads.peek_mut().unwrap();
        let std::cmp::Reverse((total, id)) = &mut *next;
        shards[*id].extend(ids);
        *total = total.saturating_add(weight);
    }
    clocks.inventory += thread_cpu() - inventory_start;
    clocks.inventory_wall += inventory_wall.elapsed().as_secs_f64();
    clocks.file_shards += shards.len();
    let snapshots = &state.bytes;
    let work = |files: &[usize]| {
        let wall = std::time::Instant::now();
        let mut part = World::default();
        part.boundaries = Some(Vec::new());
        part.declare_words(languages);
        let prefix = part.run.texts.len();
        let mut a = Vec::new();
        let mut links = Vec::new();
        let mut notes = Vec::new();
        let mut time = Clock { measure_file_wall: true, ..Default::default() };
        let mut local = ReaderState::default();
        local.run.paths = paths.expect("file worker inventory").clone();
        if parallel_open {
            local.bytes.extend(files.iter().filter_map(|id| {
                snapshots.get(id).map(|value| (*id, value.clone()))
            }));
        }
        visit(
            &mut local,
            &mut Retention::default(),
            parallel_open,
            parser,
            reader_id,
            manifests,
            &mut notes,
            programs,
            rules,
            contracts,
            cuts,
            &mut part,
            sources,
            files,
            &mut links,
            &mut a,
            &host,
            &mut time,
        );
        // All aliases to one parsed version share its provider notes.
        let mut ids = HashMap::new();
        let mut unique = Vec::new();
        let mut slots = Vec::new();
        for (slot, contribution) in notes {
            if let Some(contribution) = contribution {
                let id = *ids.entry(Rc::as_ptr(&contribution)).or_insert_with(|| {
                    let id = unique.len();
                    unique.push(contribution);
                    id
                });
                slots.push((slot, id));
            }
        }
        let owned: Vec<_> = unique
            .into_iter()
            .map(|c| {
                Rc::try_unwrap(c)
                    .ok()
                    .expect("closed worker owns each contribution")
            })
            .collect();
        (a, links, slots, owned, part.transfer(prefix), time, local.bytes, wall.elapsed().as_secs_f64())
    };
    {
        let (results, children) = jobs::run(&shards, |shard| work(shard));
        clocks.file_workers_cpu += children;
        let mut transfers = Vec::with_capacity(shards.len());
        let mut bytes = Vec::new();
        let mut longest = 0.0_f64;
        for (a, links, slots, notes, part, time, snapshots, worker_wall) in results {
            longest = longest.max(worker_wall);
            clocks.shard_busy_wall += worker_wall;
            if time.largest_file_wall > clocks.largest_file_wall {
                clocks.largest_file_wall = time.largest_file_wall;
                clocks.largest_file.clone_from(&time.largest_file);
            }
            let merge_wall = std::time::Instant::now();
            let start = thread_cpu();
            for (id, answer) in a {
                answers.set(id, answer);
            }
            for (id, target) in links {
                aliases.set(id, target);
            }
            let notes: Vec<_> = notes.into_iter().map(Rc::new).collect();
            for (slot, id) in slots {
                if contributions[slot].is_none() {
                    contributions[slot] = Some(notes[id].clone());
                }
            }
            transfers.push(part);
            bytes.push(snapshots);
            clocks.join += thread_cpu() - start;
            clocks.join_wall += merge_wall.elapsed().as_secs_f64();
            clocks.read_versions += time.read_versions;
            clocks.build_file += time.build_file;
            for (name, part) in time.readers {
                let total = clocks.readers.entry(name).or_default();
                total.total += part.total;
                total.read += part.read;
                total.build += part.build;
                total.tree += part.tree;
                total.facts += part.facts;
                total.parsed += part.parsed;
            }
            clocks.prepare += time.prepare;
            clocks.read_parse += time.read_parse;
            clocks.screen += time.screen;
            clocks.bound_proofs += time.bound_proofs;
            clocks.named_cpu += time.named_cpu;
            for (sum, value) in clocks.named_counts.iter_mut().zip(time.named_counts) { *sum += value; }
            for (sum, value) in clocks.witnesses.iter_mut().zip(time.witnesses) { *sum += value; }
            clocks.literal_positions += time.literal_positions;
            clocks.requirement_programs += time.requirement_programs;
            clocks.requirement_bindings += time.requirement_bindings;
            clocks.evaluate += time.evaluate;
            clocks.files += time.files;
            clocks.reads += time.reads;
            clocks.unique_pairs += time.unique_pairs;
            clocks.skipped += time.skipped;
            clocks.evaluated += time.evaluated;
            clocks.certified += time.certified;
            clocks.productive += time.productive;
            clocks.empty += time.empty;
            clocks.refused += time.refused;
        }
        clocks.shard_critical_wall += longest;
        let merge_wall = std::time::Instant::now();
        let start = thread_cpu();
        world.receive(transfers, sources);
        clocks.join += thread_cpu() - start;
        clocks.join_wall += merge_wall.elapsed().as_secs_f64();
        bytes
    }.into_iter().for_each(|snapshots| state.bytes.extend(snapshots));
}
