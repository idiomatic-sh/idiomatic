//! The plan codec port, implemented with JSON.

use super::formatter::PythonFormatter;
use super::schemas::PlanSchema;
use crate::domain::finish_plans::types::FinishPlan;
use crate::outbound::plan_codec::errors::PlanCodecError;
use crate::outbound::plan_codec::interface::PlanCodecPort;
use serde::Serialize;

/// The JSON plan codec.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PlanCodecJsonAdapter;

impl PlanCodecPort for PlanCodecJsonAdapter {
    fn encoded(&self, plan: &FinishPlan) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut serializer = serde_json::Serializer::with_formatter(&mut bytes, PythonFormatter);
        // A plan's schema holds only strings, numbers, lists and string-keyed maps: writing it to memory cannot fail.
        if PlanSchema::from(plan).serialize(&mut serializer).is_err() {
            bytes.clear();
        }
        bytes
    }

    fn decoded(&self, bytes: &[u8]) -> Result<FinishPlan, PlanCodecError> {
        serde_json::from_slice::<PlanSchema>(bytes)
            .map(FinishPlan::from)
            .map_err(|error| PlanCodecError::Undecoded(error.to_string()))
    }
}
