//! The `ReadSet` value: the reads of the relations one step rests on, stored once per program and shared by every step that
//! rests on the same reads. Only a program's `ReadSets` makes one that is not empty.
use super::super::RelationRead;
#[cfg(any(test, feature = "fixtures"))]
use super::super::SetSharing;
use std::sync::Arc;

/// The reads of the relations one step rests on, each once, shared by every step of the program that rests on the same reads.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::trust::types::read_set::value::ReadSet;
/// let none = ReadSet::default();
/// assert!(none.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReadSet(pub(in crate::domain::trust) Arc<[RelationRead]>);

impl std::ops::Deref for ReadSet {
    type Target = [RelationRead];
    fn deref(&self) -> &[RelationRead] {
        &self.0
    }
}

impl ReadSet {
    /// Whether `self` and `other` are one stored set, not two equal copies.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::trust::types::read_set::value::ReadSet;
    /// let set = ReadSet::default();
    /// assert!(set.is_empty());
    /// # #[cfg(feature = "fixtures")]
    /// assert_eq!(
    ///     set.sharing(&set.clone()),
    ///     ryo_execution_domain::domain::trust::types::SetSharing::Shared
    /// );
    /// ```
    #[must_use]
    #[cfg(any(test, feature = "fixtures"))]
    pub fn sharing(&self, other: &Self) -> SetSharing {
        if Arc::ptr_eq(&self.0, &other.0) {
            SetSharing::Shared
        } else {
            SetSharing::Copied
        }
    }
}
