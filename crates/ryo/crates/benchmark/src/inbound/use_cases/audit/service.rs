//! Default coverage is a prerequisite for any performance claim.
use crate::domain::diagnostics::errors::Error;
use crate::domain::suite::types::Audit;
pub fn require_complete(audit: &Audit) -> Result<(), Error> {
    if audit.missing.is_empty() {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "default coverage incomplete: {} missing rules: {}",
            audit.missing.len(),
            audit.missing.join(", ")
        )))
    }
}
