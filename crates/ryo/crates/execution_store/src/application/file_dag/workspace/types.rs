//! The borrowed parts of one invocation's workspace a kernel works in, lent together so one evaluation can hold them at once.
use ryo_execution_domain::domain::operating::types::{Memo, NameCount};
use ryo_execution_domain::domain::operator::contract::types::{PrimitiveWorkspace, WorkCounters};
use ryo_execution_domain::domain::table_scratch::types::{
    LazyScratch, OperatorScratch, SelectScratch,
};
use ryo_matching::inbound::use_cases::matching::types::CompiledRegex;

/// The working allocations of one invocation, each borrowed apart from the others: a view of the workspace, never
/// a second owner of them. The admitted publications are not lent.
pub struct WorkspaceParts<'w> {
    /// The relation owner the invocation reads through.
    pub owner: &'w crate::application::file_dag::relations::RelationOwner,
    /// The rows each input was found to hold.
    pub work: &'w WorkCounters,
    /// The literal pattern compiled for this invocation, once it needed one.
    pub literal_regex: &'w mut Option<CompiledRegex>,
    /// The predicates' lazily prepared scratch.
    pub predicates: &'w mut LazyScratch<SelectScratch>,
    /// The predicates' memo of decided rows.
    pub predicate_memo: &'w mut Vec<Memo>,
    /// The names the predicates counted.
    pub predicate_names: &'w mut Option<NameCount>,
    /// The primitive operators' working allocations.
    pub primitive: &'w mut PrimitiveWorkspace,
    /// The downstream operators' scratch.
    pub downstream_scratch: &'w mut OperatorScratch,
    /// The downstream operators' memo.
    pub downstream_memo: &'w mut Vec<Memo>,
}
