//! The sarif port, implemented with `serde-sarif`: a tool's log read through its model, and the printed log built
//! with its builders, one run per tool, each result's `baselineState` saying how it stands against the lock. The
//! model and its builders are the adapter's schemas' to name.

use super::constants::NOTHING;
use super::schemas::{decoded, written};
use crate::domain::judgement::types::{Judgement, Run as Judged};
use crate::domain::manifest::types::ProductRoot;
use crate::outbound::sarif::errors::SarifError;
use crate::outbound::sarif::interface::SarifPort;
use crate::outbound::sarif::types::{Read, SarifText};

/// The serde-sarif adapter.
#[derive(Debug)]
pub(crate) struct SarifSerdeSarifAdapter;

impl SarifPort for SarifSerdeSarifAdapter {
    fn read(&self, text: &SarifText) -> Result<Read, SarifError> {
        if text.0.trim().is_empty() {
            return Err(SarifError::Unparsable {
                reason: NOTHING.to_owned(),
            });
        }
        decoded(&text.0)
    }

    fn write(&self, root: &ProductRoot, runs: &[Judged], outcome: &Judgement) -> SarifText {
        written(root, runs, outcome)
    }
}
