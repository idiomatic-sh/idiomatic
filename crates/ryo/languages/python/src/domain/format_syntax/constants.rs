//! Canonical percent flag spellings in ascending ASCII mask order.
pub(crate) const FLAG_SPELLINGS: [&str; 32] = [
    "", " ", "#", " #", "+", " +", "#+", " #+", "-", " -", "#-", " #-", "+-", " +-", "#+-", " #+-",
    "0", " 0", "#0", " #0", "+0", " +0", "#+0", " #+0", "-0", " -0", "#-0", " #-0", "+-0", " +-0",
    "#+-0", " #+-0",
];

/// In-process memo mode; the selected parser type supplies implementation identity.
pub(crate) const DECODED_GRAMMAR: super::types::GrammarIdentity =
    super::types::GrammarIdentity("percent-decoded");
/// Raw token bodies use an independent memo mode from decoded string values.
pub(crate) const RAW_GRAMMAR: super::types::GrammarIdentity =
    super::types::GrammarIdentity("percent-raw");
