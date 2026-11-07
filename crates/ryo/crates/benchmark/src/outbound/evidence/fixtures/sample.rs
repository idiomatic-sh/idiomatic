//! The owned sample data the evidence contract rows are written in.
use ryo_benchmark_contracts::domain::measurements::types::{Measurement, PublicationSample, SampleName};

/// Owned test data, not a certification or measurement proof.
#[derive(Clone)]
pub struct SampleInput {
    pub(super) complete_inventory: bool,
    pub(super) name: SampleName,
    pub(super) measurement: Measurement,
}
impl SampleInput {
    pub(super) fn publication(&self) -> PublicationSample<'_> {
        PublicationSample {
            complete_inventory: self.complete_inventory,
            name: &self.name,
            measurement: &self.measurement,
        }
    }
}
