//! Bind typed pair witnesses to the source adapter's actual row populations.
#[path = "proof_atoms.rs"]
mod atoms;

use super::{FileId, FileSource, Program, Result, Run, l, reader_run};
use crate::{
    pair_checks::{AtomAnswers, Bits, SharedAtoms, SyntaxText},
    pair_summary::{self, LocationColumn, Source as FactSource},
};
use ryo_relations::domain::{catalog, labels::types::NameRef, relations};
use ryo_values::domain::finish_plans::types::FinishStep;
use std::{collections::{BTreeSet, HashMap}, sync::Arc};

pub(super) struct Checks {
    program: pair_summary::Program,
    roots: Vec<Option<Vec<usize>>>,
    lefts: Vec<Vec<usize>>,
    pub(super) exact_roots: Vec<(bool, bool)>,
    domains: Vec<Vec<(usize, usize)>>,
    live: Vec<bool>,
    selected: Vec<u64>,
    locatable: Vec<u64>,
    atoms: Vec<Option<usize>>,
    words: l::FileWords,
    row_guards: Vec<(usize, l::Relation)>,
    known: Vec<(usize, bool)>,
    exact: Vec<(usize, Bits)>,
}
fn vocabulary<L: FileSource>(parser: Option<&L>) -> crate::first::Vocab {
    let kinds = parser.map_or_else(Vec::new, FileSource::kinds);
    let names: Vec<_> = kinds.into_iter().map(l::NameText).collect();
    reader_run::pairs::vocabulary(&names, parser.map_or_else(|| l::FieldNames(Vec::new()), FileSource::fields))
}

impl Checks {
    fn from_roots<L: FileSource>(
        parser: Option<&L>, vocab: &crate::first::Vocab, programs: &[Result<Program>], program_of: &[usize],
        mut roots: Vec<Vec<usize>>, mut domains: Vec<Vec<(usize, usize)>>,
    ) -> Self {
        let mut steps = vec![BTreeSet::new(); programs.len()];
        let mut lefts = vec![Vec::new(); roots.len()];
        for (binding, outputs) in roots.iter().enumerate() {
            let id = program_of[binding];
            let wanted = &mut steps[id];
            wanted.extend(outputs.iter().copied());
            wanted.extend(domains[binding].iter().map(|&(_, d)| d));
            if let Ok(program) = &programs[id]
                && outputs.iter().copied().eq(program.plan.emits.iter().map(|emit| emit.1))
                && let Some(left) = outputs.iter().map(|&step| match &program.typed.steps[step] {
                    FinishStep::Antijoin(join) => Some(join.left.0),
                    _ => None,
                }).collect::<Option<Vec<_>>>()
            {
                wanted.extend(left.iter().copied());
                lefts[binding] = left;
            }
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
            for step in outputs.iter_mut().chain(&mut lefts[binding]) {
                *step = indices[&(program_of[binding], *step)];
            }
        }
        let mut language = super::reader_declaration(parser);
        language.fact_family = reader_run::fact_family;
        let numbered: &[l::Relation] = parser.map_or(&[], FileSource::numbered_projections);
        let mut supplied = Vec::new();
        for name in bases {
            let Some(rel) = relations::service::relation_named(NameRef(name)) else {
                continue;
            };
            if catalog::service::scope_of(rel) != catalog::types::RowScope::File
                || relations::service::class_of(rel) == relations::types::Class::Engine
                || numbered.contains(&rel)
            {
                continue;
            }
            if parser.is_some_and(|p| p.expands(rel))
                || catalog::service::fact_form(rel) != relations::types::FactForm::NodeTable
            {
                supplied.push(name.to_owned());
                if parser.is_none_or(|p| p.available(rel)) {
                    language.facts.push(name.to_owned());
                    language.file_sets.push(name.to_owned());
                }
            }
        }
        language.text_kinds = (0..vocab.kind_names.len())
            .map(|k| u16::try_from(k).expect("kind code"))
            .collect();
        let program = pair_summary::prepare_plans(
            &plans,
            &targets,
            vocab,
            &language,
            &supplied,
            Some(crate::finish::host::CURRENT_FILE),
            true,
        );
        // All requested outputs must be impossible to discharge the binding.
        // One unconditional output makes testing its siblings unnecessary
        // unless the binding can be answered exactly.
        let unconditional = |&root: &usize|
            matches!(program.needs[program.roots[root].1], pair_summary::Need::Any);
        for left in &mut lefts {
            if left.iter().any(unconditional) {
                left.clear();
            }
        }
        let exact_roots: Vec<_> = roots.iter().map(|outputs| {
            let count = outputs.iter().filter(|root| program.exact.binary_search_by_key(*root, |(index, _)| *index).is_ok()).count();
            (count != 0, count != 0 && count == outputs.len())
        }).collect();
        let roots = roots.into_iter().enumerate().map(|(binding, outputs)| {
            let exact = exact_roots[binding].1;
            (!outputs.iter().any(unconditional) || exact || outputs.len() > 1
                || !domains[binding].is_empty() || !lefts[binding].is_empty())
                .then_some(outputs)
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
            exact_roots,
            lefts,
            domains,
            row_guards,
            known: Vec::new(),
            exact: Vec::new(),
            selected: vec![0; live.len().div_ceil(64)],
            locatable: vec![0; live.len().div_ceil(64)],
            atoms: Vec::new(),
            live,
            words: l::facts::file_words(&l::facts::base_words(Default::default(), &[])),
        }
    }
    pub(super) fn exclude(
        &mut self,
        positions: &[usize],
        active: &mut [(usize, usize, bool)],
        bound: &mut super::Bindings<'_>,
        run: &mut Run,
        tree: &l::NodeTable,
        text: Option<&[u32]>,
        distinct: &std::cell::RefCell<crate::pair_checks::FileSymbols>,
        literal_positions: &std::cell::RefCell<crate::pair_checks::LiteralPositions>,
        patterns: crate::pair_checks::Predicates<'_>,
        answers: &mut AtomAnswers,
        bytes: &[u8],
        path: &str,
        id: FileId,
        symbols: &l::Symbols,
        local_words: Option<&mut l::FileWords>,
        lazy: Option<&reader_run::packet::Lazy<'_, '_>>,
        tables: &mut crate::pair_checks::WitnessTables,
        requested: &[l::Relation],
        packet: &reader_run::packet::Packet,
        unknown: &HashMap<u32, u32>,
        cuts: &[Option<Arc<crate::finish::partition::Partition>>],
        partials: &[ryo_values::domain::stored_facts::types::ReadInPart],
        layouts: &[Option<super::project::Layout>],
        witnesses: &mut [usize; 4],
    ) {
        self.live.fill(false);
        self.selected.fill(0);
        self.locatable.fill(0);
        self.exact.clear();
        let mut any = false;
        let can_locate = text.is_some() && partials.is_empty();
        let mut locate = false;
        for &(binding, _, excluded) in active.iter() {
            let index = positions[binding];
            if !excluded && let Some(roots) = &self.roots[index] {
                let unpartitioned = cuts[binding].is_none();
                any |= !roots.is_empty();
                locate |= can_locate && unpartitioned && !roots.is_empty();
                for &root in roots {
                    if unpartitioned {
                        self.locatable[root / 64] |= 1 << (root % 64);
                    }
                    self.selected[root / 64] |= 1 << (root % 64);
                }
                for &(_, root) in &self.domains[index] {
                    self.selected[root / 64] |= 1 << (root % 64);
                }
                if unpartitioned {
                    any |= !self.lefts[index].is_empty();
                    for &root in &self.lefts[index] {
                        self.selected[root / 64] |= 1 << (root % 64);
                    }
                }
            }
        }
        if !any || { static S: std::sync::OnceLock<bool> = std::sync::OnceLock::new(); *S.get_or_init(|| crate::finish::off_name("pair-witness")) } {
            crate::pair_checks::each(&self.selected, |root| self.live[root as usize] = true);
        } else {
            self.known.clear();
            for &(need, relation) in &self.row_guards {
                if !partials.iter().any(|p| p.relation == relation)
                    && let Some(exists) = packet.has_rows(relation)
                {
                    self.known.push((need, exists));
                }
            }
            // Numbered cells already use the supplier's file dictionary. New terms
            // are appended without renumbering any word the provider lent us.
            let numbered_end = local_words.as_deref().map(l::facts::words_end);
            let words = local_words.unwrap_or_else(|| {
                l::facts::clear_words(&mut self.words);
                &mut self.words
            });
            let column = |source, index, predicate, pattern: &pair_summary::regexes::Matcher, words: &l::FileWords| {
                let FactSource::Catalog(rel) = source else { return None };
                if partials.iter().any(|p| p.relation == rel) { return None; }
                patterns.with_predicate(predicate, pattern, &run.texts, |test| {
                    packet.any_symbol(rel, index, symbols, bytes, numbered_end.map(|end| (words, end)), |text, word| {
                        if let Some(text) = text {
                            let subject = text.0.as_bytes();
                            return match run.ids.get(subject) {
                                Some(&id) => test(id),
                                None => pattern.is_match(subject),
                            };
                        }
                        unknown.get(&word.0).copied().or(run.absent_word).is_none_or(|id| test(id))
                    })
                })
            };
            let atoms = (text.is_some() && !self.atoms.is_empty()).then_some(SharedAtoms {
                slots: &self.atoms,
                answers,
            });
            self.program.selected_columns(
                tree,
                bytes,
                path.as_bytes(),
                text.map(|ids| SyntaxText {
                    ids,
                    symbols: &run.texts,
                    dictionary: &run.ids,
                    positions: literal_positions,
                    distinct: Some(distinct),
                    patterns: Some(patterns),
                    column: Some(&column),
                }),
                crate::pair_checks::ReservedWords { names: unknown, texts: &run.texts },
                tables,
                words,
                |source, words, out| {
                    let FactSource::Catalog(rel) = source else {
                        return false;
                    };
                    // The file's own host relation, as the former loop's `host_columns::fill`.
                    if rel == l::Relation::FilesystemExecutable {
                        return !partials.iter().any(|p| p.relation == rel) && host_executable(run, id, out);
                    }
                    // The project layout answers for its own file, as the former loop's `identity` did.
                    if let Some((layout, part)) = layouts.iter().flatten()
                        .find_map(|layout| layout.relations.iter().position(|r| *r == rel).map(|part| (layout, part)))
                    {
                        return layout.witness_columns(part, path, id, words, out);
                    }
                    requested.contains(&rel)
                        && !partials.iter().any(|p| p.relation == rel)
                        && packet.columns(rel, symbols, bytes, id, numbered_end, unknown, words, lazy, out)
                },
                &mut self.live,
                &self.known,
                &self.selected,
                atoms,
                locate.then_some(&mut self.exact),
            );
            self.exact.retain(|(root, _)| self.locatable[root / 64] & (1 << (root % 64)) != 0);
        }
        for (binding, _, excluded) in active {
            if *excluded {
                continue;
            }
            let index = positions[*binding];
            let Some(roots) = &self.roots[index] else { continue };
            let cut = cuts[*binding].as_deref();
            // Complete locations can go straight to delivery.
            if cut.is_none() && self.exact_roots[index].1 && self.exact_locations(index).is_some() {
                continue;
            }
            let b = bound[binding].as_mut().unwrap();
            let full_absent = roots.iter().any(|&r| !self.live[r])
                || self.domains[index].iter().any(|&(_, d)| !self.live[d]);
            let left = &self.lefts[index];
            let left_absent = cut.is_none() && !left.is_empty() && left.iter().all(|&r| !self.live[r]);
            if !left_absent && !full_absent {
                continue;
            }
            witnesses[1] += 1;
            if partials.iter().any(|p| b.facts.contains(&p.relation)) {
                witnesses[2] += 1;
                continue;
            }
            // As in the former file loop, an absent witness removes the pair.
            if left_absent {
                *excluded = true;
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
                for &(i, d) in &self.domains[index] {
                    absent[i] |= !self.live[d]
                        && b.projections
                            .get(&cut.inputs[i])
                            .is_some_and(|p| p.needs_domain());
                }
                *excluded = absent.iter().all(|&v| v);
            }
        }
    }

    /// The catalog relations this screen asks about before it loads any column: the symbol predicates. Every other
    /// relation it reads, a row guard included, is loaded, and so filled, only when a witness needs it.
    pub(super) fn asked_first(&self) -> Vec<l::Relation> {
        let mut asked = Vec::new();
        for domain in &self.program.domains {
            if let pair_summary::Domain::Matching(slot, column, _) = domain {
                let fact = &self.program.facts[*slot];
                if let FactSource::Catalog(relation) = fact.source
                    && fact.kinds[*column] == l::CellKind::Symbol
                {
                    asked.push(relation);
                }
            }
        }
        asked.sort_by_key(|relation| relations::service::number_of(*relation).0);
        asked.dedup();
        asked
    }

    /// Borrow the proof mask and the declared output shape for every emit.
    pub(super) fn exact_locations(
        &self, binding: usize,
    ) -> Option<impl Iterator<Item = (&[LocationColumn], &[u64])>> {
        let roots = self.roots[binding].as_deref()?;
        let lookup = |root: &usize| {
            self.exact.binary_search_by_key(root, |(index, _)| *index)
                .ok()
                .map(|index| &self.exact[index].1)
        };
        if roots.is_empty() || roots.iter().any(|root| lookup(root).is_none()) {
            return None;
        }
        Some(roots.iter().map(move |root| {
            let certificate = self.program.exact.binary_search_by_key(root, |(index, _)| *index)
                .expect("exact mask has a certificate");
            (self.program.exact[certificate].1.columns.as_ref(),
             lookup(root).expect("all outputs have masks").as_slice())
        }))
    }
}

/// The executable-permission row of this file: the input edge's supplied permission, else the file system's.
fn host_executable(run: &Run, id: FileId, out: &mut l::FilledColumns) -> bool {
    use ryo_values::domain::files::types::ExecutablePermission::{PosixExecutable, PosixNotExecutable};
    if let Some((file, permission)) = run.supplied_permission.filter(|&(file, _)| file == id.0) {
        if !matches!(permission, PosixExecutable | PosixNotExecutable) { return false; }
        out.0 = vec![
            l::FilledColumn(vec![l::CellWord(u64::from(file))]),
            l::FilledColumn(vec![l::CellWord(u64::from(permission == PosixExecutable))]),
        ];
        return true;
    }
    crate::host_columns::fill(l::Relation::FilesystemExecutable, Some(run), id.0 as usize, out).unwrap_or(false)
}

fn domain_steps(cut: Option<&crate::finish::partition::Partition>) -> Vec<(usize, usize)> {
    // A source with one file-local origin step is absent where that step is.
    cut.into_iter().flat_map(|c| c.inputs.iter().enumerate()
        .filter_map(|(i, n)| {
            let nodes = c.domains.get(n).map(|(_, nodes)| nodes).or_else(|| c.reach.get(n))?;
            let mut steps = Vec::new();
            nodes.steps(&mut steps);
            steps.dedup();
            (steps.len() == 1).then(|| (i, steps[0]))
        })).collect()
}
fn target_steps(program: &Program, cut: Option<&crate::finish::partition::Partition>) -> Vec<usize> {
    match cut {
        Some(cut) => cut.inputs.clone(),
        None => program.plan.emits.iter().map(|emit| emit.1).collect(),
    }
}

/// Each binding has immutable output steps. A visit prepares the union of the
/// admitted shapes once, then uses the same roots in every file of that visit.
#[derive(Default)]
pub(super) struct Retained {
    vocabulary: Option<crate::first::Vocab>,
    shapes: HashMap<(usize, Vec<usize>, Vec<(usize, usize)>), usize>,
    bindings: Vec<Option<usize>>,
    sets: HashMap<Vec<usize>, usize>,
    programs: Vec<Checks>,
    atoms: atoms::Atoms,
}

impl Retained {
    pub(super) fn merged<L: FileSource>(
        &mut self, parser: Option<&L>, programs: &[Result<Program>], program_of: &[usize],
        needed: &[bool], cuts: &[Option<Arc<crate::finish::partition::Partition>>],
    ) -> (usize, &mut Checks, Vec<usize>) {
        self.bindings.resize(program_of.len(), None);
        let mut members = std::collections::BTreeMap::new();
        let mut positions = vec![usize::MAX; needed.len()];
        for (binding, &admitted) in needed.iter().enumerate() {
            if !admitted { continue; }
            let program_id = program_of[binding];
            let Ok(program) = &programs[program_id] else { continue };
            let shape = *self.bindings[binding].get_or_insert_with(|| {
                let cut = cuts[binding].as_deref();
                let key = (program_id, target_steps(program, cut), domain_steps(cut));
                let next = self.shapes.len();
                *self.shapes.entry(key).or_insert(next)
            });
            positions[binding] = shape;
            members.insert(shape, binding);
        }
        let key: Vec<_> = members.keys().copied().collect();
        let index = *self.sets.entry(key).or_insert_with(|| {
            let mut ids = Vec::with_capacity(members.len());
            let mut roots = Vec::with_capacity(members.len());
            let mut domains = Vec::with_capacity(members.len());
            for &binding in members.values() {
                let id = program_of[binding];
                let program = programs[id].as_ref().expect("admitted program");
                let cut = cuts[binding].as_deref();
                ids.push(id);
                roots.push(target_steps(program, cut));
                domains.push(domain_steps(cut));
            }
            let vocab = self.vocabulary.get_or_insert_with(|| vocabulary(parser));
            let mut screen = Checks::from_roots(parser, vocab, programs, &ids, roots, domains);
            screen.atoms = self.atoms.bind(&screen.program);
            let index = self.programs.len();
            self.programs.push(screen);
            index
        });
        for (pos, slot) in members.values_mut().enumerate() { *slot = pos; }
        for shape in &mut positions {
            if *shape != usize::MAX { *shape = members[shape]; }
        }
        (index, &mut self.programs[index], positions)
    }
}
