//! A relative path and all its overlaid versions belong to one worker, so the
//! existing version/binding deduplication sees every consumer of that path.
use super::{Answer, Clock, Program, Result, Run, Source, Tables, l, thread_cpu, visit_groups};
use crate::finisher::workers;
use std::{collections::BTreeMap, num::NonZeroUsize, path::Path, sync::Arc};

#[derive(Default)]
pub(super) struct Output {
    pub(super) aliases: Vec<(usize, usize)>,
    pub(super) answers: Vec<(usize, Answer)>,
    empty: Vec<usize>,
}

pub(super) enum Sink<'a> {
    Direct(&'a mut [usize], &'a mut [Answer]),
    Buffered(&'a mut Output),
}
impl Sink<'_> {
    pub(super) fn alias(&mut self, slot: usize, target: usize) {
        match self {
            Self::Direct(aliases, _) => aliases[slot] = target,
            Self::Buffered(out) => out.aliases.push((slot, target)),
        }
    }
    pub(super) fn answer(&mut self, slot: usize, answer: Answer) {
        match self {
            Self::Direct(_, answers) => answers[slot] = answer,
            Self::Buffered(out) if answer.as_ref().is_ok_and(Vec::is_empty) => out.empty.push(slot),
            Self::Buffered(out) => out.answers.push((slot, answer)),
        }
    }
}

pub(crate) fn visit(
    requested: Option<NonZeroUsize>,
    language: Option<l::Language>,
    programs: &[Result<Program>],
    program_of: &[usize],
    sources: &[Source],
    paths: &Arc<[String]>,
    selected: &[usize],
    aliases: &mut [usize],
    answers: &mut [Answer],
    host: impl Fn(usize, &mut Run) -> Result<Tables> + Sync,
    clocks: &mut Clock,
) {
    let inventory_start = thread_cpu();
    let parser = language.map(|lang| lang.parse_engine());
    let mut by_path = BTreeMap::<&str, Vec<usize>>::new();
    for &index in selected {
        by_path.entry(&sources[index].path).or_default().push(index);
    }
    let groups: Vec<_> = by_path.into_values().collect();
    let n = workers::count(requested, groups.len());
    let shards = workers::divide(0..groups.len(), n, |index| {
        let source = &sources[groups[index][0]];
        // A failed stat is recorded by the ordinary version reader as a refusal.
        let bytes =
            std::fs::metadata(Path::new(&*source.root).join(&source.path)).map_or(0, |m| m.len());
        Ok(bytes
            .max(1)
            .saturating_mul(source.bindings.len().max(1) as u64))
    })
    .expect("closed path groups");
    clocks.inventory += thread_cpu() - inventory_start;
    let work = |ids: &[usize], sink: &mut Sink<'_>| {
        visit_groups(
            parser.as_deref(),
            programs,
            program_of,
            sources,
            paths,
            &groups,
            ids,
            &host,
            sink,
        )
    };
    let completed = workers::run(
        &shards,
        |ids| {
            let time = work(ids, &mut Sink::Direct(aliases, answers));
            (Output::default(), time)
        },
        |ids| {
            let mut output = Output::default();
            let time = work(ids, &mut Sink::Buffered(&mut output));
            (output, time)
        },
    );
    let merge_start = thread_cpu();
    for (part, time) in completed {
        for (slot, target) in part.aliases {
            aliases[slot] = target;
        }
        for slot in part.empty {
            answers[slot] = Ok(Vec::new());
        }
        for (slot, result) in part.answers {
            answers[slot] = result;
        }
        clocks.prepare += time.prepare;
        clocks.read_parse += time.read_parse;
        clocks.screen += time.screen;
        clocks.evaluate += time.evaluate;
        clocks.files += time.files;
        clocks.reads += time.reads;
        clocks.unique_pairs += time.unique_pairs;
        clocks.skipped += time.skipped;
        clocks.evaluated += time.evaluated;
    }
    clocks.merge += thread_cpu() - merge_start;
}
