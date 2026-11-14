//! Diagnostic vocabulary of the executable program owner.
pub(super) const WALK_TARGET: &str = "ryo::diagnostic::file_program";
pub(super) const WALK_CENSUS: &str = "shared_walk_census";
pub(super) const WALK_CLASS: &str = "shared_walk_class";
pub(super) const WALK_ROUTE: &str = "shared_walk_route";
pub(super) const WALK_DISPATCH: &str = "shared_walk_dispatch";
pub(super) const WALK_REFUSED: &str = "shared_walk_refused";
pub(super) const EXECUTION_TARGET: &str =
    ryo_values::domain::table_words::constants::EXECUTION_TARGET;
pub(super) const INPUT_ADMISSION_REFUSED: &str = "input_admission_refused";
pub(super) const PHASE_DEPENDENCY: &str = "dependency";
pub(super) const PHASE_AVAILABILITY: &str = "availability";
pub(super) const PHASE_PHYSICAL: &str = "physical";
pub(super) const READINESS_QUEUE: &str = "readiness_queue";
pub(super) const REGION_FAILURE: &str = "region_failure";
pub(super) const REGION_POLL: &str = "region_poll";
pub(super) const OUTCOME_MISSING: &str = "missing";
pub(super) const OUTCOME_RETRY: &str = "retry";
pub(super) const OUTCOME_SUSPENDED: &str = "suspended";
pub(super) const OUTCOME_READY: &str = "ready";
pub(super) const OUTCOME_RUNNING: &str = "running";
pub(super) const OUTCOME_PUBLISHED: &str = "published";
pub(super) const OUTCOME_RETIRED: &str = "retired";
pub(super) const OUTCOME_FAILED: &str = "failed";
pub(super) const BOUND_PRODUCER_UNAVAILABLE: &str = "declared bound producer is unavailable";
pub(super) const OBSERVATION_UNAVAILABLE: &str = "declared observation publication is unavailable";
pub(super) const PRODUCER_WITHOUT_QUERY: &str = "named producer has no executable query";
pub(super) const PRODUCER_ROOT_INCOMPLETE: &str = "named producer root is not complete";
pub(super) const NO_EXECUTABLE_QUERY: &str = "named producer has no executable query";
pub(super) const PACKED_READINESS_DISAGREES: &str =
    "packed readiness disagrees with the admitted producer coverage";
pub(super) const RESTORED_FRAME_REJECTED: &str =
    "restored semantic frame belongs to a currently syntax-rejected producer partition";
