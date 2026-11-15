//! The types of the `predicates` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::types::PreparedPredicates;
use ryo_execution_domain::domain::operator::contract::views::types::PredicateProgram;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use {
    ryo_operators::domain::table_texts::types::Texts, ryo_values::domain::files::types::SourceView,
};

pub(crate) struct BoundPredicate<'a> {
    pub program: &'a PreparedPredicates,
    pub bindings: &'a [ParameterBinding],
    pub source: SourceView<'a>,
    pub paths: &'a [FilePath],
    pub arenas: &'a [ArenaBytes],
    pub prepared: Option<&'a PreparedMatchers>,
    pub state: &'a mut ryo_operators::domain::table_scratch::types::LazyScratch<
        ryo_operators::domain::table_scratch::types::SelectScratch,
    >,
    pub memo: &'a mut Vec<ryo_operators::domain::table_scratch::types::Memo>,
    pub names: &'a mut Option<ryo_operators::domain::comparing::types::NameCount>,
    /// The file's profile, which counts the predicates this binding decides.
    #[cfg(feature = "diagnostic-row-progress")]
    pub progress: &'a ryo_execution_store::application::file_dag::route_counts::profile::service::RouteProfile,
}

impl PredicateProgram for BoundPredicate<'_> {
    fn holds(
        &mut self,
        row: &[Value],
        symbols: &Symbols,
    ) -> Result<PredicateOutcome, OperatingError> {
        #[cfg(feature = "diagnostic-row-progress")]
        ryo_execution_domain::domain::operating::types::ProgressObserver::observe(
            self.progress,
            ryo_execution_domain::domain::operating::types::ProgressCounter::PredicateEvaluations,
            ryo_execution_domain::domain::operating::types::ProgressAmount(1),
        );
        if !self.program.comparisons.is_empty() && row.len() != self.program.kinds.len() {
            return Err(OperatingError::ShapeMismatch);
        }
        let texts = Texts {
            symbols,
            source: self.source,
            paths: self.paths,
            arenas: self.arenas,
        };
        if self.program.comparisons.is_empty() {
            return Ok(PredicateOutcome::Holds);
        }
        let names = *self.names.get_or_insert_with(|| {
            let count = ryo_operators::domain::comparing::types::NameCount(
                ryo_language::domain::names::service::name_count(symbols).0,
            );
            self.memo.clear();
            self.memo.resize(
                self.program.comparisons.len() * count.0,
                ryo_operators::domain::table_scratch::types::Memo::Unknown,
            );
            count
        });
        ryo_operators::domain::comparing::service::holds_bound(
            &self.program.comparisons,
            row,
            ryo_operators::domain::comparing::types::RowShape {
                kinds: &self.program.kinds,
                names,
            },
            &texts,
            self.bindings,
            self.prepared,
            ryo_operators::domain::patterns::types::PatternMatcher(
                &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
            ),
            ryo_operators::domain::comparing::types::DecisionWorkspace {
                select: &mut *self.state,
                memo: &mut *self.memo,
            },
        )
        .map_err(ryo_execution_domain::domain::operating::errors::OperatingError::from)
    }
}
