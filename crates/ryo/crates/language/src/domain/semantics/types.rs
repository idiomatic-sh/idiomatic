//! The syntax roles and outcomes shared by language providers and their consumers.

use super::errors::SemanticsError;
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::NodeId;
use ryo_relations::domain::terms::types::UnknownReason;
use std::sync::Arc;

/// What a language answers of the optional semantic capabilities (`OptionalCapability`) beyond its syntax. A language that answers none
/// of them implements no `Semantics` and says so once, with `None` from `ParseUseCase::semantics`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::semantics::types::Semantics;
///
/// let answers: Option<&dyn Semantics> = None;
/// assert!(answers.is_none());
/// ```
pub trait Semantics: Send + Sync {}

/// A language's private rendering state. The common application owns its lifetime and cache.
pub trait SemanticSession: Send {
    /// Answer a batch in request order, atomically. A successful answer has exactly one result per request.
    ///
    /// # Errors
    ///
    /// A stale or foreign node refuses the file; a partial vector is never returned.
    fn render_batch(
        &mut self,
        source: ByteSlice<'_>,
        requests: &[RenderRequest],
    ) -> Result<Vec<RenderOutcome>, SemanticsError>;
}

/// The place the engine keeps the session a language begins for a file: empty until the language fills it.
#[derive(Default)]
pub enum SemanticSlot {
    /// No language has begun a session for the file.
    #[default]
    Empty,
    /// What the language kept of its read of the file for a session it may begin, its parsed tree: the session takes it and
    /// never parses the file again. Only the language that kept it knows what it is.
    Retained(Box<dyn std::any::Any + Send>),
    /// The session the language begun.
    Begun(Box<dyn SemanticSession>),
}

impl From<SemanticSlot> for Option<Box<dyn SemanticSession>> {
    fn from(slot: SemanticSlot) -> Self {
        match slot {
            SemanticSlot::Begun(session) => Some(session),
            SemanticSlot::Empty | SemanticSlot::Retained(_) => None,
        }
    }
}

/// The role in which a rendered syntax node will be embedded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenderContext {
    /// A target on the left of an assignment.
    AssignmentTarget,
    /// The receiver before an attribute access.
    AttributeReceiver,
    /// An argument in a call's positional argument list.
    PositionalArgument,
}

/// One syntax node and the embedding role for which its text is requested.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderRequest {
    /// The node in the current file's element table.
    pub node: NodeId,
    /// The syntax role of the resulting text.
    pub context: RenderContext,
}

/// The exact text of a node rendered in a requested context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedNode(pub Arc<str>);

/// What the language can prove about rendering one requested node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderOutcome {
    /// Exact text in the requested context; the empty string remains a known answer.
    Known(RenderedNode),
    /// This node cannot occur in the requested syntax role.
    NotApplicable,
    /// Valid syntax for which the provider cannot prove exact rendered text.
    Unsupported(UnknownReason),
}
