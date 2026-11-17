//! The refusal of the plan's growth check.

/// Why the plan's growth check refused a plan, and the one text of that refusal: the node by its number in the plan, its kind
/// and the degree in rows it grows by. The engine, which knows the rule that built each node, names it before this text.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::growth::errors::GrowthError;
/// use ryo_operators::domain::growth::types::{Degree, GrowthKind, GrowthNode};
/// let refused = GrowthError::Superlinear {
///     node: GrowthNode(2),
///     kind: GrowthKind("BandJoin"),
///     degree: Degree(2),
/// };
/// assert!(refused.to_string().starts_with("node 2 (BandJoin) pairs every row of a file"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GrowthError {
    /// A node's rows grow faster than the rows of every input it reads, and the query did not declare every pair the answer.
    #[error(
        "node {} ({}) pairs every row of a file with every row of it (grows as rows^{}): write `nearest join` to keep the \
         closest row, or `all-pairs join` to declare every pair",
        .node.0,
        .kind.0,
        .degree.0
    )]
    Superlinear {
        /// The node that causes the growth.
        node: super::types::GrowthNode,
        /// The name of its kind.
        kind: super::types::GrowthKind,
        /// The degree in rows of its output or its work.
        degree: super::types::Degree,
    },
}
