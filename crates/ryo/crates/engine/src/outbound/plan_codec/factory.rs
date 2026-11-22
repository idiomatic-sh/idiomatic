//! Builds the plan codec adapters, one function each. Called only by an edge.

/// The JSON codec: the plan format of record.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::types::FinishPlan;
/// use ryo_engine::outbound::plan_codec::factory::json;
/// use ryo_engine::outbound::plan_codec::interface::PlanCodecPort;
///
/// let empty = FinishPlan { steps: Vec::new(), rules: Vec::new(), unplanned: Vec::new() };
/// assert_eq!(json().encoded(&empty), br#"{"nodes": [], "emits": {}, "failed": {}}"#.to_vec());
/// ```
#[must_use]
pub fn json() -> impl crate::outbound::plan_codec::interface::PlanCodecPort {
    super::adapters::json::adapter::PlanCodecJsonAdapter
}
