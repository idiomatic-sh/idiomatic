//! Predicate-derived ordered positions; all cache, candidate and matcher policy is external.
use super::interface::DerivedViewsItem;
use crate::domain::{
    derived_views::{service, types::*},
    operating::errors::OperatingError,
    operation_schema::types::InputSchema,
    stored_selection::types::LiteralPostings,
    table_model::types::Verdict,
};
use ryo_language::domain::symbols::{service::text_of, types::ByteSlice};
use ryo_relations::domain::relations::types::{CellKind, Value};
use std::sync::Arc;

pub(crate) struct PredicateMatchesItem;
impl DerivedViewsItem for PredicateMatchesItem {
    fn prepare(
        &self,
        request: &ViewRequest,
        inputs: &[InputSchema<'_>],
    ) -> Result<DerivedContract, OperatingError> {
        let mut contract = service::prepare(DerivedKind::PredicateMatches, request, inputs)?;
        if let ViewRequest::PredicateMatches(selector) = request {
            crate::domain::text_gate::service::needed_of(
                &selector.identity.comparisons,
                &mut contract.needed,
            );
        }
        Ok(contract)
    }
    fn build(
        &self,
        contract: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        scratch: &mut DerivedScratch,
    ) -> Result<(), OperatingError> {
        service::validate(DerivedKind::PredicateMatches, contract)?;
        let DerivedInputs::Predicate {
            rows,
            candidates,
            program,
            symbols,
        } = inputs
        else {
            return Err(OperatingError::WrongOperation);
        };
        let mut output = Vec::new();
        let mut decide = |at: usize| -> Result<(), OperatingError> {
            let row = if let crate::domain::operator::contract::views::BorrowedRows::Stored(
                source,
            ) = rows
            {
                scratch.row.resize(
                    source.kinds.len(),
                    Value::Flag(ryo_relations::domain::relations::types::Flag::Lowered),
                );
                if crate::domain::table_cells::service::read_cells(
                    &source,
                    RowIndex(at),
                    &mut scratch.row,
                    &contract.needed,
                ) != crate::domain::table_cells::types::CellsRead::Read
                {
                    return Err(OperatingError::ShapeMismatch);
                }
                scratch.row.as_slice()
            } else {
                rows.read(RowIndex(at), &mut scratch.row)
                    .ok_or(OperatingError::ShapeMismatch)?
            };
            if program.holds(row, symbols)? == Verdict::Holds {
                output.push(u32::try_from(at).map_err(|_| OperatingError::ShapeMismatch)?);
            }
            Ok(())
        };
        if let Some(candidates) = candidates {
            let mut previous = None;
            for &at in candidates {
                if previous.is_some_and(|before| before >= at) {
                    return Err(OperatingError::ShapeMismatch);
                }
                previous = Some(at);
                decide(at as usize)?;
            }
        } else {
            for at in 0..rows.count() {
                decide(at)?;
            }
        }
        *storage = DerivedStorage::RowIds {
            kind: DerivedKind::PredicateMatches,
            rows: Arc::new(output),
        };
        Ok(())
    }
    fn view<'a>(
        &self,
        contract: &DerivedContract,
        storage: &'a DerivedStorage,
    ) -> Result<DerivedView<'a>, OperatingError> {
        service::view(DerivedKind::PredicateMatches, contract, storage)
    }
}
pub(crate) struct CandidatePostingsItem;
impl DerivedViewsItem for CandidatePostingsItem {
    fn prepare(
        &self,
        request: &ViewRequest,
        inputs: &[InputSchema<'_>],
    ) -> Result<DerivedContract, OperatingError> {
        let mut contract = service::prepare(DerivedKind::CandidatePostings, request, inputs)?;
        if let ViewRequest::CandidatePostings { column, literals } = &mut contract.request {
            literals.sort_by(|left, right| left.0.cmp(&right.0));
            literals.dedup_by(|left, right| left.0 == right.0);
            if contract.metadata.is_ok()
                && (inputs
                    .first()
                    .and_then(|input| input.kinds.get(column.0 as usize))
                    != Some(&CellKind::Symbol)
                    || literals.iter().any(|text| text.0.is_empty()))
            {
                contract.metadata = Err(OperatingError::ShapeMismatch)
            }
        }
        Ok(contract)
    }
    fn build(
        &self,
        contract: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        _scratch: &mut DerivedScratch,
    ) -> Result<(), OperatingError> {
        service::validate(DerivedKind::CandidatePostings, contract)?;
        let (ViewRequest::CandidatePostings { column, literals }, supplied) =
            (&contract.request, inputs)
        else {
            return Err(OperatingError::WrongOperation);
        };
        let (source, texts, active) = match supplied {
            DerivedInputs::Text { rows, texts } => (
                *rows
                    .first()
                    .ok_or(OperatingError::MissingInput { position: 1 })?,
                texts,
                literals.as_slice(),
            ),
            DerivedInputs::Postings {
                rows,
                texts,
                active,
            } => (rows, texts, active),
            _ => return Err(OperatingError::WrongOperation),
        };
        for (at, text) in active.iter().enumerate() {
            if (at > 0 && active[at - 1].0 >= text.0)
                || literals
                    .binary_search_by(|allowed| allowed.0.cmp(&text.0))
                    .is_err()
            {
                return Err(OperatingError::WrongOperation);
            }
        }
        let mut output = active
            .iter()
            .map(|text| LiteralPostings {
                text: text.0.to_string(),
                rows: Arc::new(Vec::new()),
            })
            .collect::<Vec<_>>();
        for at in 0..source.count() {
            let Some(Value::Symbol(symbol)) = source.cell(RowIndex(at), *column) else {
                return Err(OperatingError::ShapeMismatch);
            };
            let Some(text) = text_of(texts.symbols, ByteSlice(texts.source.0), symbol) else {
                continue;
            };
            if let Ok(place) = output.binary_search_by(|entry| entry.text.as_str().cmp(text.0)) {
                Arc::get_mut(&mut output[place].rows)
                    .ok_or(OperatingError::NotRun)?
                    .push(u32::try_from(at).map_err(|_| OperatingError::ShapeMismatch)?);
            }
        }
        *storage = DerivedStorage::Postings(Arc::new(output));
        Ok(())
    }
    fn view<'a>(
        &self,
        contract: &DerivedContract,
        storage: &'a DerivedStorage,
    ) -> Result<DerivedView<'a>, OperatingError> {
        service::view(DerivedKind::CandidatePostings, contract, storage)
    }
}
pub(crate) struct DistinctRepresentativesItem;
impl DerivedViewsItem for DistinctRepresentativesItem {
    fn prepare(
        &self,
        request: &ViewRequest,
        inputs: &[InputSchema<'_>],
    ) -> Result<DerivedContract, OperatingError> {
        service::prepare(DerivedKind::DistinctRepresentatives, request, inputs)
    }
    fn build(
        &self,
        contract: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        _scratch: &mut DerivedScratch,
    ) -> Result<(), OperatingError> {
        service::validate(DerivedKind::DistinctRepresentatives, contract)?;
        let (
            ViewRequest::DistinctRepresentatives { columns },
            DerivedInputs::Distinct { rows, positions },
        ) = (&contract.request, inputs)
        else {
            return Err(OperatingError::WrongOperation);
        };
        if let crate::domain::operator::contract::views::BorrowedRows::Stored(source) = rows {
            let output = crate::domain::stored_selection::service::representatives(
                source,
                columns,
                positions,
                crate::domain::stored_selection::service::projection_hash,
            )
            .ok_or(OperatingError::ShapeMismatch)?;
            *storage = DerivedStorage::RowIds {
                kind: DerivedKind::DistinctRepresentatives,
                rows: Arc::new(output),
            };
            return Ok(());
        }
        use crate::domain::table_model::{
            service::{mix, word_of},
            types::KeyCellsHash,
        };
        let mut seen = std::collections::HashMap::<KeyCellsHash, Vec<u32>>::new();
        let mut output = Vec::new();
        for &at in positions {
            let hash = columns.iter().try_fold(KeyCellsHash(0), |hash, column| {
                rows.cell(RowIndex(at as usize), *column)
                    .map(|value| mix(hash, word_of(&value)))
                    .ok_or(OperatingError::ShapeMismatch)
            })?;
            let bucket = seen.entry(hash).or_default();
            if !bucket.iter().any(|&prior| {
                columns.iter().all(|column| {
                    rows.cell(RowIndex(prior as usize), *column)
                        == rows.cell(RowIndex(at as usize), *column)
                })
            }) {
                bucket.push(at);
                output.push(at);
            }
        }
        *storage = DerivedStorage::RowIds {
            kind: DerivedKind::DistinctRepresentatives,
            rows: Arc::new(output),
        };
        Ok(())
    }
    fn view<'a>(
        &self,
        contract: &DerivedContract,
        storage: &'a DerivedStorage,
    ) -> Result<DerivedView<'a>, OperatingError> {
        service::view(DerivedKind::DistinctRepresentatives, contract, storage)
    }
}

use ryo_values::domain::rows::types::RowIndex;

use ryo_derived::domain::contract::types::DerivedKind;
