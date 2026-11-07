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

impl RuleObservationDeclarations {
    /// Resolve only profile defaults for the selected rules; an explicit case declaration bypasses this policy.
    #[must_use]
    pub fn project_profile(
        &self,
        selected: &[RuleId],
        policy: crate::domain::workloads::types::ObservationSelectionPolicy,
    ) -> Self {
        if !selected.iter().any(|id| self.contains_key(id)) {
            return Self::default();
        }
        let selected: std::collections::BTreeSet<_> = selected.iter().collect();
        let mut pending: Vec<RuleId> = selected
            .iter()
            .filter(|id| self.contains_key(**id))
            .map(|id| (**id).clone())
            .collect();
        let mut resolved = Self::default();
        while let Some(id) = pending.pop() {
            if resolved.contains_key(&id) {
                continue;
            }
            let Some(authored) = self.get(&id) else {
                continue;
            };
            let mut bindings = authored.clone();
            if policy == crate::domain::workloads::types::ObservationSelectionPolicy::Selected {
                for binding in &mut bindings {
                    binding.producers.retain(|producer| match producer {
                        ryo::linter::ObservationProducer::Output { rule, .. }
                        | ryo::linter::ObservationProducer::NoQuery { rule } => {
                            selected.contains(rule)
                        }
                    });
                }
            }
            for binding in &bindings {
                for producer in &binding.producers {
                    if let ryo::linter::ObservationProducer::Output { rule, .. } = producer
                        && self.contains_key(rule)
                    {
                        pending.push(rule.clone());
                    }
                }
            }
            resolved.insert(id, bindings);
        }
        resolved
    }
}
