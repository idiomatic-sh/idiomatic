//! The vocabulary of one complete inventory verification.
/// Text of the unknown or duplicate registered rule in this part.
pub(super) const UNKNOWN_OR_DUPLICATE_REGISTERED_RULE: &str =
    "unknown or duplicate registered rule";
/// Text of the profiles in this part.
pub(super) const PROFILES: &str = "profiles";
/// Text of the missing implementation registration in this part.
pub(super) const MISSING_IMPLEMENTATION_REGISTRATION: &str = "missing implementation registration";
/// Text of the missing implementation invocation in this part.
pub(super) const MISSING_IMPLEMENTATION_INVOCATION: &str = "missing implementation invocation";
/// Text of the executable rule has no tested definition in this part.
pub(super) const EXECUTABLE_RULE_HAS_NO_TESTED_DEFINITION: &str =
    "executable rule has no tested definition pin";
/// Text of the query module pins are outside the in this part.
pub(super) const QUERY_MODULE_PINS_ARE_OUTSIDE_THE: &str =
    "query/module pins are outside the registered case context";
/// Text of the missing independent cases in this part.
pub(super) const MISSING_INDEPENDENT_CASES: &str = "missing independent cases";
/// Text of the case identity belongs to another registered in this part.
pub(super) const CASE_IDENTITY_BELONGS_TO_ANOTHER_REGISTERED: &str =
    "case identity belongs to another registered rule";
/// Text of the case verification requires execution authori in this part.
pub(super) const CASE_VERIFICATION_REQUIRES_EXECUTION_AUTHORI: &str =
    "case verification requires execution authority";
/// Text of the native receipt digest is missing in this part.
pub(super) const NATIVE_RECEIPT_DIGEST_IS_MISSING: &str = "native receipt digest is missing";
/// Text of the receipt only inventory cannot authorize exec in this part.
pub(super) const RECEIPT_ONLY_INVENTORY_CANNOT_AUTHORIZE_EXEC: &str =
    "receipt-only inventory cannot authorize execution";
/// Text of the canonical inventory profile executable ident in this part.
pub(super) const CANONICAL_INVENTORY_PROFILE_EXECUTABLE_IDENT: &str =
    "canonical inventory/profile/executable identity differs";
/// A registered rule without a case-verification command cannot be verified.
pub(super) const MISSING_VERIFICATION_COMMAND: &str = "missing verification command";
/// The registered module names must be exactly the pinned ones.
pub(super) const REGISTERED_MODULE_INVENTORY_DIFFERS_FROM_PIN: &str =
    "registered module inventory differs from pinned context profile";
/// Every module the context declares needs a pin.
pub(super) const MISSING_DECLARED_MODULE_PIN: &str = "missing declared module pin";
/// A registered case context lists no artifact at all.
pub(super) const REGISTERED_CASE_CONTEXT_IS_EMPTY: &str = "registered case context is empty";
/// The files of the registered context must be exactly the pinned artifacts.
pub(super) const REGISTERED_CONTEXT_NAMESPACE_DIFFERS_FROM_PI: &str =
    "registered context namespace differs from pinned artifacts";
