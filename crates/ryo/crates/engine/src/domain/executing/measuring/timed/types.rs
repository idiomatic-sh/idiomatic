//! The types of the `timed` part: the facts a measuring scan reads the language through, which time its work apart from the
//! unit that asks for it.

use super::service::{provider_spent, thread_time};
use crate::domain::counting::types::{FillClock, ProviderWork};
use ryo_language::domain::facts::types::{
    FactAnswer, FactSource, FileWords, FilledColumns, Wanted,
};
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_relations::domain::relations::types::{Count, Relation};

/// The language's facts as a measuring scan reads them: each fill and each rendering is asked of the language's own and timed,
/// and its time kept on the clock apart from the unit that asked for it. A plain scan reads the language's own.
pub(crate) struct TimedFacts<'a> {
    /// The language's own.
    pub(crate) facts: &'a mut dyn FactSource,
    /// The worker's clock of the language's work.
    pub(crate) clock: &'a FillClock,
}

impl FactSource for TimedFacts<'_> {
    fn fill(
        &mut self,
        relation: Relation,
        wanted: Wanted,
        words: &mut FileWords,
        out: &mut FilledColumns,
    ) -> FactAnswer {
        let rows = |out: &FilledColumns| out.0.first().map_or(0, |column| column.0.len());
        let before = rows(out);
        let begun = thread_time();
        let answer = self.facts.fill(relation, wanted, words, out);
        let written = Count(rows(out).saturating_sub(before));
        provider_spent(self.clock, ProviderWork::Relation(relation), written, begun);
        answer
    }

    fn render(&mut self, request: RenderRequest) -> RenderOutcome {
        let begun = thread_time();
        let outcome = self.facts.render(request);
        provider_spent(self.clock, ProviderWork::Render, Count(0), begun);
        outcome
    }
}
