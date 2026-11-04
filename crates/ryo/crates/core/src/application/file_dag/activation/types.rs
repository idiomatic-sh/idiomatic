//! One bounded activation template, identified by its retained request and exact eligibility.
use crate::domain::file_program::types::RequestGraph;
use std::sync::Arc;

pub(super) const RELATIONS: usize = ryo_relations::domain::relations::service::ALL.len();

/// No rows, publications, owners, or mutable execution counters enter this value.
pub(in crate::application) struct ActivatedSchedule {
    pub(in crate::application::file_dag) graph: Arc<RequestGraph>,
    pub(super) governing: Vec<bool>,
    pub(super) complete: [bool; RELATIONS],
    pub(in crate::application::file_dag) active: Vec<bool>,
    /// Complete schema/population eligibility controls retention, never scheduling.
    pub(in crate::application::file_dag) reusable: Vec<bool>,
    pub(in crate::application::file_dag) remaining: Vec<usize>,
}

/// At most one template per worker; a different request or eligibility replaces it.
#[derive(Default)]
pub(in crate::application) struct ActivationCache {
    pub(super) entry: Option<Arc<ActivatedSchedule>>,
    pub(super) governing: Vec<bool>,
}
