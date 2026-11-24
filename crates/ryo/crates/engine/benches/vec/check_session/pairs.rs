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
    roots: Vec<Option<usize>>,
    live: Vec<bool>,
    patterns: crate::pair_checks::SymbolMatches,
    words: l::FileWords,
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
        let mut needed = vec![false; programs.len()];
        for &source in selected {
            for &binding in sources[source].bindings.keys() {
                if cuts[binding].is_none() {
                    needed[program_of[binding]] = true;
                }
            }
        }
        let mut roots = vec![None; programs.len()];
        let mut plans = Vec::new();
        let mut bases = BTreeSet::new();
        for (id, program) in programs.iter().enumerate() {
            if needed[id]
                && let Ok(program) = program
            {
                roots[id] = Some(plans.len());
                plans.push(program.typed.as_ref());
                bases.extend(program.bases.iter().map(String::as_str));
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
            &vocab,
            &language,
            &supplied,
            Some(crate::finish::host::CURRENT_FILE),
            true,
        );
        let live = vec![false; program.roots.len()];
        Self {
            program,
            roots,
            live,
            patterns: Default::default(),
            words: l::facts::file_words(&l::facts::base_words(Default::default(), &[])),
        }
    }
    pub(super) fn exclude(
        &mut self,
        active: &mut [(usize, usize, bool)],
        program_of: &[usize],
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
    ) {
        let root = |binding: usize| {
            if cuts[binding].is_none() {
                self.roots[program_of[binding]]
            } else {
                None
            }
        };
        self.live.fill(false);
        let mut any = false;
        for &(b, _, excluded) in active.iter() {
            if !excluded && let Some(r) = root(b) {
                self.live[r] = true;
                any = true;
            }
        }
        if !any {
            return;
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
                    && packet.columns(rel, symbols, bytes, id, text.len(), words, out)
            },
            &mut self.live,
            None,
        );
        for (binding, _, excluded) in active {
            if !*excluded && root(*binding).is_some_and(|r| !self.live[r]) {
                let b = bound.get_mut(binding).unwrap().as_mut().unwrap();
                *excluded = *b.screenable.get_or_insert_with(|| {
                    crate::finish::session::certificate::certified_empty(
                        &mut b.exec,
                        run,
                        &b.hosted,
                    )
                });
            }
        }
    }
}
