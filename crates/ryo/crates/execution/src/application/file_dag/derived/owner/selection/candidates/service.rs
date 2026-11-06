//! Candidate choice and immutable equality-cache acquisition remain application policy.
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
use crate::application::file_dag::derived::owner::acquisition::selection::types::StoredColumns;
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
use crate::application::file_dag::derived::owner::selection::fanout::service::fanout_candidates;
use crate::application::file_dag::derived::owner::selection::types::*;
use crate::domain::{
    derived_views::types::{DerivedInputs, DerivedScratch},
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::views::BorrowedRows,
    stored_selection::types::{PreparedSelectorViews, Selector},
    table_model::service::matching_stored,
};
use ryo_derived::domain::contract::types::DerivedView;
use ryo_language::domain::symbols::{
    service::{find_hashed, hash_of},
    types::ByteSlice,
};
use ryo_relations::domain::relations::{
    service::from_bits,
    types::{Bits, CellKind, ColumnsView, Integer, Value},
};
use ryo_relations::domain::terms::service::symbols_with_text;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Operand, QueryLiteral, Test},
    query_bindings::types::ParameterBinding,
};

/// Use a column index only when multiple active descriptors can amortize its build.
pub(in crate::application::file_dag::derived::owner) fn candidates(
    cache: &mut CacheBackend,
    resources: crate::application::file_dag::derived::owner::acquisition::selection::service::SelectionSource<'_>,
    program: &Selector,
    views: &PreparedSelectorViews,
    source: ColumnsView<'_>,
    context: &OperatorContext<'_>,
) -> Result<CandidateChoice, OperatingError> {
    let population = source.columns.first().map_or(0, |column| column.0.len());
    if let Some(FanoutHit {
        rows,
        resource,
        path,
    }) = fanout_candidates(cache, resources, program, &views.postings, source, context)?
    {
        cache.selection_counts.candidates += rows.0.len() as u64;
        return Ok(CandidateChoice {
            rows: Some(CandidateRows::Shared {
                rows,
                _resource: resource,
            }),
            path,
        });
    }
    let mut path = SelectionPath::NoDiscriminator;
    for (number, comparison) in program.identity.comparisons.iter().enumerate() {
        let (column, value) = match discriminator(comparison, program, source, context) {
            // holds_all evaluates in order. Only the first impossible comparison
            // may bypass rows without hiding an earlier comparison's error.
            Discriminator::Impossible if number == 0 => {
                return Ok(CandidateChoice {
                    rows: Some(CandidateRows::Owned(CandidatePositions(Vec::new()))),
                    path: SelectionPath::NoDiscriminator,
                });
            }
            Discriminator::Indexable(column, value) => (column, value),
            Discriminator::Impossible | Discriminator::Unsupported => continue,
        };
        path = path.max(SelectionPath::NotRepeated);
        let reused = cache.discriminator_columns.iter().any(|entry| {
            entry.relation == program.relation && entry.column == column && entry.repeated
        });
        if !reused {
            continue;
        }
        if population < 32 {
            path = path.max(SelectionPath::SmallPopulation);
            continue;
        }
        if selective(source, column, value, RowPopulation(population)) == Selectivity::Dense {
            path = path.max(SelectionPath::DenseSample);
            continue;
        }
        let equality = views
            .equalities
            .get(&column)
            .cloned()
            .ok_or(OperatingError::NotRun)?;
        if resources.can_retain_equality(cache, program.relation, source, &[column])
            == Retainability::Unretainable
        {
            path = path.max(SelectionPath::BudgetRefused);
            continue;
        }
        let acquired = resources.acquire(
            crate::application::file_dag::route_counts::BuildRoute::SelectorCandidate,
            cache,
            &equality,
            crate::application::file_dag::derived::owner::ViewContext::default(),
            DerivedInputs::Rows(&[BorrowedRows::Stored(source)]),
            &mut DerivedScratch::default(),
            Some(StoredColumns {
                relation: program.relation,
            }),
        )?;
        if acquired.construction() == Construction::Built {
            cache.selection_counts.scanned += population as u64;
        }
        let DerivedView::Equality(order) = acquired.view()? else {
            return Err(OperatingError::WrongOperation);
        };
        let result: Vec<_> = matching_stored(
            source,
            &[column],
            order,
            &[value],
            #[cfg(feature = "diagnostic-row-progress")]
            crate::application::file_dag::route_counts::progress,
        )
        .flat_map(|index| u32::try_from(index.0))
        .collect();
        cache.selection_counts.candidates += result.len() as u64;
        return Ok(CandidateChoice {
            rows: Some(CandidateRows::Owned(CandidatePositions(result))),
            path: if acquired.construction() == Construction::Built {
                SelectionPath::IndexBuilt
            } else {
                SelectionPath::IndexReused
            },
        });
    }
    Ok(CandidateChoice { rows: None, path })
}

/// Route all active texts for one physical column in one pass. This is a
/// file-owned candidate index, never a cached predicate verdict: every owner
/// still evaluates its ordered comparisons and reports its own error origin.
/// A bounded typed sample rejects dense discriminators before allocating an index.
fn selective(
    source: ColumnsView<'_>,
    column: ColumnIndex,
    value: Value,
    population: RowPopulation,
) -> Selectivity {
    let population = population.0;
    let Some(kind) = source.kinds.get(column.0 as usize) else {
        return Selectivity::Dense;
    };
    let Some(values) = source.columns.get(column.0 as usize) else {
        return Selectivity::Dense;
    };
    let samples = population.min(64);
    let matches = (0..samples)
        .filter(|index| {
            values
                .0
                .get(index * population / samples)
                .is_some_and(|bits| from_bits(*kind, Bits(*bits)) == value)
        })
        .count();
    if matches.saturating_mul(4) < samples {
        Selectivity::Selective
    } else {
        Selectivity::Dense
    }
}

/// Equality lookup never interns query text and verifies symbol hashes by exact text.
fn discriminator(
    comparison: &ryo_values::domain::plan_parts::types::PlanComparison,
    program: &Selector,
    source: ColumnsView<'_>,
    context: &OperatorContext<'_>,
) -> Discriminator {
    if comparison.test != Test::Equal {
        return Discriminator::Unsupported;
    }
    let (column, operand) = match (&comparison.left, &comparison.right) {
        (Operand::Column(column), value) | (value, Operand::Column(column)) => (*column, value),
        _ => return Discriminator::Unsupported,
    };
    let literal = match operand {
        Operand::Literal(value) => value.clone(),
        Operand::Parameter(id) => {
            let Some(binding) = program.identity.bindings.get(id.0 as usize) else {
                return Discriminator::Unsupported;
            };
            match binding {
                ParameterBinding::Text(text) => QueryLiteral::Text(text.clone()),
                ParameterBinding::Number(number) => QueryLiteral::Number(Integer(number.0)),
                ParameterBinding::Flag(_)
                | ParameterBinding::Numbers(_)
                | ParameterBinding::Texts(_)
                | ParameterBinding::TextSets(_)
                | ParameterBinding::Tables(_) => return Discriminator::Unsupported,
            }
        }
        Operand::Column(_) | Operand::Function(_) => return Discriminator::Unsupported,
    };
    let value = match (source.kinds.get(column.0 as usize), literal) {
        (Some(CellKind::Number), QueryLiteral::Number(value)) => Value::Number(value),
        (Some(CellKind::Symbol), QueryLiteral::Text(text)) => {
            let hash = hash_of(ByteSlice(text.0.as_bytes()));
            // A file name and one or more closed-set terms can have the same text but
            // different Symbol IDs. The ordinary evaluator compares their text, while
            // matching_stored compares exact IDs. Index only an unambiguous spelling.
            let mut found_symbol = find_hashed(
                context.symbols.as_ref(),
                ByteSlice(context.source.0),
                ByteSlice(text.0.as_bytes()),
                hash,
            );
            let mut ambiguous = false;
            symbols_with_text(NameText(&text.0), |candidate| {
                if found_symbol.replace(candidate).is_some() {
                    ambiguous = true;
                }
            });
            if ambiguous {
                return Discriminator::Unsupported;
            }
            let Some(symbol) = found_symbol else {
                // Literal equality on Symbol rows compares text. Valid symbols
                // with this spelling would be in the dynamic or term lookup.
                // Keep empty text on the scan path: invalid symbols decode to
                // empty text in the ordinary evaluator. Bound parameters also
                // use the ordinary path, including their error behavior.
                return if matches!(operand, Operand::Literal(QueryLiteral::Text(_)))
                    && !text.0.is_empty()
                {
                    Discriminator::Impossible
                } else {
                    Discriminator::Unsupported
                };
            };
            Value::Symbol(symbol)
        }
        _ => return Discriminator::Unsupported,
    };
    Discriminator::Indexable(column, value)
}
