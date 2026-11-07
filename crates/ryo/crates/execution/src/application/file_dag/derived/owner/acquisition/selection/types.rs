//! Values of owner-bound selector acquisition: the predicate under evaluation and its bounds.
use crate::domain::{
    comparing::service::holds_bound,
    operating::{
        errors::OperatingError,
        types::{Memo, NameCount},
    },
    operator::contract::views::PredicateProgram,
    table_model::types::Verdict,
    table_scratch::types::SelectScratch,
    table_texts::types::Texts,
};
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{CellKind, Relation, Value};
use ryo_values::domain::{
    files::types::SourceView, plan_parts::types::PlanComparison,
    query_bindings::types::ParameterBinding, stored_facts::types::ArenaBytes,
};

/// The index of one selector group within the request's prepared selection views.
#[derive(Clone, Copy)]
pub(in crate::application::file_dag::derived::owner) struct SelectorGroup(pub usize);

/// A row count a shared position list must stay within.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::derived::owner) struct RowBound(pub usize);

/// The count of rows the mandatory predicate visited.
#[derive(Clone, Copy)]
pub(in crate::application::file_dag::derived::owner) struct Visits(pub u64);

/// The stored relation a build reads, when the build is for a stored relation.
pub(in crate::application::file_dag) struct StoredColumns {
    pub relation: Relation,
}

/// Borrowed comparison state that counts actual visits around the mandatory predicate item.
pub(in crate::application::file_dag::derived::owner) struct PredicateDecision<'a> {
    pub comparisons: &'a [PlanComparison],
    pub kinds: &'a [CellKind],
    pub names: NameCount,
    pub source: SourceView<'a>,
    pub paths: &'a [FilePath],
    pub arenas: &'a [ArenaBytes],
    pub bindings: &'a [ParameterBinding],
    pub prepared: Option<&'a crate::domain::matching::types::prepared::value::PreparedMatchers>,
    pub select: &'a mut SelectScratch,
    pub memo: &'a mut Vec<Memo>,
    pub visits: Visits,
}

impl PredicateProgram for PredicateDecision<'_> {
    fn holds(&mut self, row: &[Value], symbols: &Symbols) -> Result<Verdict, OperatingError> {
        self.visits.0 += 1;
        let texts = Texts {
            symbols,
            source: self.source,
            paths: self.paths,
            arenas: self.arenas,
        };
        holds_bound(
            self.comparisons,
            row,
            crate::domain::comparing::types::RowShape {
                kinds: self.kinds,
                names: self.names,
            },
            &texts,
            self.bindings,
            self.prepared,
            crate::domain::matching::types::Matcher(
                &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
            ),
            crate::domain::comparing::types::DecisionWorkspace {
                select: &mut *self.select,
                memo: &mut *self.memo,
            },
        )
    }
}
