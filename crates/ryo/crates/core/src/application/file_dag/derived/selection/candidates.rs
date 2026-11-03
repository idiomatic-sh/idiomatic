//! Candidate choice and immutable equality-cache acquisition remain application policy.
use super::*;
/// Use a column index only when multiple active descriptors can amortize its build.
pub(super) fn candidates(
    cache: &mut CacheBackend,
    resources: super::super::owner::selection::SelectionSource<'_>,
    program: &Selector,
    views: &PreparedSelectorViews,
    source: ColumnsView<'_>,
    context: &OperatorContext<'_>,
) -> Result<(Option<CandidateRows>, SelectionPath), OperatingError> {
    let population = source.columns.first().map_or(0, |column| column.0.len());
    if let Some((rows, acquired, path)) =
        fanout_candidates(cache, resources, program, &views.postings, source, context)?
    {
        cache.selection_counts.candidates += rows.len() as u64;
        return Ok((
            Some(CandidateRows::Shared {
                rows,
                _resource: acquired,
            }),
            path,
        ));
    }
    let mut path = SelectionPath::NoDiscriminator;
    for (number, comparison) in program.identity.comparisons.iter().enumerate() {
        let (column, value) = match discriminator(comparison, program, source, context) {
            // holds_all evaluates in order. Only the first impossible comparison
            // may bypass rows without hiding an earlier comparison's error.
            Discriminator::Impossible if number == 0 => {
                return Ok((
                    Some(CandidateRows::Owned(Vec::new())),
                    SelectionPath::NoDiscriminator,
                ));
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
        if !selective(source, column, value, population) {
            path = path.max(SelectionPath::DenseSample);
            continue;
        }
        let equality = views
            .equalities
            .get(&column)
            .cloned()
            .ok_or(OperatingError::NotRun)?;
        if !resources.can_retain_equality(cache, program.relation, source, &[column]) {
            path = path.max(SelectionPath::BudgetRefused);
            continue;
        }
        let acquired = resources.acquire(
            crate::application::file_dag::route_counts::BuildRoute::SelectorCandidate,
            cache,
            &equality,
            super::super::owner::ViewContext::default(),
            DerivedInputs::Rows(&[BorrowedRows::Stored(source)]),
            &mut DerivedScratch::default(),
            Some((program.relation, source)),
        )?;
        if acquired.was_built() {
            cache.selection_counts.scanned += population as u64;
        }
        let DerivedView::Equality(order) = acquired.view()? else {
            return Err(OperatingError::WrongOperation);
        };
        let result: Vec<_> = matching_stored(source, &[column], order, &[value])
            .filter_map(|index| u32::try_from(index.0).ok())
            .collect();
        cache.selection_counts.candidates += result.len() as u64;
        return Ok((
            Some(CandidateRows::Owned(result)),
            if acquired.was_built() {
                SelectionPath::IndexBuilt
            } else {
                SelectionPath::IndexReused
            },
        ));
    }
    Ok((None, path))
}

/// Only a total, authored, nonempty text equality in the first predicate can
/// reject rows before any caller-owned comparison or error origin is visited.
pub(super) fn first_literal<'a>(
    selector: &'a Selector,
    kinds: &[CellKind],
) -> Option<(ColumnIndex, &'a str)> {
    let comparison = selector.identity.comparisons.first()?;
    if comparison.test != Test::Equal {
        return None;
    }
    let (column, text) = match (&comparison.left, &comparison.right) {
        (Operand::Column(column), Operand::Literal(QueryLiteral::Text(text)))
        | (Operand::Literal(QueryLiteral::Text(text)), Operand::Column(column)) => {
            (column, &text.0)
        }
        _ => return None,
    };
    (kinds.get(column.0 as usize) == Some(&CellKind::Symbol) && !text.is_empty())
        .then_some((*column, text.as_ref()))
}

/// Route all active texts for one physical column in one pass. This is a
/// file-owned candidate index, never a cached predicate verdict: every owner
/// still evaluates its ordered comparisons and reports its own error origin.
/// A bounded typed sample rejects dense discriminators before allocating an index.
fn selective(
    source: ColumnsView<'_>,
    column: ColumnIndex,
    value: Value,
    population: usize,
) -> bool {
    let Some(kind) = source.kinds.get(column.0 as usize) else {
        return false;
    };
    let Some(values) = source.columns.get(column.0 as usize) else {
        return false;
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
    matches.saturating_mul(4) < samples
}

/// Equality lookup never interns query text and verifies symbol hashes by exact text.
fn discriminator(
    comparison: &ryo_values::domain::plan_parts::types::Comparison,
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
            symbols_with_text(&text.0, |candidate| {
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
