//! Checked conversion of the collector's typed per-child resource record.
use ryo_benchmark_contracts::domain::measurements::types::Resources;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
/// Per-child clocks and peak resident bytes returned by pinned `wait4`.
struct ResourcesSchema(
    /// Native resource record collected for exactly one waited child.
    wait4::ResourceUsage,
);
impl TryFrom<ResourcesSchema> for Resources {
    type Error = Error;
    fn try_from(ResourcesSchema(record): ResourcesSchema) -> Result<Self, Error> {
        let cpu = record
            .utime
            .checked_add(record.stime)
            .ok_or_else(|| Error::Invalid(super::constants::CPU.into()))?;
        Ok(Resources {
            cpu,
            peak_rss_bytes: ryo_benchmark_contracts::domain::measurements::types::ResidentBytes(
                record.maxrss,
            ),
        })
    }
}
/// Sum child CPU durations with overflow refusal and preserve peak resident bytes.
pub(super) fn decode(record: wait4::ResourceUsage) -> Result<Resources, Error> {
    ResourcesSchema(record).try_into()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
