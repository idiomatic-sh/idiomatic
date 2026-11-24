//! Bind typed pair witnesses to the source adapter's actual row populations.
use super::{FileId, FileSource, Program, Result, Run, l, reader_run};
use crate::{
    pair_checks::SyntaxText,
    pair_summary::{self, Source as FactSource},
};
use ryo_relations::domain::{catalog, labels::types::NameRef, relations};
use std::{collections::{BTreeSet, HashMap}, sync::Arc};

pub(super) struct Checks {
    program: pair_summary::Program,
    roots: Vec<Option<Vec<usize>>>,
    domains: Vec<Vec<(usize, usize)>>,
    live: Vec<bool>,
    patterns: crate::pair_checks::SymbolMatches,
    words: l::FileWords,
    row_guards: Vec<(usize, l::Relation)>,
    known: Vec<(usize, bool)>,
}
impl Checks {
    pub(super) fn new<L: FileSource>(
        parser: Option<&L>,
        programs: &[Result<Program>],
        program_of: &[usize],
        needed: &[bool],
        cuts: &[Option<Arc<crate::finish::partition::Partition>>],
    ) -> Self {
        let mut roots = vec![Vec::new(); program_of.len()];
        let mut domains = vec![Vec::new(); program_of.len()];
        for (binding, outputs) in roots.iter_mut().enumerate().filter(|(b, _)| needed[*b]) {
            let id = program_of[binding];
            let Ok(program) = &programs[id] else { continue };
            domains[binding] = domain_steps(cuts[binding].as_deref());
            *outputs = target_steps(program, cuts[binding].as_deref());
        }
        Self::from_roots(parser, programs, program_of, roots, domains)
    }
    fn from_roots<L: FileSource>(
        parser: Option<&L>, programs: &[Result<Program>], program_of: &[usize],
        mut roots: Vec<Vec<usize>>, mut domains: Vec<Vec<(usize, usize)>>,
    ) -> Self {
        let mut steps = vec![BTreeSet::new(); programs.len()];
        for (binding, outputs) in roots.iter().enumerate() {
            steps[program_of[binding]].extend(outputs.iter().copied());
            steps[program_of[binding]].extend(domains[binding].iter().map(|&(_, d)| d));
        }
        let mut plans = Vec::new();
        let mut targets = Vec::new();
        let mut indices = HashMap::new();
        let mut bases = BTreeSet::new();
        for (id, wanted) in steps.into_iter().enumerate().filter(|(_, s)| !s.is_empty()) {
            let program = programs[id].as_ref().expect("prepared file program");
            for &step in &wanted {
                indices.insert((id, step), indices.len());
            }
            targets.push(wanted.into_iter().collect());
            plans.push(program.typed.as_ref());
            bases.extend(program.bases.iter().map(String::as_str));
        }
        for (binding, outputs) in roots.iter_mut().enumerate() {
            for (_, d) in &mut domains[binding] {
                *d = indices[&(program_of[binding], *d)];
            }
            for step in outputs {
                *step = indices[&(program_of[binding], *step)];
            }
        }
        let mut language = crate::declaration();
        language.fact_family = reader_run::fact_family;
        let mut supplied = Vec::new();
        for name in bases {
            let Some(rel) = relations::service::relation_named(NameRef(name)) else {
                continue;
            };
            if catalog::service::scope_of(rel) != catalog::types::RowScope::File
                || relations::service::class_of(rel) == relations::types::Class::Engine
            {
                continue;
            }
            if parser.is_some_and(|p| p.expands(rel))
                || catalog::service::fact_form(rel) != relations::types::FactForm::NodeTable
            {
                supplied.push(name.to_owned());
                if parser.is_none_or(|p| p.available(rel)) {
                    language.facts.push(name.to_owned());
                }
            }
        }
        let kinds = parser.map_or_else(Vec::new, FileSource::kinds);
        let names: Vec<_> = kinds.into_iter().map(l::NameText).collect();
        let vocab = reader_run::pairs::vocabulary(&names, l::FieldNames(Vec::new()));
        language.text_kinds = (0..names.len())
            .map(|k| u16::try_from(k).expect("kind code"))
            .collect();
        let program = pair_summary::prepare_plans(
            &plans,
            Some(&targets),
            &vocab,
            &language,
            &supplied,
            Some(crate::finish::host::CURRENT_FILE),
            true,
        );
        // All requested outputs must be impossible to discharge the binding.
        // One unconditional output makes testing its siblings unnecessary.
        let roots = roots.into_iter().enumerate().map(|(binding, outputs)| {
            let unconditional = outputs.iter().any(|&r|
                matches!(program.needs[program.roots[r].1], pair_summary::Need::Any));
            (!unconditional || outputs.len() > 1 || !domains[binding].is_empty()).then_some(outputs)
        }).collect();
        let live = vec![false; program.roots.len()];
        let row_guards = program
            .needs
            .iter()
            .enumerate()
            .filter_map(|(i, need)| {
                let pair_summary::Need::Some(node) = *need else {
                    return None;
                };
                let pair_summary::Domain::Fact(slot, None) = program.domains[node] else {
                    return None;
                };
                let FactSource::Catalog(relation) = program.facts[slot].source else {
                    return None;
                };
                Some((i, relation))
            })
            .collect();
        Self {
            program,
            roots,
            domains,
            row_guards,
            known: Vec::new(),
            live,
            patterns: Default::default(),
            words: l::facts::file_words(&l::facts::base_words(Default::default(), &[])),
        }
    }
    pub(super) fn exclude(
        &mut self,
        root_for: impl Fn(usize) -> usize,
        active: &mut [(usize, usize, bool)],
        bound: &mut super::Bindings<'_>,
        run: &mut Run,
        tree: &l::NodeTable,
        text: Option<&[u32]>,
        distinct: &std::cell::RefCell<crate::pair_checks::FileSymbols>,
        bytes: &[u8],
        path: &str,
        id: FileId,
        symbols: &l::Symbols,
        requested: &[l::Relation],
        packet: &reader_run::packet::Packet,
        unknown: &HashMap<u32, u32>,
        cuts: &[Option<Arc<crate::finish::partition::Partition>>],
        partials: &[ryo_values::domain::stored_facts::types::ReadInPart],
    ) {
        self.live.fill(false);
        let mut any = false;
        for &(binding, _, excluded) in active.iter() {
            if !excluded && let Some(roots) = &self.roots[root_for(binding)]
                && bound[&binding].as_ref().expect("active binding").screenable != Some(false)
            {
                for &(_, d) in &self.domains[root_for(binding)] { self.live[d] = true; }
                for &root in roots {
                    self.live[root] = true;
                    any = true;
                }
            }
        }
        if any {
            self.known.clear();
            for &(need, relation) in &self.row_guards {
                if !partials.iter().any(|p| p.relation == relation)
                    && let Some(exists) = packet.has_rows(relation)
                {
                    self.known.push((need, exists));
                }
            }
            let column = |source, index, pattern: &pair_summary::regexes::Matcher| {
                let FactSource::Catalog(rel) = source else { return None };
                if partials.iter().any(|p| p.relation == rel) { return None; }
                packet.any_symbol(rel, index, |word| {
                    if let Some(text) = l::text_of(symbols, l::ByteSlice(bytes), word) {
                        return pattern.is_match(text.0.as_bytes());
                    }
                    let value = unknown.get(&word.0).copied().or(run.absent_word);
                    value.is_none_or(|id| pattern.is_match(&run.texts[id as usize]))
                })
            };
            self.program.file_columns(
                tree,
                bytes,
                Some(path.as_bytes()),
                text.map(|ids| SyntaxText {
                    ids,
                    symbols: &run.texts,
                    dictionary: &run.ids,
                    distinct: Some(distinct),
                    patterns: Some(&self.patterns),
                    column: Some(&column),
                }),
                &mut self.words,
                |source, words, out| {
                    let FactSource::Catalog(rel) = source else {
                        return false;
                    };
                    requested.contains(&rel)
                        && !partials.iter().any(|p| p.relation == rel)
                        && packet.columns(rel, symbols, bytes, id, l::node_table::kind_codes(tree).len(), words, out)
                },
                &mut self.live,
                None,
                &self.known,
            );
        }
        for (binding, _, excluded) in active {
            if *excluded {
                continue;
            }
            let Some(roots) = &self.roots[root_for(*binding)] else { continue };
            let cut = cuts[*binding].as_deref();
            let b = bound[binding].as_mut().unwrap();
            if partials.iter().any(|p| b.facts.contains(&p.relation)) {
                continue;
            }
            let any_absent = roots.iter().any(|&r| !self.live[r]);
            if !any_absent && !self.domains[root_for(*binding)].iter().any(|&(_, d)| !self.live[d]) {
                continue;
            }
            if !*b.screenable.get_or_insert_with(|| {
                crate::finish::session::certificate::certified_empty(
                    &mut crate::finish::Exec::new(b.plan),
                    run,
                    &b.hosted,
                    cut,
                )
            }) {
                continue;
            }
            if roots.iter().all(|&r| !self.live[r]) {
                *excluded = true;
                continue;
            }
            if let Some(cut) = cut {
                let (home, absent) = b.absent.get_or_insert_with(|| (id.0, Vec::new()));
                *home = id.0;
                absent.clear();
                absent.extend(roots.iter().map(|&r| !self.live[r]));
                for &(i, d) in &self.domains[root_for(*binding)] {
                    absent[i] |= !self.live[d]
                        && b.projections
                            .get(&cut.inputs[i])
                            .is_some_and(|p| p.needs_domain());
                }
                *excluded = absent.iter().all(|&v| v);
            }
        }
    }
}

fn domain_steps(cut: Option<&crate::finish::partition::Partition>) -> Vec<(usize, usize)> {
    cut.into_iter().flat_map(|c| c.inputs.iter().enumerate()
        .filter_map(|(i, n)| c.domains.get(n).map(|&(_, d)| (i, d)))).collect()
}
fn target_steps(program: &Program, cut: Option<&crate::finish::partition::Partition>) -> Vec<usize> {
    match cut {
        Some(cut) => cut.inputs.clone(),
        None => program.plan.emits.iter().map(|emit| emit.1).collect(),
    }
}

/// Open sessions receive more host bindings after each SDK call. A reader's
/// vocabulary and a prepared program's output steps survive those boundaries.
#[derive(Default)]
pub(super) struct Retained {
    shapes: HashMap<(usize, Vec<usize>, Vec<(usize, usize)>), usize>,
    bindings: Vec<Option<usize>>,
    programs: Vec<Checks>,
}
impl Retained {
    pub(super) fn groups<L: FileSource>(
        &mut self, parser: Option<&L>, programs: &[Result<Program>], program_of: &[usize],
        cuts: &[Option<Arc<crate::finish::partition::Partition>>],
        active: &mut [(usize, usize, bool)],
        mut visit: impl FnMut(&mut Checks, &mut [(usize, usize, bool)]),
    ) -> f64 {
        let mut prepare = 0.;
        self.bindings.resize(program_of.len(), None);
        for &(binding, _, excluded) in active.iter() {
            if excluded || self.bindings[binding].is_some() { continue; }
            let id = program_of[binding];
            let program = programs[id].as_ref().expect("active program");
            let steps = target_steps(program, cuts[binding].as_deref());
            let domains = domain_steps(cuts[binding].as_deref());
            let index = *self.shapes.entry((id, steps, domains)).or_insert_with_key(|(_, steps, domains)| {
                let start = super::thread_cpu();
                let screen = Checks::from_roots(parser, std::slice::from_ref(&programs[id]),
                    &[0], vec![steps.clone()], vec![domains.clone()]);
                let index = self.programs.len();
                self.programs.push(screen);
                prepare += super::thread_cpu() - start;
                index
            });
            self.bindings[binding] = Some(index);
        }
        let group = |job: &(usize, usize, bool)| (!job.2).then(|| self.bindings[job.0]).flatten();
        let sort = !active.is_sorted_by_key(group);
        if sort { active.sort_unstable_by_key(group); }
        for jobs in active.chunk_by_mut(|a, b| group(a) == group(b)) {
            if let Some(index) = group(&jobs[0]) { visit(&mut self.programs[index], jobs); }
        }
        // FileInputs shares contributions in the schedule's original binding order.
        if sort { active.sort_unstable_by_key(|job| job.0); }
        prepare
    }
}
