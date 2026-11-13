//! The names and refusals of the registrar.
/// The staged context of one rule, below its output.
pub(super) const REPRO: &str = "repro";
/// The keyword that starts an import line.
pub(super) const IMPORT: &str = "import";
/// The keyword before an imported module.
pub(super) const FROM: &str = "from";
/// How far above a bench (`crates/ryo/languages/<lang>/benchmarks/linters/<linter>`) its repository is.
pub(super) const REPOSITORY_DEPTH: usize = 7;
/// The identity key of the bench profile's digest.
pub(super) const INVENTORY_SHA256: &str = "inventory_sha256";
/// The reason of a default without a definition.
pub(super) const MISSING_IMPLEMENTATION: &str = "missing implementation";
/// The prefix of the reason of a rule that could not be staged.
pub(super) const RULE_REGISTRATION: &str = "rule registration";
/// The refusal of a case file without cases or with a repeated name.
pub(super) const CASES_EMPTY_OR_DUPLICATE: &str = "canonical case registration is empty or has duplicate names";
/// The refusal of a settings profile pinned to another tool.
pub(super) const PINS_DIFFER: &str = "per-rule profile does not retain the pinned tool identity";
/// The refusal of a settings profile that does not select the rule.
pub(super) const NOT_SELECTED: &str = "settings registration does not select this rule";
/// The refusal of a cohort rule without a definition.
pub(super) const NO_DEFINITION: &str = "supporting rule has no definition";
/// The refusal of an import no profile declares.
pub(super) const UNDECLARED_MODULE: &str = "undeclared canonical helper";
/// The refusal of a cohort rule without a declared applicability.
pub(super) const NO_APPLICABILITY: &str = "no declared applicability for registered rule";
/// The refusal of one evidence path declared with two digests.
pub(super) const CONFLICTING_EVIDENCE: &str = "conflicting discrepancy evidence declarations";
/// The refusal of unsafe, missing or stale declared evidence.
pub(super) const EVIDENCE_REFUSED: &str = "discrepancy evidence refused";
/// The refusal of evidence that would overwrite a staged input.
pub(super) const EVIDENCE_COLLIDES: &str = "discrepancy evidence collides with staged input";
/// The quote around an imported module.
pub(super) const QUOTE: char = '"';
/// The end of an import line.
pub(super) const END_OF_IMPORT: char = ';';
/// The character a module name may hold beside letters and digits.
pub(super) const UNDERSCORE: char = '_';
/// The directory of a bench's per-rule settings profiles, and of each rule's stage below the output.
pub(super) const SETTINGS_PROFILES: &str = "profiles";
