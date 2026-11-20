//! The rule sets of a run of the new engine, loaded once before its files: read back from the store's packs of what the run was
//! given, which is asked before anything is compiled, or lowered from the compiled rules; and what the rules a rule set does
//! not run could not decide.
pub(crate) mod service;
