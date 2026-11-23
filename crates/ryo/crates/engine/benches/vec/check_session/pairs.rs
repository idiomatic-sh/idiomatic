//! Bind the same certified pair program once to each reader's vocabulary.
//! Coverage refusals belong to the request and take precedence over an empty witness.
use super::{Answer, Bound, Clock, Program, Result, Run, Source, reader, reader_run};
use crate::{pair_checks::SyntaxText, pair_summary};
use reader_run::{
    packet::Packet,
    pairs::{Decisions, vocabulary},
};
use ryo::languages::{self as l, contract::ParseEngine};
use ryo_relations::domain::{catalog, labels::types::NameRef, relations};
use std::collections::{BTreeSet, HashMap};

pub(super) struct Checks {
    decisions: Decisions,
    root: Vec<Option<usize>>,
    wanted: Vec<usize>,
}
impl Checks {
    pub(super) fn new(
        parser: Option<&dyn ParseEngine>,
        programs: &[Result<Program>],
        program_of: &[usize],
        sources: &[Source],
        selected: &[usize],
    ) -> Self {
        let mut selected_programs = vec![false; programs.len()];
        for &index in selected {
            for &binding in sources[index].bindings.keys() {
                selected_programs[program_of[binding]] = true;
            }
        }
        let mut plans = Vec::new();
        let mut bases = BTreeSet::new();
        let mut root = vec![None; programs.len()];
        for (i, selected) in selected_programs.into_iter().enumerate() {
            if selected && let Ok(program) = &programs[i] {
                root[i] = Some(plans.len());
                plans.push(program.typed.as_ref());
                bases.extend(program.bases.iter().map(String::as_str));
            }
        }
        let mut language = crate::declaration();
        language.fact_family = reader_run::fact_family;
        let mut supplied_rows = Vec::new();
        for name in bases {
            let Some(relation) = relations::service::relation_named(NameRef(name)) else {
                continue;
            };
            if catalog::service::scope_of(relation) != catalog::types::RowScope::File
                || relations::service::class_of(relation) == relations::types::Class::Engine
            {
                continue;
            }
            let expanded = parser.is_some_and(|p| p.includes_expansions(relation));
            if expanded
                || catalog::service::fact_form(relation) != relations::types::FactForm::NodeTable
            {
                supplied_rows.push(name.to_owned());
                // An unsupported relation stays unknown. A declared absent construct is a complete empty packet.
                if !parser
                    .is_some_and(|p| p.absence_of(relation) == Some(l::AbsenceCause::NotYetRead))
                {
                    language.facts.push(name.to_owned());
                }
            }
        }
        let kinds = parser.map(|p| p.kind_names());
        let vocab = vocabulary(
            kinds.as_ref().map_or(&[], |v| &v.0),
            parser.map_or_else(|| l::FieldNames(Vec::new()), |p| p.stable_fields()),
        );
        language.text_kinds = (0..vocab.kind_names.len())
            .map(|k| k.try_into().expect("kind code"))
            .collect();
        let program = pair_summary::prepare_plans(
            &plans,
            &vocab,
            &language,
            &supplied_rows,
            Some(crate::finish::host::CURRENT_FILE),
        );
        let count = program.roots.len();
        Self {
            decisions: Decisions::with_roots(program, (0..count).collect()),
            root,
            wanted: Vec::new(),
        }
    }

    pub(super) fn filter(
        &mut self,
        active: &mut Vec<(usize, usize)>,
        program_of: &[usize],
        bound: &mut HashMap<usize, Result<Bound<'_>>>,
        answers: &mut [Answer],
        run: &mut Run,
        provider: &reader::ReaderFacts<'_>,
        tree: &l::NodeTable,
        bytes: &[u8],
        source: &Source,
        elements: &mut l::Elements,
        symbols: &mut l::Symbols,
        id: l::FileId,
        facts: &[l::Relation],
        packet: &mut Packet,
        clocks: &mut Clock,
    ) {
        self.wanted.clear();
        self.wanted
            .extend(active.iter().filter_map(|(b, _)| self.root[program_of[*b]]));
        self.wanted.sort_unstable();
        self.wanted.dedup();
        if self.wanted.is_empty() {
            return;
        }
        self.decisions.filter(
            &mut self.wanted,
            tree,
            SyntaxText {
                ids: &provider.text,
                symbols: &run.texts,
            },
            bytes,
            &source.path,
            elements,
            symbols,
            id,
            facts,
            packet,
            |_, _, _| unreachable!("session indexed the complete requested packet"),
        );
        active.retain(|&(binding, slot)| {
            if self.root[program_of[binding]].is_none_or(|r| self.decisions.kept(r)) {
                return true;
            }
            let b = bound.get_mut(&binding).unwrap().as_mut().unwrap();
            if packet
                .rows
                .partials
                .iter()
                .any(|(r, _, _)| b.facts.contains(r))
                || packet
                    .rows
                    .partials_at
                    .iter()
                    .any(|(r, _, _)| b.facts.contains(r))
            {
                return true;
            }
            if !*b.screenable.get_or_insert_with(|| {
                crate::finish::session::certificate::certified_empty(&mut b.exec, run, &b.hosted)
            }) {
                return true;
            }
            answers[slot] = Ok(Vec::new());
            clocks.skipped += 1;
            false
        });
    }
}
