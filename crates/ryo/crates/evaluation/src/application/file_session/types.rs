//! The types of a file's session.

use crate::domain::fused::types::Fused;
use ryo_matching::inbound::use_cases::matching::types::CompiledRegex;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;

/// What every file's session shares: the fused program, its regular expressions compiled once, and the meanings it calls.
#[derive(Debug)]
pub struct Program {
    /// The fused program.
    pub fused: Fused,
    /// The compiled pattern of each `replace` and `source-matches` step, by its node; the refusal of one that does not compile.
    pub regexes: Vec<Option<Result<CompiledRegex, String>>>,
    /// The operators' row meanings: occurrences, captures, replacements.
    pub operators: OperatorsUseCaseImpl,
    /// The pattern semantics comparisons are decided with.
    pub matching: MatchingUseCaseImpl,
}
