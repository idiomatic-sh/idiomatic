//! Read-only physical inputs expose no owned cache or retained selection handle.

use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use ryo_derived::domain::contract::types::KeyIndex;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(crate) use crate::domain::table_model::collections::representations::types::RepresentationInput as BorrowedRows;

/// Profile-only semantic observations. No execution or ownership policy is reachable.
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Copy)]
pub(crate) enum ProgressCounter {
    InputRows,
    PredicateEvaluations,
    IndexProbes,
    IndexRowsExamined,
    EqualityMatches,
    JoinCandidatePairs,
    KernelOutputRows,
}
#[cfg(feature = "diagnostic-row-progress")]
/// How much one profile observation adds to its counter: the bare count inside is named here.
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Copy)]
pub(crate) struct ProgressAmount(pub u64);
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) type ProgressObserver = fn(ProgressCounter, ProgressAmount);

/// Exact typed key lookup prepared by the engine; primitives only probe it.
#[derive(Clone, Copy)]
pub(crate) struct IndexedRows<'a> {
    #[cfg(feature = "diagnostic-row-progress")]
    pub progress: ProgressObserver,
    pub rows: BorrowedRows<'a>,
    pub columns: &'a [ColumnIndex],
    pub index: &'a KeyIndex,
}
impl IndexedRows<'_> {
    pub(crate) fn contains(&self, key: &[Value]) -> bool {
        self.matching(key).next().is_some()
    }
    pub(crate) fn probe(&self, key: &[Value]) -> KeyProbe {
        #[cfg(feature = "diagnostic-row-progress")]
        (self.progress)(ProgressCounter::IndexProbes, ProgressAmount(1));
        let hash = ryo_derived::domain::contract::key::hash_key(key.iter().map(Some));
        let bucket =
            usize::try_from(hash.0).unwrap_or(0) & self.index.heads().len().saturating_sub(1);
        KeyProbe {
            hash,
            at: self
                .index
                .heads()
                .get(bucket)
                .copied()
                .unwrap_or(ryo_derived::domain::contract::types::ChainLink(0)),
        }
    }
    pub(crate) fn next_match(&self, key: &[Value], probe: &mut KeyProbe) -> Option<RowIndex> {
        use ryo_derived::domain::contract::types::ChainLink;
        while probe.at.0 != 0 {
            #[cfg(feature = "diagnostic-row-progress")]
            (self.progress)(ProgressCounter::IndexRowsExamined, ProgressAmount(1));
            let position = usize::try_from(probe.at.0 - 1).unwrap_or(usize::MAX);
            probe.at = self
                .index
                .links()
                .get(position)
                .copied()
                .unwrap_or(ChainLink(0));
            let row = self.index.rows().get(position).copied()?;
            if self.index.hashes().get(position) == Some(&probe.hash)
                && self
                    .columns
                    .iter()
                    .zip(key)
                    .all(|(column, expected)| self.rows.cell(row, *column) == Some(*expected))
            {
                #[cfg(feature = "diagnostic-row-progress")]
                (self.progress)(ProgressCounter::EqualityMatches, ProgressAmount(1));
                return Some(row);
            }
        }
        None
    }
    pub(crate) fn matching<'a>(&'a self, key: &'a [Value]) -> MatchingRows<'a> {
        MatchingRows {
            view: self,
            key,
            probe: self.probe(key),
        }
    }
}
/// Physical hash-chain position, independent of the caller-owned key buffer.
pub(crate) struct KeyProbe {
    hash: ryo_derived::domain::contract::types::KeyCellsHash,
    at: ryo_derived::domain::contract::types::ChainLink,
}
pub(crate) struct MatchingRows<'a> {
    view: &'a IndexedRows<'a>,
    key: &'a [Value],
    probe: KeyProbe,
}
impl Iterator for MatchingRows<'_> {
    type Item = RowIndex;
    fn next(&mut self) -> Option<RowIndex> {
        self.view.next_match(self.key, &mut self.probe)
    }
}

use ryo_values::domain::rows::types::RowIndex;

/// Finite immutable partition access; ownership and recycling remain in the invocation owner.
pub(crate) trait ClosureParts {
    fn len(&self) -> usize;
    fn get(
        &self,
        at: usize,
    ) -> Result<
        &ryo_derived::domain::contract::types::ClosurePartition,
        crate::domain::operating::errors::OperatingError,
    >;
}
#[derive(Clone, Copy, Default)]
pub(crate) enum ClosureViews<'a> {
    #[default]
    Empty,
    One(&'a ryo_derived::domain::contract::types::ClosurePartition),
    Many(&'a dyn ClosureParts),
}
impl<'a> ClosureViews<'a> {
    fn len(self) -> usize {
        match self {
            Self::Empty => 0,
            Self::One(_) => 1,
            Self::Many(parts) => parts.len(),
        }
    }
    fn get(
        self,
        at: usize,
    ) -> Result<
        &'a ryo_derived::domain::contract::types::ClosurePartition,
        crate::domain::operating::errors::OperatingError,
    > {
        match self {
            Self::Empty => Err(crate::domain::operating::errors::OperatingError::ShapeMismatch),
            Self::One(part) => (at == 0)
                .then_some(part)
                .ok_or(crate::domain::operating::errors::OperatingError::ShapeMismatch),
            Self::Many(parts) => parts.get(at),
        }
    }
}
pub(crate) struct ClosureIter<'a> {
    views: ClosureViews<'a>,
    at: usize,
}
impl<'a> Iterator for ClosureIter<'a> {
    type Item = Result<
        &'a ryo_derived::domain::contract::types::ClosurePartition,
        crate::domain::operating::errors::OperatingError,
    >;
    fn next(&mut self) -> Option<Self::Item> {
        if self.at >= self.views.len() {
            return None;
        }
        let at = self.at;
        self.at += 1;
        Some(self.views.get(at))
    }
}
impl<'a> IntoIterator for ClosureViews<'a> {
    type Item = <ClosureIter<'a> as Iterator>::Item;
    type IntoIter = ClosureIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        ClosureIter { views: self, at: 0 }
    }
}

/// The evaluator is bound to the prepared contract's exact ordered program.
/// Its implementation is engine-private; this interface cannot choose predicates,
/// schemas, cache keys or upstream scheduling.
pub(crate) trait PredicateProgram {
    fn holds(
        &mut self,
        row: &[Value],
        symbols: &ryo_language::domain::symbols::types::Symbols,
    ) -> Result<Verdict, OperatingError>;
}
