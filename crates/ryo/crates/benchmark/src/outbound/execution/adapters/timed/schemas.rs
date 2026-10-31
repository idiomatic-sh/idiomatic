//! Checked conversion of the collector's typed per-child resource record.
use crate::domain::{diagnostics::errors::Error, measurements::types::Resources};
/// Per-child clocks and peak resident bytes returned by pinned wait4.
struct ResourcesSchema(pub(super) wait4::ResourceUsage);
impl TryFrom<ResourcesSchema> for Resources {
    type Error = Error;
    fn try_from(ResourcesSchema(record): ResourcesSchema) -> Result<Self, Error> {
        let cpu = record
            .utime
            .checked_add(record.stime)
            .ok_or_else(|| Error::Invalid(super::constants::CPU.into()))?;
        Ok(Resources {
            cpu,
            peak_rss_bytes: record.maxrss,
        })
    }
}
pub(super) fn decode(record: wait4::ResourceUsage) -> Result<Resources, Error> {
    ResourcesSchema(record).try_into()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
