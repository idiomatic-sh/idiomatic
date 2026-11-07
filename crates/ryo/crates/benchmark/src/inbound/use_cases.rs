//! Auditing precedes correctness; correctness precedes measurement.
pub mod admit_measurement;
pub mod audit;
pub mod compare;
pub mod measure;
pub mod measure_corpus;
pub mod run_ryo;

pub mod certify;

/// Refuse a worker request carrying another identity than its executable's.
pub mod check_worker_request;

pub mod collect;

/// Complete pinned inventory verification.
pub mod verify_inventory;

/// Recheck one explicit receipt file through the shared policy.
pub mod validate_case_receipts;

pub mod publish;

pub mod prepare_cases;
pub mod select_cases;
pub mod stage_suite;
pub mod verify_cases;
pub mod verify_reference_cases;
