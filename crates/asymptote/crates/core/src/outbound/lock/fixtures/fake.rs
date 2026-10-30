//! The fake of the `lock` port: operations kept in memory, addressed by their order.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::domain::manifest::types::Pathspec;
use crate::domain::operations::types::{Operation, OperationId, OperationText};
use crate::domain::violations::types::{Id, Key, Standing};
use crate::outbound::lock::errors::LockError;
use crate::outbound::lock::interface::LockPort;
use crate::outbound::lock::types::{Listing, Operations};

/// What the core's tests stand in for `lock` with.
#[derive(Debug, Default)]
pub struct LockFake {
    /// The operations, by id.
    pub operations: Mutex<BTreeMap<OperationId, Operation>>,
    /// The operations other revisions hold, by id, which `parse` answers.
    pub elsewhere: BTreeMap<OperationId, Operation>,
    /// How many operations were ever appended, which numbers the next.
    pub appended: std::sync::atomic::AtomicUsize,
}

impl LockFake {
    /// The operations, read.
    #[must_use]
    pub fn held(&self) -> BTreeMap<OperationId, Operation> {
        self.operations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl LockPort for LockFake {
    fn listing(&self) -> Result<Listing, LockError> {
        Ok(Listing {
            files: self.held().into_keys().collect(),
        })
    }

    fn read(&self) -> Result<Operations, LockError> {
        Ok(Operations(self.held()))
    }

    fn parse(&self, id: &OperationId, _text: &OperationText) -> Result<Operation, LockError> {
        self.elsewhere
            .get(id)
            .cloned()
            .ok_or_else(|| LockError::Address {
                expected: id.0.to_string(),
                actual: String::new(),
            })
    }

    fn append(&self, operation: &Operation) -> Result<(), LockError> {
        let mut held = self
            .operations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if held.values().any(|kept| kept == operation) {
            return Ok(());
        }
        let appended = self
            .appended
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .saturating_add(1);
        // The id counts the operations ever appended, so it is never one already taken.
        held.extend([(
            OperationId(format!("{appended:064}").into()),
            operation.clone(),
        )]);
        Ok(())
    }

    fn compact(&self, snapshot: &Operation) -> Result<(), LockError> {
        self.append(snapshot)?;
        self.operations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|id, _| !snapshot.covers.contains(id));
        Ok(())
    }

    fn directory(&self) -> Pathspec {
        Pathspec(".asymptote/operations".into())
    }

    fn id(&self, key: &Key) -> Id {
        let standing = match key.standing {
            Standing::Reported => "",
            Standing::Suppressed => ":suppressed",
        };
        Id(format!(
            "{}:{}:{}:{:?}:{:016x}{standing}",
            key.tool.0, key.rule.0, key.path.0, key.level, key.fingerprint.0
        )
        .into())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
