//! Limits owned by the bounded rule compilation operation.

/// How many compiled plans the use cases sharing one engine keep: above it, the plan kept longest is dropped, whatever the run in
/// progress names, so a session linting under new rule ids again and again stays bounded and concurrent runs lose one plan at a time.
pub(crate) const COMPILED_PLANS: usize = 4096;

/// The tracing target of the compiled-subplan evidence.
pub(crate) const COMPILED_SUBPLAN_TARGET: &str = "ryo::diagnostic::compiled_subplan";
/// The tracing target of the compiled-prefix evidence.
pub(crate) const COMPILED_PREFIX_TARGET: &str = "ryo::diagnostic::compiled_prefix";
/// The record kind of one rule's compiled-prefix evidence.
pub(crate) const RECORD_RULE: &str = "rule";
/// The record kind of one observed prefix.
pub(crate) const RECORD_PREFIX: &str = "prefix";
/// The record kind of one compiled plan.
pub(crate) const RECORD_PLAN: &str = "plan";
/// The record kind of one plan step.
pub(crate) const RECORD_STEP: &str = "step";
/// What the observed prefixes are normalized to.
pub(crate) const NORMALIZATION: &str = "ordered predicates and bag-preserving projections only";
/// The scope an observed prefix runs in.
pub(crate) const SCOPE_PER_FILE: &str = "per-file";
/// The name the rule compilation shows in debug output.
pub(crate) const RULE_COMPILATION_NAME: &str = "RuleCompilation";
/// Why every original rule has a plan once preparation ends.
pub(crate) const EVERY_RULE_PLANNED: &str =
    "topological preparation visits every original rule exactly once";
/// Why every original rule has bindings once preparation ends.
pub(crate) const EVERY_RULE_BOUND: &str = "each prepared rule has its own checked bindings";
/// The domain a rule's producer identities are digested under.
pub(crate) const OBSERVATION_PRODUCERS_DOMAIN: &[u8] = b"ryo-observation-producers-v1";
