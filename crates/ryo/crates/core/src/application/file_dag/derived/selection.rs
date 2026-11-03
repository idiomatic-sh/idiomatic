//! Application policy for prepared compact selections and their shared file backend.
use crate::domain::{
    derived_views::types::{
        DerivedContract, DerivedInputs, DerivedKind, DerivedScratch, DerivedStorage, ViewRequest,
    },
    operating::{
        errors::OperatingError,
        types::{NameCount, OperatorContext},
    },
    operator::{contract::views::BorrowedRows, types::PreparedPool},
    stored_indexes::types::FileIndexes,
    stored_selection::types::{
        LiteralFanout, Matches, PreparedSelectionViews, PreparedSelectorViews, Selector,
        SelectorHistogram,
    },
    table_model::{
        service::{check_columns, matching_stored, rectangular},
        types::{KeyIndex, ParameterBinding, SelectedRows, StoredSelection, Verdict},
    },
    table_texts::types::Texts,
};
use ryo_language::domain::{
    relations::{
        service::from_bits,
        types::{Bits, CellKind, ColumnsView, Integer, Value},
    },
    symbols::{
        service::{find_hashed, hash_of, name_count},
        types::ByteSlice,
    },
    terms::service::symbols_with_text,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, LiteralText, Operand, QueryLiteral, Test,
};
use std::sync::Arc;
mod candidates;
mod fanout;
mod predicate;
mod preparation;
mod retention;
#[cfg(test)]
use crate::domain::stored_selection::types::program::value::SelectionProgram;
use candidates::{candidates, first_literal};
use fanout::fanout_candidates;
use predicate::matches;
pub(crate) use preparation::{
    activate, install_views, prepare_request, prepare_views_with_schemas,
};
#[cfg(test)]
pub(crate) use preparation::{prepare, prepare_views};
use retention::admit;
#[cfg(test)]
use ryo_values::domain::plans::types::Plan;
// One file owns this cache and drops it after the file. Reserve a fixed, conservative
// slice of its existing limit for at most this many retained match positions and
// entries; no allocation-size estimate is maintained for individual entries.
pub(super) const RETAINED_SELECTION_RESERVATION: usize = 4 * 1024 * 1024;
const MAX_RETAINED_SELECTIONS: usize = 128;
pub(super) const MAX_RETAINED_POSITION_CAPACITY: usize = 512 * 1024;
// A single pass can route several authored literals without a whole-column key index.
// The position and metadata bounds fit inside the existing 4 MiB selection reservation.
const MAX_FANOUT_POPULATION: usize = 256 * 1024;
const MAX_FANOUT_LITERALS: usize = 128;
const MAX_FANOUT_GROUPS: usize = 4;
const MAX_FANOUT_TEXT_BYTES: usize = 64 * 1024;
const MIN_FANOUT_LITERALS: usize = 8;

/// Diagnostic reasons are fixed integers so normal selection never formats strings.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SelectionPath {
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

enum CandidateRows {
    Owned(Vec<u32>),
    Shared(Arc<Vec<u32>>),
}

impl CandidateRows {
    fn as_slice(&self) -> &[u32] {
        match self {
            Self::Owned(rows) => rows,
            Self::Shared(rows) => rows.as_ref().as_slice(),
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

#[derive(Clone, Copy)]
pub(super) enum Admission {
    NotRepeated = 0,
    AllocationRefused = 1,
    BudgetRefused = 2,
    Retained = 3,
}

fn histogram<'a>(
    cache: &'a mut FileIndexes,
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

/// Produce a compact handle only after the provider has been read successfully by the caller.
pub(crate) fn select<'a>(
    program: &Arc<Selector>,
    source: ColumnsView<'a>,
    context: &mut OperatorContext<'_>,
) -> Result<Option<SelectedRows<'a>>, OperatingError> {
    if !rectangular(source)
        || source
            .columns
            .first()
            .is_some_and(|column| column.0.len() > u32::MAX as usize)
    {
        return Ok(None);
    }
    let Some(cache) = context.stored_indexes.take() else {
        return Ok(None);
    };
    let result = if cache.eligible.contains(&program.relation) {
        make(cache, program, source, context)
    } else {
        Ok(None)
    };
    context.stored_indexes = Some(cache);
    result
}

/// Validate projected columns before cache acquisition and leave matching ownership independent of projection.
fn make<'a>(
    cache: &mut FileIndexes,
    program: &Arc<Selector>,
    source: ColumnsView<'a>,
    context: &mut OperatorContext<'_>,
) -> Result<Option<SelectedRows<'a>>, OperatingError> {
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
    let rows = if let Some(found) = cache.selections.iter().find(|entry| {
        same(&entry.selector, program)
            && entry.kinds == source.kinds
            && entry.population == population
    }) {
        let rows = Arc::clone(&found.rows);
        cache.selection_counts.reuses += 1;
        if let Some(entry) = histogram(cache, program) {
            entry.reuses += 1;
        }
        rows
    } else {
        let rows = Arc::new(matches(cache, program, &views, source, context)?);
        cache.selection_counts.builds += 1;
        let admission = admit(cache, program, &views, source, population, &rows)?;
        if let Some(entry) = histogram(cache, program) {
            entry.builds += 1;
            entry.matched_rows += rows.len() as u64;
            entry.admission[admission as usize] += 1;
        }
        rows
    };
    // The shared result above always contains predicate matches, never distinct representatives.
    // Each terminal projection owns its representative IDs independently of other output maps.
    let rows = if program.distinct {
        let mut storage = DerivedStorage::Unbuilt;
        super::owner::build_scoped(
            &views.distinct,
            DerivedInputs::Distinct {
                rows: BorrowedRows::Stored(source),
                positions: &rows,
            },
            &mut storage,
            &mut DerivedScratch::default(),
        )?;
        Arc::new(take_positions(
            &views.distinct,
            &mut storage,
            population,
            rows.len(),
        )?)
    } else {
        rows
    };
    let kinds = program
        .columns
        .iter()
        .map(|column| source.kinds[column.0 as usize])
        .collect();
    Ok(Some(SelectedRows {
        source,
        selection: Arc::new(StoredSelection {
            relation: program.relation,
            rows,
            columns: program.columns.clone(),
            kinds,
            identity: Arc::clone(&program.identity),
        }),
    }))
}

#[cfg(test)]
#[path = "selection/retention_test.rs"]
mod retention_test;

/// Admit the trait's immutable positions before transferring the same owned buffer
/// to the legacy compact handle. Registered predicate/distinct kernels establish
/// the ordered admitted-position invariant; first/last checks here are constant-time
/// cardinality/edge guards, not validation of arbitrary untrusted position vectors.
fn take_positions(
    plan: &DerivedContract,
    storage: &mut DerivedStorage,
    population: usize,
    maximum: usize,
) -> Result<Vec<u32>, OperatingError> {
    let crate::domain::derived_views::types::DerivedView::RowIds(rows) =
        crate::domain::derived_views::collections::views::registration::registered(plan.kind)
            .view(plan, storage)?
    else {
        return Err(OperatingError::WrongOperation);
    };
    if rows.len() > maximum
        || rows.first().is_some_and(|row| *row as usize >= population)
        || rows.last().is_some_and(|row| *row as usize >= population)
    {
        return Err(OperatingError::ShapeMismatch);
    }
    let DerivedStorage::RowIds { rows, .. } = std::mem::take(storage) else {
        return Err(OperatingError::WrongOperation);
    };
    Ok(rows)
}
