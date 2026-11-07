//! Owned declaration map; duplicate keys have the same BTreeMap replacement semantics.
use ryo::linter::{ObservationBinding, RuleId};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default)]
pub struct RuleObservationDeclarations(BTreeMap<RuleId, Vec<ObservationBinding>>);
impl std::ops::Deref for RuleObservationDeclarations {
    type Target = BTreeMap<RuleId, Vec<ObservationBinding>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for RuleObservationDeclarations {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<BTreeMap<RuleId, Vec<ObservationBinding>>> for RuleObservationDeclarations {
    fn from(value: BTreeMap<RuleId, Vec<ObservationBinding>>) -> Self {
        Self(value)
    }
}
impl FromIterator<(RuleId, Vec<ObservationBinding>)> for RuleObservationDeclarations {
    fn from_iter<T: IntoIterator<Item = (RuleId, Vec<ObservationBinding>)>>(values: T) -> Self {
        Self(values.into_iter().collect())
    }
}
impl IntoIterator for RuleObservationDeclarations {
    type Item = (RuleId, Vec<ObservationBinding>);
    type IntoIter = std::collections::btree_map::IntoIter<RuleId, Vec<ObservationBinding>>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<'a> IntoIterator for &'a RuleObservationDeclarations {
    type Item = (&'a RuleId, &'a Vec<ObservationBinding>);
    type IntoIter = std::collections::btree_map::Iter<'a, RuleId, Vec<ObservationBinding>>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
