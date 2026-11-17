//! The `ReadSets` value: the read sets of one program, each stored once, so a step that rests on the reads another step
//! rests on holds the same set, not a copy; and what each step of a plan reads, as sets of it.
#[cfg(any(test, feature = "fixtures"))]
use super::super::StoredSets;
use super::super::read_set::value::ReadSet;
use super::super::{ReadSetHash, RelationRead};
use std::hash::{Hash, Hasher};

/// The read sets of one program, each stored once, found by their hash.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::trust::types::read_sets::value::ReadSets;
/// let mut sets = ReadSets::default();
/// assert_eq!(sets.shared(Vec::new()), sets.shared(Vec::new()));
/// ```
#[derive(Debug, Default)]
pub struct ReadSets {
    /// The sets stored, by their hash: the sets of one hash, each unequal to the others.
    stored: hashbrown::HashMap<ReadSetHash, Vec<ReadSet>>,
}

/// What one step's rows rest on, each set stored once in its program's [`ReadSets`]: every relation read on the way to them,
/// and the reads whose absence of rows the step or a step it reads trusts.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::trust::types::read_sets::value::StepReadSets;
/// let none = StepReadSets::default();
/// assert!(none.uses.is_empty() && none.needs.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepReadSets {
    /// Every relation read on the way to the step's rows, narrowed as the steps narrow it.
    pub uses: ReadSet,
    /// The reads whose absence of rows the step or a step it reads trusts.
    pub needs: ReadSet,
}

/// What each step of one plan reads, by step.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::trust::types::read_sets::value::PlanReadSets;
/// assert!(PlanReadSets::default().0.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlanReadSets(pub Box<[StepReadSets]>);

impl ReadSets {
    /// The stored set equal to `reads`, or `reads` stored as a new one.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::trust::types::read_sets::value::ReadSets;
    /// let mut sets = ReadSets::default();
    /// assert!(sets.shared(Vec::new()).is_empty());
    /// ```
    #[must_use]
    pub fn shared(&mut self, reads: Vec<RelationRead>) -> ReadSet {
        let mut hasher = rustc_hash::FxHasher::default();
        reads.hash(&mut hasher);
        self.stored_by(reads, ReadSetHash(hasher.finish()))
    }

    /// The stored set equal to `reads` among those of `hash`, or `reads` stored under it as a new one.
    pub(in crate::domain::trust) fn stored_by(
        &mut self,
        reads: Vec<RelationRead>,
        hash: ReadSetHash,
    ) -> ReadSet {
        let bucket = self.stored.entry(hash).or_default();
        // Equal hashes are not equal sets: a set is shared only with one equal to it.
        if let Some(found) = bucket.iter().find(|each| ***each == *reads) {
            return found.clone();
        }
        let made = ReadSet(reads.into());
        bucket.push(made.clone());
        made
    }

    /// How many distinct sets the program stores.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::trust::types::read_sets::value::ReadSets;
    /// let mut sets = ReadSets::default();
    /// assert!(sets.shared(Vec::new()).is_empty());
    /// # #[cfg(feature = "fixtures")]
    /// assert_eq!(sets.distinct(), ryo_execution_domain::domain::trust::types::StoredSets(1));
    /// ```
    #[cfg(any(test, feature = "fixtures"))]
    #[must_use]
    pub fn distinct(&self) -> StoredSets {
        StoredSets(self.stored.values().map(Vec::len).sum())
    }
}
