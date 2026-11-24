//! Dispatch all versions of a relative path together. Workers return answers and
//! immutable project contributions by the original admission slot.
use super::*;
use std::num::NonZeroUsize;
pub(crate) trait Slots<T> {
    fn set(&mut self, slot: usize, value: T);
}

impl<T> Slots<T> for [T] {
    fn set(&mut self, slot: usize, value: T) {
        self[slot] = value;
    }
}

impl<T> Slots<T> for Vec<(usize, T)> {
    fn set(&mut self, slot: usize, value: T) {
        self.push((slot, value));
    }
}

pub(crate) fn dispatch<L: FileSource>(
    request: Option<NonZeroUsize>,
    languages: &[L],
    paths: Option<&Arc<Vec<String>>>,
    state: &mut ReaderState<L::File, L::Dictionary>,
    keep: bool,
    parser: Option<&L>,
    reader_id: Option<usize>,
    manifests: &BTreeSet<&str>,
    contributions: &mut [Option<Rc<project::Contribution>>],
    programs: &[Result<Program>],
    rules: &[usize],
    contracts: &HashMap<usize, &completeness::Contract>,
    cuts: &[Option<&finish::partition::Partition>],
    world: &mut World,
    sources: &[Source],
    selected: &[usize],
    aliases: &mut [usize],
    answers: &mut [Answer],
    host: impl Fn(usize, &mut Run) -> Result<Tables> + Sync,
    clocks: &mut Clock,
) {
    let count = request
        .or_else(|| std::thread::available_parallelism().ok())
        .map_or(1, NonZeroUsize::get)
        .min(selected.len());
    // An open epoch retains reader-native symbols and files across successive calls.
    if keep || count <= 1 {
        if let Some(paths) = paths {
            state.run.paths = paths.clone();
        }
        visit(
            state,
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
    let work = |files: &[usize]| {
        let mut part = World::default();
        part.boundaries = Some(Vec::new());
        part.declare_words(languages);
        let prefix = part.run.texts.len();
        let mut a = Vec::new();
        let mut links = Vec::new();
        let mut notes = Vec::new();
        let mut time = Clock::default();
        let mut local = ReaderState::default();
        local.run.paths = paths.expect("closed file inventory").clone();
        visit(
            &mut local,
            false,
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
        (a, links, slots, owned, part.transfer(prefix), time)
    };
    std::thread::scope(|scope| {
        let threads: Vec<_> = shards[1..]
            .iter()
            .map(|s| scope.spawn(|| work(s)))
            .collect();
        let first = work(&shards[0]);
        let mut transfers = Vec::with_capacity(shards.len());
        for (a, links, slots, notes, part, time) in std::iter::once(first).chain(
            threads
                .into_iter()
                .map(|t| t.join().expect("file worker panicked")),
        ) {
            let start = thread_cpu();
            for (id, answer) in a {
                answers[id] = answer;
            }
            for (id, target) in links {
                aliases[id] = target;
            }
            let notes: Vec<_> = notes.into_iter().map(Rc::new).collect();
            for (slot, id) in slots {
                contributions[slot] = Some(notes[id].clone());
            }
            transfers.push(part);
            clocks.join += thread_cpu() - start;
            clocks.prepare += time.prepare;
            clocks.read_parse += time.read_parse;
            clocks.screen += time.screen;
            clocks.evaluate += time.evaluate;
            clocks.files += time.files;
            clocks.reads += time.reads;
            clocks.unique_pairs += time.unique_pairs;
            clocks.skipped += time.skipped;
            clocks.evaluated += time.evaluated;
            clocks.productive += time.productive;
            clocks.refused += time.refused;
        }
        let start = thread_cpu();
        world.receive(transfers, sources);
        clocks.join += thread_cpu() - start;
    });
}
