//! The pure result of projecting candidate rule configuration.
/// Existing SDK hosted inputs and exact configured rule/file count.
pub(crate) struct CandidateConfiguration {
    /// Configuration tables; no acquisition, cache or engine authority.
    pub hosted: Vec<ryo::linter::HostRelation>,
    /// Exact selected pairs after per-file exclusions.
    pub selected_pairs: usize,
}
