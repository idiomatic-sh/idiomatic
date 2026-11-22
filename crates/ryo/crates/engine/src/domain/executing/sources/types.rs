//! The types of the `sources` part: the loop's frame as what answers a need's leaves.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::types::{ConstantNumber, NeedSite, RelationNumber, Short, WordSlot};
use crate::domain::executing::measuring::service::{
    need_fill_begun, need_fill_ended, need_leaf, need_nested,
};
use crate::domain::executing::measuring::sites::service::{kind_named, row_named};
use crate::domain::executing::presence::service::{consulted, node_row};
use crate::domain::executing::types::Asker;
use crate::domain::indexes::reading::service::cell_at;
use crate::domain::made_tables::reading::service::view_of;
use crate::domain::made_tables::sources::service::{made, scanned, table_of};
use crate::domain::made_tables::types::ReadSource;
use crate::domain::needs::formulas::service::{node_musts, site_root};
use crate::domain::needs::formulas::types::{FormulaNode, FreeValue};
use crate::domain::needs::masks::types::{NeedTerm, RecordCost, Stretch, TermNumber};
use crate::domain::needs::service::decided;
use crate::domain::needs::types::{NeedFacts, NeedVerdict};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::constant;
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::node_table::service::kinds_present;
use ryo_relations::domain::relations::types::{Count, KindBits};

/// SPIKE: what a measuring worker's decisions visited, for the run: decisions, nodes valued from the free bits, nodes
/// asked, nodes whose musts failed. Printed by `counting::service::counted`.
pub(crate) static NEED_VISITS: [std::sync::atomic::AtomicU64; 4] = [const { std::sync::atomic::AtomicU64::new(0) }; 4];

/// SPIKE: counts one visit of kind `which` when the worker measures.
fn visit(state: &crate::domain::scratch::types::LoopState, which: usize) {
    if state.measure.is_some()
        && let Some(count) = NEED_VISITS.get(which)
    {
        let _was = count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// The loop's frame, as what a need's formula asks: its decisions are kept in the loop's state; a kind's presence is the
/// node table's; a relation's row is asked of the fill side, which fills the relation; a table's row makes the table when
/// its own need holds.
pub(crate) struct FileNeeds<'a, 's, 'f>(pub(crate) &'a mut StepFrame<'s, 'f>);

impl NeedFacts for FileNeeds<'_, '_, '_> {
    fn kept(&self, site: NeedSite) -> Option<NeedVerdict> {
        self.0
            .state
            .needs
            .get(usize::try_from(site.0).unwrap_or(usize::MAX))
            .copied()
            .flatten()
    }

    fn keep(&mut self, site: NeedSite, verdict: NeedVerdict) {
        let place = usize::try_from(site.0).unwrap_or(usize::MAX);
        if self.0.state.needs.len() <= place {
            self.0.state.needs.resize(place.saturating_add(1), None);
        }
        if let Some(slot) = self.0.state.needs.get_mut(place) {
            *slot = Some(verdict);
        }
        // A site decided is its own value from the free terms for the rest of the file: one read finds either.
        let value = match verdict {
            NeedVerdict::Holds => FreeValue::Holds,
            NeedVerdict::Empty => FreeValue::Empty,
        };
        let sites = &mut self.0.state.need_terms.sites;
        if sites.len() <= place {
            sites.resize(place.saturating_add(1), FreeValue::Unfound);
        }
        if let Some(held) = sites.get_mut(place) {
            *held = value;
        }
    }

    fn kind_present(&mut self, kind: KindBits) -> NeedVerdict {
        let present = kinds_present(self.0.fill.nodes)
            .0
            .get(usize::from(kind.0 / 64))
            .is_some_and(|word| (word.0 >> (kind.0 % 64)) & 1 == 1);
        let verdict = if present {
            NeedVerdict::Holds
        } else {
            NeedVerdict::Empty
        };
        if self.0.state.measure.is_some() {
            let named = kind_named(self.0, kind);
            need_leaf(self.0.state, || format!("{named}={verdict:?}"));
        }
        verdict
    }

    fn relation_row(
        &mut self,
        relation: RelationNumber,
        column: WordSlot,
        value: WordSlot,
    ) -> NeedVerdict {
        if self.0.state.measure.is_none() {
            return self.relation_decided(relation, column, value);
        }
        // SPIKE: a measuring worker traces the term: whether it cost nothing (a node relation's sets) or a fill.
        let free = node_row(self.0, relation, column, value).is_some();
        need_nested(self.0.state, NeedVerdict::Holds);
        let verdict = self.relation_decided(relation, column, value);
        need_nested(self.0.state, NeedVerdict::Empty);
        let named = row_named(self.0, relation, column, value);
        let cost = if free { "free" } else { "fill" };
        need_leaf(self.0.state, || format!("{cost}-{named}={verdict:?}"));
        verdict
    }

    fn masked(&mut self, site: NeedSite) -> Option<NeedVerdict> {
        let code = self.0.fill.programs.code;
        // What the terms that cost nothing say of the whole formula, read off the file's bits: most sites are decided
        // here. Only a site they leave open asks what costs, the cheapest first.
        visit(self.0.state, 0);
        let Some(root) = site_root(code, site, &mut self.0.state.need_masks) else {
            return Some(NeedVerdict::Holds);
        };
        self.free_answered();
        match self.site_free(site, root) {
            FreeValue::Empty => {
                if self.0.state.measure.is_some() {
                    need_leaf(self.0.state, || "free=Empty".to_owned());
                }
                return Some(NeedVerdict::Empty);
            }
            FreeValue::Holds => {
                if self.0.state.measure.is_some() {
                    need_leaf(self.0.state, || "free=Holds".to_owned());
                }
                return Some(NeedVerdict::Holds);
            }
            // A site met inside its own decision says nothing of itself.
            FreeValue::Deciding => return Some(NeedVerdict::Holds),
            FreeValue::Unfound | FreeValue::Open => {}
        }
        self.site_valued(site, FreeValue::Deciding);
        let verdict = self.node_decided(root);
        if self.0.state.measure.is_some() {
            need_leaf(self.0.state, || format!("asked={verdict:?}"));
        }
        Some(verdict)
    }

    fn leaf(&mut self, at: WordSlot) -> Option<NeedVerdict> {
        // A measuring worker traces each leaf by name: the walk asks it.
        if self.0.state.measure.is_some() {
            return None;
        }
        let leaves = &self.0.state.need_masks.leaves;
        let number = leaves
            .get(usize::try_from(at.0).unwrap_or(usize::MAX))
            .copied()
            .unwrap_or(0)
            .checked_sub(1)?;
        Some(self.term_holds(TermNumber(number)))
    }

    fn record_cost(&self, at: WordSlot) -> Option<NeedVerdict> {
        let costs = &self.0.state.need_masks.costs;
        match costs.get(usize::try_from(at.0).unwrap_or(usize::MAX)) {
            Some(RecordCost::Free) => Some(NeedVerdict::Empty),
            Some(RecordCost::Dear | RecordCost::Presence) => Some(NeedVerdict::Holds),
            Some(RecordCost::Unknown) | None => None,
        }
    }

    fn table_row(&mut self, table: MadeTableId) -> NeedVerdict {
        if self.0.state.measure.is_none() {
            return self.table_decided(table);
        }
        // SPIKE: a measuring worker traces the term; what is decided inside it (the table's own need, the needs its
        // fill's reads ask) is not the traced formula's.
        need_nested(self.0.state, NeedVerdict::Holds);
        let verdict = self.table_decided(table);
        need_nested(self.0.state, NeedVerdict::Empty);
        need_leaf(self.0.state, || format!("table:{}={verdict:?}", table.0));
        verdict
    }
}

impl FileNeeds<'_, '_, '_> {
    /// A site's value from the free terms alone, kept for the file.
    fn site_valued(&mut self, site: NeedSite, value: FreeValue) {
        let place = usize::try_from(site.0).unwrap_or(usize::MAX);
        let sites = &mut self.0.state.need_terms.sites;
        if sites.len() <= place {
            sites.resize(place.saturating_add(1), FreeValue::Unfound);
        }
        if let Some(held) = sites.get_mut(place) {
            *held = value;
        }
    }

    /// What a site's formula says of the file from the free terms alone (and what the file already answered): found
    /// once a file. A site decided in the file is its decision.
    fn site_free(&mut self, site: NeedSite, root: u32) -> FreeValue {
        // A site decided in the file holds its decision here (`keep`).
        let place = usize::try_from(site.0).unwrap_or(usize::MAX);
        match self.0.state.need_terms.sites.get(place).copied() {
            Some(FreeValue::Unfound) | None => {}
            Some(known) => return known,
        }
        self.site_valued(site, FreeValue::Deciding);
        let value = self.node_free(root);
        self.site_valued(site, value);
        value
    }

    /// What another site's formula says of the file from the free terms alone, compiled now when this is its first ask.
    fn site_free_named(&mut self, site: NeedSite) -> FreeValue {
        match self
            .0
            .state
            .need_terms
            .sites
            .get(usize::try_from(site.0).unwrap_or(usize::MAX))
            .copied()
        {
            Some(FreeValue::Empty) => return FreeValue::Empty,
            Some(FreeValue::Holds) => return FreeValue::Holds,
            Some(FreeValue::Open | FreeValue::Deciding) => return FreeValue::Open,
            Some(FreeValue::Unfound) | None => {}
        }
        let code = self.0.fill.programs.code;
        let root = site_root(code, site, &mut self.0.state.need_masks);
        // A site compiled now may have met free terms the file's bits do not answer yet.
        self.free_answered();
        match root {
            Some(root) => match self.site_free(site, root) {
                FreeValue::Deciding | FreeValue::Unfound => FreeValue::Open,
                value => value,
            },
            None => FreeValue::Open,
        }
    }

    /// A node of a compiled formula.
    fn node(&self, node: u32) -> FormulaNode {
        self.0
            .state
            .need_masks
            .formulas
            .nodes
            .get(usize::try_from(node).unwrap_or(usize::MAX))
            .copied()
            .unwrap_or(FormulaNode::Always)
    }

    /// The part at a place of the formulas' parts.
    fn part(&self, at: usize) -> u32 {
        self.0
            .state
            .need_masks
            .formulas
            .parts
            .get(at)
            .copied()
            .unwrap_or(u32::MAX)
    }

    /// Whether the file's free bit is set: the free terms met since the bits were brought up (a site compiled inside
    /// a decision) are asked first.
    fn free_bit(&mut self, bit: u32) -> bool {
        let place = usize::try_from(bit).unwrap_or(usize::MAX);
        if place >= self.0.state.need_terms.free_known.0 {
            self.free_answered();
        }
        self.0
            .state
            .need_terms
            .free
            .get(place / 64)
            .is_some_and(|word| word & (1_u64 << (place % 64)) != 0)
    }

    /// What a node says of the file from the free terms, the terms that cost that the file already answered, and the
    /// tables already made: `Open` when what is not known yet decides. Asks nothing that costs.
    fn node_free(&mut self, node: u32) -> FreeValue {
        visit(self.0.state, 1);
        let held = self.node(node);
        // A node of parts, or another site, is first held to the free terms it cannot hold without; a leaf is its own.
        if matches!(
            held,
            FormulaNode::All(_) | FormulaNode::Any(_) | FormulaNode::Site(_) | FormulaNode::TableNeed(_)
        ) && !self.musts_held(node)
        {
            visit(self.0.state, 3);
            return FreeValue::Empty;
        }
        let holds = |held: bool| {
            if held {
                FreeValue::Holds
            } else {
                FreeValue::Empty
            }
        };
        match held {
            FormulaNode::Always => FreeValue::Holds,
            FormulaNode::Never => FreeValue::Empty,
            FormulaNode::Free(bit) => holds(self.free_bit(bit)),
            FormulaNode::Term(term) => match self.term_known(term) {
                Some(verdict) => holds(verdict == NeedVerdict::Holds),
                None => FreeValue::Open,
            },
            FormulaNode::Table(table) => match table_of(self.0.fill.store, table) {
                Some(slot) => holds(slot.rows.count.0 > 0),
                None => FreeValue::Open,
            },
            FormulaNode::TableNeed(table) => {
                let code = self.0.fill.programs.code;
                let Some(need) = code
                    .tables
                    .get(usize::try_from(table.0).unwrap_or(usize::MAX))
                    .map(|held| held.need)
                else {
                    return FreeValue::Empty;
                };
                // As a reader of the table consults its need (`sources::service::table_needed`).
                consulted(self.0, need);
                self.site_free_named(need)
            }
            FormulaNode::Site(site) => self.site_free_named(site),
            FormulaNode::AllBits(words) => holds(self.bits_held(words, true)),
            FormulaNode::AnyBits(words) => holds(self.bits_held(words, false)),
            FormulaNode::All(parts) => {
                let mut value = FreeValue::Holds;
                for at in parts.start.0..parts.start.0.saturating_add(parts.count.0) {
                    match self.node_free(self.part(at)) {
                        FreeValue::Empty => return FreeValue::Empty,
                        FreeValue::Holds => {}
                        FreeValue::Open | FreeValue::Unfound | FreeValue::Deciding => {
                            value = FreeValue::Open;
                        }
                    }
                }
                value
            }
            FormulaNode::Any(parts) => {
                let mut value = FreeValue::Empty;
                for at in parts.start.0..parts.start.0.saturating_add(parts.count.0) {
                    match self.node_free(self.part(at)) {
                        FreeValue::Holds => return FreeValue::Holds,
                        FreeValue::Empty => {}
                        FreeValue::Open | FreeValue::Unfound | FreeValue::Deciding => {
                            value = FreeValue::Open;
                        }
                    }
                }
                value
            }
        }
    }

    /// What a node says of the file, asking what it costs: its parts in the order compiled (what costs nothing first,
    /// a filled relation's any-row term last), stopping at the first part that decides.
    fn node_decided(&mut self, node: u32) -> NeedVerdict {
        visit(self.0.state, 2);
        let held = self.node(node);
        if matches!(
            held,
            FormulaNode::All(_) | FormulaNode::Any(_) | FormulaNode::Site(_) | FormulaNode::TableNeed(_)
        ) && !self.musts_held(node)
        {
            return NeedVerdict::Empty;
        }
        match held {
            FormulaNode::Always => NeedVerdict::Holds,
            FormulaNode::Never => NeedVerdict::Empty,
            FormulaNode::Free(bit) => {
                if self.free_bit(bit) {
                    NeedVerdict::Holds
                } else {
                    NeedVerdict::Empty
                }
            }
            FormulaNode::Term(term) => self.term_holds(term),
            FormulaNode::Table(table) => self.table_row(table),
            FormulaNode::TableNeed(table) => {
                let ports = self.0.ports;
                ports.table_need(self.0, table)
            }
            FormulaNode::AllBits(words) | FormulaNode::AnyBits(words) => {
                let every = matches!(held, FormulaNode::AllBits(_));
                if self.bits_held(words, every) {
                    NeedVerdict::Holds
                } else {
                    NeedVerdict::Empty
                }
            }
            FormulaNode::Site(site) => {
                let code = self.0.fill.programs.code;
                decided(self, code, site)
            }
            FormulaNode::All(parts) => {
                for at in parts.start.0..parts.start.0.saturating_add(parts.count.0) {
                    if self.node_decided(self.part(at)) == NeedVerdict::Empty {
                        return NeedVerdict::Empty;
                    }
                }
                NeedVerdict::Holds
            }
            FormulaNode::Any(parts) => {
                for at in parts.start.0..parts.start.0.saturating_add(parts.count.0) {
                    if self.node_decided(self.part(at)) == NeedVerdict::Holds {
                        return NeedVerdict::Holds;
                    }
                }
                NeedVerdict::Empty
            }
        }
    }

    /// Whether the file's free bits hold every free term a node cannot hold without, and some term of its clause.
    #[inline]
    fn musts_held(&self, node: u32) -> bool {
        let (musts, clause) = node_musts(&self.0.state.need_masks, node);
        if musts.count.0 == 0 && clause.count.0 == 0 {
            return true;
        }
        let answers = &self.0.state.need_terms.free;
        let words = &self.0.state.need_masks.formulas.musts;
        let within =
            |stretch: Stretch| stretch.start.0..stretch.start.0.saturating_add(stretch.count.0);
        let all = words.get(within(musts)).unwrap_or_default();
        let every = all
            .iter()
            .all(|word| answers.get(word.word.0).copied().unwrap_or(0) & word.bits == word.bits);
        if !every {
            return false;
        }
        let some = words.get(within(clause)).unwrap_or_default();
        some.is_empty()
            || some
                .iter()
                .any(|word| answers.get(word.word.0).copied().unwrap_or(0) & word.bits != 0)
    }

    /// Whether the file's free bits hold every bit of the words (`every`), or some one of them.
    fn bits_held(&self, words: Stretch, every: bool) -> bool {
        let answers = &self.0.state.need_terms.free;
        let held = self
            .0
            .state
            .need_masks
            .formulas
            .bits
            .get(words.start.0..words.start.0.saturating_add(words.count.0))
            .unwrap_or_default();
        if every {
            held.iter().all(|word| {
                answers.get(word.word.0).copied().unwrap_or(0) & word.bits == word.bits
            })
        } else {
            held.iter()
                .any(|word| answers.get(word.word.0).copied().unwrap_or(0) & word.bits != 0)
        }
    }

    /// The file's answer to a term when it was asked in the file; none when it was not.
    fn term_known(&self, term: TermNumber) -> Option<NeedVerdict> {
        let place = usize::try_from(term.0).unwrap_or(usize::MAX);
        let (word, bit) = (place / 64, 1_u64 << (place % 64));
        let answers = &self.0.state.need_terms;
        let asked = answers.asked.get(word).copied().unwrap_or(0);
        if asked & bit == 0 {
            return None;
        }
        let holds = answers.holds.get(word).copied().unwrap_or(0);
        Some(if holds & bit != 0 {
            NeedVerdict::Holds
        } else {
            NeedVerdict::Empty
        })
    }

    /// Has the file's answer to every free term met so far in the file's free bits: the terms met since the bits
    /// were last brought up are asked now (all of them at the file's first decision).
    fn free_answered(&mut self) {
        let known = self.0.state.need_terms.free_known.0;
        let all = self.0.state.need_masks.free_terms.len();
        if known == all {
            return;
        }
        let words = all.div_ceil(64);
        if self.0.state.need_terms.free.len() < words {
            self.0.state.need_terms.free.resize(words, 0);
        }
        for index in known..all {
            let Some(term) = self.0.state.need_masks.free_terms.get(index).copied() else {
                continue;
            };
            if self.term_asked(term) == NeedVerdict::Holds
                && let Some(word) = self.0.state.need_terms.free.get_mut(index / 64)
            {
                *word |= 1_u64 << (index % 64);
            }
        }
        self.0.state.need_terms.free_known = Count(all);
    }

    /// Whether a term of the rule set's needs holds in the file: a free term's answer is read in the file's free
    /// bits; any other is asked the first time in a file (of a relation the language fills now) and read after.
    #[inline]
    fn term_holds(&mut self, term: TermNumber) -> NeedVerdict {
        let place = usize::try_from(term.0).unwrap_or(usize::MAX);
        let free = self
            .0
            .state
            .need_masks
            .free_places
            .get(place)
            .copied()
            .unwrap_or(0);
        if let Some(index) = usize::try_from(free).unwrap_or(0).checked_sub(1)
            && index < self.0.state.need_terms.free_known.0
        {
            let word = self
                .0
                .state
                .need_terms
                .free
                .get(index / 64)
                .copied()
                .unwrap_or(0);
            return if word & (1_u64 << (index % 64)) != 0 {
                NeedVerdict::Holds
            } else {
                NeedVerdict::Empty
            };
        }
        let (word, bit) = (place / 64, 1_u64 << (place % 64));
        let answers = &mut self.0.state.need_terms;
        if answers.asked.len() <= word {
            answers.asked.resize(word.saturating_add(1), 0);
            answers.holds.resize(word.saturating_add(1), 0);
        }
        if let (Some(asked), Some(holds)) = (answers.asked.get(word), answers.holds.get(word))
            && asked & bit != 0
        {
            return if holds & bit != 0 {
                NeedVerdict::Holds
            } else {
                NeedVerdict::Empty
            };
        }
        let verdict = self.term_asked(term);
        let answers = &mut self.0.state.need_terms;
        if let (Some(asked), Some(holds)) =
            (answers.asked.get_mut(word), answers.holds.get_mut(word))
        {
            *asked |= bit;
            if verdict == NeedVerdict::Holds {
                *holds |= bit;
            } else {
                *holds &= !bit;
            }
        }
        verdict
    }

    /// A term's answer for the file, asked now: of the node table's sets, or of a relation the language fills.
    fn term_asked(&mut self, term: TermNumber) -> NeedVerdict {
        let place = usize::try_from(term.0).unwrap_or(usize::MAX);
        match self.0.state.need_masks.terms.get(place).copied() {
            Some(NeedTerm::Kinds(list)) => {
                let code = self.0.fill.programs.code;
                let from = usize::try_from(list.0).unwrap_or(usize::MAX);
                let length = code
                    .pool
                    .get(from)
                    .map_or(0, |held| usize::try_from(held.0).unwrap_or(usize::MAX));
                let kinds = code
                    .pool
                    .get(from.saturating_add(1)..from.saturating_add(1).saturating_add(length))
                    .unwrap_or_default();
                let present = kinds_present(self.0.fill.nodes);
                let any = kinds.iter().any(|kind| {
                    present
                        .0
                        .get(usize::try_from(kind.0 / 64).unwrap_or(usize::MAX))
                        .is_some_and(|held| (held.0 >> (kind.0 % 64)) & 1 == 1)
                });
                if any {
                    NeedVerdict::Holds
                } else {
                    NeedVerdict::Empty
                }
            }
            Some(NeedTerm::Row {
                relation,
                column,
                constant,
            }) => self.relation_decided(relation, column, constant),
            None => NeedVerdict::Holds,
        }
    }

    /// Whether the relation has a row in the file, with the constant in the column when both are given.
    fn relation_decided(
        &mut self,
        relation: RelationNumber,
        column: WordSlot,
        value: WordSlot,
    ) -> NeedVerdict {
        let frame = &mut *self.0;
        if let Some(decided) = node_row(frame, relation, column, value) {
            return decided;
        }
        let source = ReadSource::Filled(relation);
        // SPIKE: a count under RYO_X_FILLS tells this reach of the relation, a need's, from a reader's.
        ryo_language::domain::fillers::service::fill_need_asking(true);
        let asked = scanned(
            frame.fill,
            &mut Asker {
                state: &mut *frame.state,
                found: &mut *frame.found,
                ports: frame.ports,
            },
            source,
        );
        ryo_language::domain::fillers::service::fill_need_asking(false);
        // A relation the fill side cannot answer for says nothing: the unit is evaluated, and its own read faults.
        let Ok(whole) = asked else {
            return NeedVerdict::Holds;
        };
        let rows = whole.rows.start.0..whole.rows.end.0;
        let holds = if column == NO_SLOT || value == NO_SLOT {
            !rows.is_empty()
        } else {
            let wanted = constant(frame, ConstantNumber(value.0));
            let place = Short(u16::try_from(column.0).unwrap_or(u16::MAX));
            // The column is asked once for the need, and its cells are read where they lie.
            let held = view_of(frame.fill.store, source, place);
            rows.clone().any(|row| cell_at(held, Count(row)) == wanted)
        };
        if holds {
            NeedVerdict::Holds
        } else {
            NeedVerdict::Empty
        }
    }

    /// Whether the table has a row in the file: made when its own need holds.
    fn table_decided(&mut self, table: MadeTableId) -> NeedVerdict {
        let frame = &mut *self.0;
        let ports = frame.ports;
        if ports.table_need(frame, table) == NeedVerdict::Empty {
            return NeedVerdict::Empty;
        }
        // SPIKE: a measuring worker books the fill this need starts (the table is not made yet for the file).
        let begun = (frame.state.measure.is_some() && table_of(frame.fill.store, table).is_none())
            .then(need_fill_begun);
        let made_now = made(
            frame.fill,
            &mut Asker {
                state: &mut *frame.state,
                found: &mut *frame.found,
                ports: frame.ports,
            },
            table,
        );
        let rows = table_of(frame.fill.store, table).map_or(0, |slot| slot.rows.count.0);
        if let Some(begun) = begun {
            need_fill_ended(frame.state, table, begun, Count(rows));
        }
        if made_now.is_ok() && rows > 0 {
            NeedVerdict::Holds
        } else {
            NeedVerdict::Empty
        }
    }
}
