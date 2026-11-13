//! The borrowed parts of one invocation's workspace a kernel works in, lent together so one evaluation can hold them at once.
use ryo_execution_domain::domain::operating::types::{Memo, NameCount};
use ryo_execution_domain::domain::operator::contract::types::{PrimitiveWorkspace, WorkCounters};
use ryo_execution_domain::domain::table_scratch::types::downstream_buffers::value::DownstreamBuffers;
use ryo_execution_domain::domain::table_scratch::types::{LazyScratch, SelectScratch};
use ryo_matching::inbound::use_cases::matching::types::CompiledRegex;

/// The working allocations of one invocation, each borrowed apart from the others: a view of the workspace, never
/// a second owner of them. The admitted publications are not lent.
pub struct WorkspaceParts<'w> {
    /// The relation owner the invocation reads through.
    pub owner: &'w crate::application::file_dag::relations::service::RelationOwner,
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
    /// The downstream operators' buffers, lent together to each downstream row.
    pub downstream: &'w mut DownstreamBuffers,
}

/// Sources are supplied only by admitted inputs, never independently by a view caller.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::workspace::types::ViewInputs;
/// fn named(_: Option<&ViewInputs<'_, '_>>) {}
/// ```
#[derive(Debug)]
pub enum ViewInputs<'a, 's> {
    Rows,
    MembershipValues,
    MembershipNative,
    /// Text cells, read through the texts.
    Text(&'a ryo_execution_domain::domain::table_texts::types::Texts<'a>),
    Dfs {
        symbols: &'a mut ryo_language::domain::symbols::types::QuerySymbols<'s>,
        source: ryo_values::domain::files::types::SourceView<'a>,
        paths: &'a [ryo_language::domain::files::types::FilePath],
        arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
    },
}
