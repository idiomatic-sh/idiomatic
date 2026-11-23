//! A relative path and all its overlaid versions belong to one worker, so the
//! existing version/binding deduplication sees every consumer of that path.
use super::{Answer, Clock, Program, Result, Run, Source, Tables, l, visit_groups};
use crate::finisher::workers;
use std::{collections::BTreeMap, num::NonZeroUsize, path::Path, sync::Arc};

#[derive(Default)]
pub(super) struct Output {
    pub(super) aliases: Vec<(usize, usize)>,
    pub(super) answers: Vec<(usize, Answer)>,
    pub(super) clock: Clock,
}

pub(crate) fn visit(
    requested: Option<NonZeroUsize>,
    language: Option<l::Language>,
    programs: &[Result<Program>],
    program_of: &[usize],
    sources: &[Source],
    selected: &[usize],
    aliases: &mut [usize],
    answers: &mut [Answer],
    host: impl Fn(usize, &mut Run) -> Result<Tables> + Sync,
    clocks: &mut Clock,
) {
    let parser = language.map(|lang| lang.parse_engine());
    let mut by_path = BTreeMap::<&str, Vec<usize>>::new();
    for &index in selected {
        by_path.entry(&sources[index].path).or_default().push(index);
    }
    let groups: Vec<_> = by_path.into_values().collect();
    let paths: Arc<[_]> = sources.iter().map(|s| s.path.clone()).collect();
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
    let work = |ids: &[usize]| {
        visit_groups(
            parser.as_deref(),
            programs,
            program_of,
            sources,
            &paths,
            &groups,
            ids,
            &host,
        )
    };
    for part in workers::run(&shards, &work, &work) {
        for (slot, target) in part.aliases {
            aliases[slot] = target;
        }
        for (slot, result) in part.answers {
            answers[slot] = result;
        }
        let time = part.clock;
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
}
