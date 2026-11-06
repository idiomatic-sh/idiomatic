//! Application policy for prepared compact selections and their shared file backend.
use crate::application::file_dag::derived::backend::CacheBackend;
use crate::domain::{
    derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch, ViewRequest},
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::views::BorrowedRows,
    stored_selection::types::{
        PreparedSelectionViews, PreparedSelectorViews, Selector, SelectorHistogram,
    },
    table_model::{
        service::{check_columns, matching_stored},
        types::{SelectedRows, StoredSelection, Verdict},
    },
    table_texts::types::Texts,
};
use ryo_derived::domain::contract::types::DerivedKind;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::contract::types::KeyIndex;
use ryo_language::domain::symbols::{
    service::{find_hashed, hash_of},
    types::ByteSlice,
};
use ryo_relations::domain::relations::{
    service::from_bits,
    types::{Bits, CellKind, ColumnsView, Integer, Value},
};
use ryo_relations::domain::terms::service::symbols_with_text;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, LiteralText, Operand, QueryLiteral, Test,
};
use std::sync::Arc;
pub(super) mod candidates;
mod fanout;
mod preparation;
use super::owner::acquisition::selection::service::matches;
#[cfg(test)]
use crate::domain::stored_selection::types::program::value::SelectionProgram;
use candidates::{candidates, first_literal};
use fanout::fanout_candidates;
pub(in crate::application::file_dag::derived) use preparation::activate_program;
pub(in crate::application::file_dag::derived) use preparation::install_views;
pub(in crate::application::file_dag) use preparation::prepare_views_with_schemas;
#[cfg(test)]
use ryo_values::domain::plans::types::Plan;
// One owner bounds the actual shared position allocations and retained predicate entries.
pub(super) const MAX_RETAINED_SELECTIONS: usize = 128;
pub(super) const MAX_RETAINED_POSITION_CAPACITY: usize = 512 * 1024;
// A single pass can route several authored literals without a whole-column key index.
// Existing fanout count, text and position bounds remain independent of byte admission.
const MAX_FANOUT_POPULATION: usize = 256 * 1024;
pub(super) const MAX_FANOUT_LITERALS: usize = 128;
pub(super) const MAX_FANOUT_GROUPS: usize = 4;
pub(super) const MAX_FANOUT_TEXT_BYTES: usize = 64 * 1024;
const MIN_FANOUT_LITERALS: usize = 8;

/// Diagnostic reasons are fixed integers so normal selection never formats strings.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::application::file_dag::derived) enum SelectionPath {
    NoDiscriminator = 0,
    NotRepeated = 1,
    SmallPopulation = 2,
    DenseSample = 3,
    BudgetRefused = 4,
    IndexBuilt = 5,
    IndexReused = 6,
    FanoutBuilt = 7,
    FanoutReused = 8,
}

pub(super) enum CandidateRows {
    Owned(Vec<u32>),
    Shared {
        rows: Arc<Vec<u32>>,
        _resource: crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
    },
}

impl CandidateRows {
    pub(super) fn as_slice(&self) -> &[u32] {
        match self {
            Self::Owned(rows) => rows,
            Self::Shared { rows, .. } => rows.as_ref().as_slice(),
        }
    }
}

/// An absent authored symbol spelling can rule out every row without building an index.
/// Other unsupported predicates still need ordinary row evaluation.
enum Discriminator {
    Unsupported,
    Impossible,
    Indexable(ColumnIndex, Value),
}

pub(super) fn histogram<'a>(
    cache: &'a mut CacheBackend,
    program: &Arc<Selector>,
) -> Option<&'a mut SelectorHistogram> {
    let entries = cache.selection_histogram.as_mut()?;
    let at = entries
        .iter()
        .position(|entry| Arc::ptr_eq(&entry.selector, program));
    let at = at.unwrap_or_else(|| {
        entries.push(SelectorHistogram::new(program));
        entries.len() - 1
    });
    entries.get_mut(at)
}

/// Prepare request-specific metadata once per worker, without evaluating any relation.
/// Exact ordered predicates and referenced binding values; origins/projections are not result identity.
fn same(left: &Selector, right: &Selector) -> bool {
    left.relation == right.relation
        && left.identity.comparisons == right.identity.comparisons
        && left.identity.bindings == right.identity.bindings
}

pub(in crate::application::file_dag) struct SelectionOutput<'a> {
    pub rows: SelectedRows<'a>,
    pub resources: [Option<
        crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
    >; 2],
}
/// Produce a compact handle only after the provider has been read successfully by the caller.
pub(in crate::application::file_dag::derived) fn select<'a>(
    cache: &mut CacheBackend,
    resources: super::owner::acquisition::selection::service::SelectionSource<'_>,
    program: &Arc<Selector>,
    source: ColumnsView<'a>,
    context: &mut OperatorContext<'_>,
) -> Result<Option<SelectionOutput<'a>>, OperatingError> {
    if cache.eligible.contains(&program.relation) {
        make(cache, resources, program, source, context)
    } else {
        Ok(None)
    }
}

/// Validate projected columns before cache acquisition and leave matching ownership independent of projection.
fn make<'a>(
    cache: &mut CacheBackend,
    resources: super::owner::acquisition::selection::service::SelectionSource<'_>,
    program: &Arc<Selector>,
    source: ColumnsView<'a>,
    context: &mut OperatorContext<'_>,
) -> Result<Option<SelectionOutput<'a>>, OperatingError> {
    check_columns(source.kinds, &program.columns)?;
    if let Some(entry) = histogram(cache, program) {
        entry.calls += 1;
    }
    let population = source.columns.first().map_or(0, |column| column.0.len());
    let Some(views) = cache
        .selection_views
        .as_ref()
        .and_then(|prepared| prepared.by_selector.get(&(Arc::as_ptr(program) as usize)))
        .cloned()
    else {
        return Ok(None);
    };
    #[cfg(test)]
    {
        cache.selector_descriptor = Some(Arc::as_ptr(&views) as usize);
    }
    if views
        .predicate
        .schemas
        .first()
        .is_none_or(|kinds| kinds.as_slice() != source.kinds)
    {
        return Ok(None);
    }
    let predicate = matches(cache, resources, program, &views, source, context)?;
    let rows = shared_positions(&predicate, population, population)?;
    let mut dependencies = [Some(predicate), None];
    // The shared result above always contains predicate matches, never distinct representatives.
    // Each terminal projection owns its representative IDs independently of other output maps.
    let rows = if program.distinct {
        let acquired = resources.acquire(
            crate::application::file_dag::route_counts::BuildRoute::SelectorDistinct,
            cache,
            &views.distinct,
            super::owner::ViewContext::for_predicate(&views.predicate),
            DerivedInputs::Distinct {
                rows: BorrowedRows::Stored(source),
                positions: &rows,
            },
            &mut DerivedScratch::default(),
            None,
        )?;
        let result = shared_positions(&acquired, population, rows.len())?;
        dependencies[1] = Some(acquired);
        result
    } else {
        rows
    };
    let kinds = program
        .columns
        .iter()
        .map(|column| source.kinds[column.0 as usize])
        .collect();
    Ok(Some(SelectionOutput {
        resources: dependencies,
        rows: SelectedRows {
            source,
            selection: Arc::new(StoredSelection {
                relation: program.relation,
                rows,
                columns: program.columns.clone(),
                kinds,
                identity: (program.identity).clone(),
            }),
        },
    }))
}

#[cfg(test)]
#[path = "selection/retention_test.rs"]
mod retention_test;

/// Admit the trait's immutable positions before transferring the same owned buffer
/// to the legacy compact handle. Registered predicate/distinct kernels establish
/// the ordered admitted-position invariant; first/last checks here are constant-time
/// cardinality/edge guards, not validation of arbitrary untrusted position vectors.
pub(super) fn shared_positions(
    acquired: &crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
    population: usize,
    maximum: usize,
) -> Result<Arc<Vec<u32>>, OperatingError> {
    let DerivedView::RowIds(rows) = acquired.view()? else {
        return Err(OperatingError::WrongOperation);
    };
    if rows.len() > maximum
        || rows.first().is_some_and(|row| *row as usize >= population)
        || rows.last().is_some_and(|row| *row as usize >= population)
    {
        return Err(OperatingError::ShapeMismatch);
    }
    let rows = acquired
        .storage()
        .payload()
        .share_positions()
        .ok_or(OperatingError::WrongOperation)?;
    Ok(Arc::clone(rows.backing()))
}

use ryo_values::domain::query_bindings::types::ParameterBinding;

#[cfg(test)]
pub(crate) mod fixtures;
