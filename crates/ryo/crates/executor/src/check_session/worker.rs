//! Dispatch all versions of a relative path together. Workers return answers and
//! immutable project contributions by the original admission slot.
use super::*;
use std::num::NonZeroUsize;
pub trait Slots<T> {
    fn previous(&self, _slot: usize) -> Option<&T> { None }
    fn set(&mut self, slot: usize, value: T);
    /// A pair whose answer is the empty report. The default builds and stores it.
    fn set_empty(&mut self, slot: usize, empty: impl FnOnce() -> T) { self.set(slot, empty()); }
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
pub struct PairAliases {
    pub pages: Vec<Option<Box<[usize; 256]>>>,
    pub writes: usize,
}
impl PairAliases {
    pub fn lookup(&self, id: usize) -> usize {
        self.pages.get(id / 256).and_then(Option::as_deref)
            .map_or(id, |page| page[id % 256])
    }

    pub fn is_empty(&self) -> bool { self.pages.is_empty() }

    pub fn counts(&self) -> (usize, usize) {
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

pub fn dispatch<L: FileSource>(
    request: Option<NonZeroUsize>,
    languages: &[L],
    paths: &Arc<Vec<String>>,
    state: &mut ReaderState,
    parser: Option<&L>,
    reader_id: Option<usize>,
    manifests: &BTreeSet<&str>,
    contributions: &mut [Option<Rc<project::Contribution>>],
    programs: &[crate::check_session::refusal::Planned],
    rules: &[usize],
    contracts: &[Arc<completeness::Contract>],
    cuts: &[Option<Arc<finish::partition::Partition>>],
    layouts: (&[Option<usize>], &[Option<project::Layout>]),
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
    if count <= 1
        || selected[1..].iter().all(|&id| sources[id].path == sources[selected[0]].path)
    {
        let previous_paths = std::mem::replace(&mut state.run.paths, Arc::clone(paths));
        visit(
            state,
            parser,
            reader_id,
            manifests,
            contributions,
            programs,
            rules,
            contracts,
            cuts,
            layouts,
            world,
            sources,
            selected,
            aliases,
            answers,
            host,
            clocks,
        );
        state.run.paths = previous_paths;
        return;
    }
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
    let work = |files: &[usize]| {
        let wall = std::time::Instant::now();
        let mut part = World::default();
        part.boundaries = Some(Vec::new());
        part.declare_words(languages);
        let prefix = part.run.texts.len();
        let mut a = Vec::new();
        let mut links = Vec::new();
        let mut notes = Vec::new();
        let mut time = Clock::default();
        let mut local = ReaderState::default();
        local.run.paths = Arc::clone(paths);
        visit(
            &mut local,
            parser,
            reader_id,
            manifests,
            &mut notes,
            programs,
            rules,
            contracts,
            cuts,
            layouts,
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
        let (results, children) = jobs::run(&shards, |shard| {
            let _budget = finish::PoolBudget::for_workers(shards.len());
            work(shard)
        });
        clocks.file_workers_cpu += children;
        let mut transfers = Vec::with_capacity(shards.len());
        for (a, links, slots, notes, part, time, snapshots, _) in results {
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
            state.bytes.extend(snapshots);
            clocks.prepare += time.prepare;
            clocks.read_parse += time.read_parse;
            clocks.screen += time.screen;
            clocks.bound_proofs += time.bound_proofs;
            for (sum, value) in clocks.exact_inputs.iter_mut().zip(time.exact_inputs) { *sum += value; }
            clocks.literal_positions += time.literal_positions;
            clocks.requirement_programs += time.requirement_programs;
            clocks.requirement_bindings += time.requirement_bindings;
            clocks.visit_projects += time.visit_projects;
            clocks.node_runs += time.node_runs;
            clocks.nodes_made += time.nodes_made;
            clocks.bulk_screen_files += time.bulk_screen_files;
            clocks.evaluate += time.evaluate;
            clocks.files += time.files;
            clocks.reads += time.reads;
            clocks.skipped += time.skipped;
            clocks.evaluated += time.evaluated;
            clocks.certified += time.certified;
            clocks.productive += time.productive;
            clocks.empty += time.empty;
            clocks.refused += time.refused;
        }
        world.receive(transfers, sources);
    }
}
