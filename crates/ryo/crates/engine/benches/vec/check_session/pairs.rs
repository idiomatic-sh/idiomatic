//! Bind typed pair witnesses to the source adapter's actual row populations.
use super::{Bound, FileId, FileSource, Program, Result, Run, Source, l, reader_run};
use crate::{
    pair_checks::SyntaxText,
    pair_summary::{self, Source as FactSource},
};
use ryo_relations::domain::{catalog, labels::types::NameRef, relations};
use std::collections::{BTreeSet, HashMap};

pub(super) struct Checks {
    program: pair_summary::Program,
    roots: Vec<Vec<usize>>,
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
        sources: &[Source],
        selected: &[usize],
        cuts: &[Option<std::rc::Rc<crate::finish::partition::Partition>>],
    ) -> Self {
        let mut needed = vec![false; program_of.len()];
        for &source in selected {
            for &binding in sources[source].bindings.keys() {
                needed[binding] = true;
            }
        }
        let mut steps = vec![BTreeSet::new(); programs.len()];
        let mut roots = vec![Vec::new(); program_of.len()];
        for (binding, outputs) in roots.iter_mut().enumerate().filter(|(b, _)| needed[*b]) {
            let id = program_of[binding];
            let Ok(program) = &programs[id] else { continue };
            if let Some(partition) = &cuts[binding] {
                outputs.extend_from_slice(&partition.inputs);
            } else {
                outputs.extend(program.plan.emits.iter().map(|e| e.1));
            }
            steps[id].extend(outputs.iter().copied());
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
            row_guards,
            known: Vec::new(),
            live,
            patterns: Default::default(),
            words: l::facts::file_words(&l::facts::base_words(Default::default(), &[])),
        }
    }
    pub(super) fn exclude(
        &mut self,
        active: &mut [(usize, usize, bool)],
        bound: &mut HashMap<usize, Result<Bound<'_>>>,
        run: &mut Run,
        tree: &l::NodeTable,
        text: &[u32],
        bytes: &[u8],
        path: &str,
        id: FileId,
        symbols: &l::Symbols,
        requested: &[l::Relation],
        packet: &reader_run::packet::Packet,
        cuts: &[Option<std::rc::Rc<crate::finish::partition::Partition>>],
        partials: &[ryo_values::domain::stored_facts::types::ReadInPart],
    ) {
        self.live.fill(false);
        let mut any = false;
        for &(binding, _, excluded) in active.iter() {
            if !excluded {
                for &root in &self.roots[binding] {
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
            self.program.file_columns(
                tree,
                bytes,
                Some(path.as_bytes()),
                Some(SyntaxText {
                    ids: text,
                    symbols: &run.texts,
                    patterns: Some(&self.patterns),
                }),
                &mut self.words,
                |source, words, out| {
                    let FactSource::Catalog(rel) = source else {
                        return false;
                    };
                    requested.contains(&rel)
                        && !partials.iter().any(|p| p.relation == rel)
                        && packet.columns(rel, symbols, bytes, id, text.len(), words, out)
                },
                &mut self.live,
                None,
                &self.known,
            );
        }
        for (binding, _, excluded) in active {
            if !*excluded && self.roots[*binding].iter().all(|&r| !self.live[r]) {
                let b = bound.get_mut(binding).unwrap().as_mut().unwrap();
                if partials.iter().any(|p| b.facts.contains(&p.relation)) {
                    continue;
                }
                *excluded = *b.screenable.get_or_insert_with(|| {
                    crate::finish::session::certificate::certified_empty(
                        &mut b.exec,
                        run,
                        &b.hosted,
                        cuts[*binding].as_deref(),
                    )
                });
            }
        }
    }
}
