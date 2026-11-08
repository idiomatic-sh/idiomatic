//! The hash that sorts canonical operations into buckets; it only accelerates, equality decides.

use crate::domain::coverage::types::RuleIndex;
use crate::domain::file_program::types::NodeInput;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Operation;
use std::hash::{Hash, Hasher};

/// The hash of an operation's kind, edges, scope and phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Fingerprint(pub u64);

impl Fingerprint {
    /// Hash is an accelerator only: exact typed operation, edges and scope are compared.
    pub(crate) fn of(
        operation: &Operation,
        inputs: &[NodeInput],
        scope: Option<RuleIndex>,
        phase: StepScope,
    ) -> Self {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        std::mem::discriminant(operation).hash(&mut hash);
        inputs.hash(&mut hash);
        scope.hash(&mut hash);
        std::mem::discriminant(&phase).hash(&mut hash);
        Self(hash.finish())
    }
}
