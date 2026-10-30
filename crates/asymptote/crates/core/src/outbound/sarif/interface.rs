//! What the `sarif` port asks of the outside, in the core's own vocabulary.

use super::errors::SarifError;
use super::types::{Read, SarifText};
use crate::domain::judgement::types::{Judgement, Run};
use crate::domain::manifest::types::ProductRoot;

/// What every `sarif` adapter implements.
pub trait SarifPort: std::fmt::Debug + Send + Sync {
    /// The located results of `text` and the rules its runs say they evaluated.
    ///
    /// # Errors
    ///
    /// `Unparsable` when it is no SARIF log, empty text included; `Unlocatable` when a result's location cannot be
    /// resolved to a local file.
    fn read(&self, text: &SarifText) -> Result<Read, SarifError>;

    /// The SARIF 2.1.0 log of `outcome`, one run per tool of `runs`, its paths relative to `root`: each new violation
    /// `baselineState` `new`, each accepted one `unchanged`, each resolved record `absent`.
    fn write(&self, root: &ProductRoot, runs: &[Run], outcome: &Judgement) -> SarifText;
}
