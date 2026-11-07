//! The command line vocabulary of the offline case-receipt check.
/// The command word that selects the check.
pub const COMMAND: &str = "validate-case-receipts";
/// What the command takes.
pub const USAGE: &str = "usage: validate-case-receipts <registration.json> <receipt.jsonl>";
/// The refusal when any case failed strict verification.
pub const FAILED: &str = "case receipts failed strict verification";
