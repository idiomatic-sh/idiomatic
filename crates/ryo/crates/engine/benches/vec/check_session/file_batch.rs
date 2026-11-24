//! Packets waiting for the same reader's file-local program. Row positions,
//! coverage and projected facts retain the originating version throughout.
use super::*;

pub(super) struct Job {
    pub binding: usize,
    pub slot: usize,
    pub skip: bool,
    pub unavailable: Vec<String>,
    pub coverage: BTreeMap<String, Vec<Option<u32>>>,
    pub unknowns: Vec<(String, String)>,
    pub absent: Option<Vec<bool>>,
}

pub(super) struct Packet<'a, D> {
    pub index: usize,
    pub bytes: Arc<[u8]>,
    pub tree: l::NodeTable,
    pub native: Option<Arc<l::NodeTable>>,
    pub symbols: D,
    pub facts: reader_run::packet::Packet,
    pub provider: reader::ReaderFacts<'a>,
    pub fields: Vec<u32>,
    pub lines: Option<(String, Rc<Table>)>,
    pub active: Vec<(usize, usize, bool)>,
    pub jobs: Vec<Job>,
}

pub(super) fn execute<L: FileSource>(
    packets: &mut [Packet<'_, L::Dictionary>],
    demand: &mut finish::batch::Demand,
    exec: &mut Exec<'_>,
    run: &mut Run,
    supplied: &Tables,
    bound: &mut Bindings<'_>,
    programs: &[Result<Program>],
    program_of: &[usize],
    cuts: &[Option<&finish::partition::Partition>],
    reader: Option<usize>,
    world: &mut World,
    carried: &mut HashMap<u32, u32>,
    sources: &[Source],
    answers: &mut (impl Slots<Answer> + ?Sized),
    clocks: &mut Clock,
) {
    if packets.is_empty() {
        return;
    }
    let start = thread_cpu();
    let mut work = Vec::with_capacity(packets.len());
    let mut versions = Vec::with_capacity(packets.len());
    for file in packets.iter_mut().filter(|f| !f.jobs.is_empty()) {
        let tree = file.native.as_deref().unwrap_or(&file.tree);
        let text = file
            .provider
            .projects_words
            .then_some(file.provider.text.as_slice());
        versions.push(finish::FileVersion::new(
            tree,
            &file.bytes,
            file.index as u32,
            text,
        ));
        let words = file.provider.unknown;
        let provider = file
            .facts
            .view(&mut file.provider, L::symbols(&file.symbols), &file.bytes);
        work.push((
            file.index,
            words,
            provider,
            &mut file.fields,
            &file.lines,
            &file.active,
            &mut file.jobs,
        ));
    }
    let mut frames: Vec<_> = work
        .iter()
        .zip(&versions)
        .map(|(f, v)| finish::session::frame(v, exec.plan, supplied, f.4.as_ref()))
        .collect();
    if std::env::var_os("VEC_BATCH_COUNT").is_some() {
        let bindings: BTreeSet<_> = work
            .iter()
            .flat_map(|f| f.6.iter().filter(|j| !j.skip).map(|j| j.binding))
            .collect();
        let pairs: usize = work
            .iter()
            .map(|f| f.6.iter().filter(|j| !j.skip).count())
            .sum();
        eprintln!(
            "BATCH_PROGRAMS masks={} pairs={} packets={}",
            bindings.len(),
            pairs,
            work.len()
        );
    }
    // Populate shared node results in partitions before the sink consumes them.

    for (lane, (_, _, _, _, _, _, jobs)) in work.iter().enumerate() {
        for job in jobs.iter().filter(|j| !j.skip && j.coverage.is_empty()) {
            let binding = job.binding;
            let b = bound[&binding].as_ref().unwrap();
            let mut wanted = |node: usize| demand.include(b.slots[node], 1 << lane);
            if let Some(cut) = cuts[binding] {
                for (i, &n) in cut.inputs.iter().enumerate() {
                    if !job.absent.as_ref().is_some_and(|v| v[i]) && !b.projections.contains_key(&n)
                    {
                        wanted(n);
                    }
                }
            } else {
                for emit in &b.plan.emits {
                    wanted(emit.1);
                }
            }
        }
    }
    {
        let mut lanes: Vec<_> = frames
            .iter_mut()
            .zip(work.iter_mut())
            .map(|(frame, entry)| finish::batch::Lane {
                frame,
                facts: &mut entry.2,
                fields: &mut *entry.3,
                root: &sources[entry.0].root,
            })
            .collect();
        if !exec.prime_batch(run, &mut lanes, demand) {
            drop(lanes);
            for ((frame, v), entry) in frames.iter_mut().zip(&versions).zip(&work) {
                *frame = finish::session::frame(v, exec.plan, supplied, entry.4.as_ref());
            }
        }
    }
    for ((index, words, mut provider, fields, lines, active, jobs), (v, mut frame)) in
        work.into_iter().zip(versions.iter().zip(frames))
    {
        std::mem::swap(fields, &mut run.field_sym);
        run.root_dir.clear();
        run.root_dir.push_str(&sources[index].root);
        let mut shared = frontier::FileInputs::new(active, bound, programs, program_of, cuts);
        for job in jobs.drain(..) {
            let b = bound[&job.binding].as_mut().unwrap();
            if !job.unavailable.is_empty() {
                let file = world.file(&sources[index].path);
                world
                    .unavailable
                    .entry(job.slot)
                    .or_default()
                    .extend(job.unavailable.into_iter().map(|n| (n, file)));
            }
            if job.skip {
                if let Some(cut) = cuts[job.binding] {
                    world.insert_empty(job.slot, &sources[index].path, b, run, cut);
                }
                answers.set(job.slot, Ok(Reported::default()));
                continue;
            }
            let mut restricted;
            let current = if job.coverage.is_empty() {
                &mut frame
            } else {
                restricted = finish::session::frame(v, exec.plan, supplied, lines.as_ref());
                &mut restricted
            };
            let answer = if let Some(cut) = cuts[job.binding] {
                let r = shared.contribute(
                    b,
                    exec,
                    current,
                    run,
                    index as u32,
                    &mut provider,
                    cut,
                    &job.coverage,
                    world,
                    carried,
                    reader,
                    words,
                    job.slot,
                    job.absent.as_deref(),
                );
                clocks.productive += usize::from(world.productive(job.slot));
                r.map(|()| Vec::new())
            } else {
                let r = finish::session::pair(
                    exec,
                    current,
                    run,
                    &mut provider,
                    b.plan,
                    job.binding,
                    &b.slots,
                    &job.coverage,
                );
                clocks.productive += usize::from(r.as_ref().is_ok_and(|r| !r.is_empty()));
                r
            };
            if answer.is_err() {
                exec.release_request();
                frame = finish::session::frame(v, exec.plan, supplied, lines.as_ref());
                clocks.refused += 1;
            }
            answers.set(
                job.slot,
                answer.map(|rows| Reported {
                    rows,
                    unknowns: job.unknowns,
                }),
            );
        }
        std::mem::swap(fields, &mut run.field_sym);
    }
    clocks.evaluate += thread_cpu() - start;
}
