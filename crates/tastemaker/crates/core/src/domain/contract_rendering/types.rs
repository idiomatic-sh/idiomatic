//! The types of the `contract_rendering` concept: plain data, public fields and derives, with no constructor and no
//! inherent impl.

use tastemaker_values::domain::contexts::types::RenderContext;
use tastemaker_values::domain::contracts::types::TemplateIdentity;
use tastemaker_values::domain::definitions::types::{Exactness, TemplateText};
use tastemaker_values::domain::findings::types::Finding;
use tastemaker_values::domain::paths::types::RepoPath;

/// A placed file's template content, with the context its placement gives it: what the template engine renders.
#[derive(Clone, Debug)]
pub struct ContractRequest {
    /// The placed file.
    pub target: RepoPath,
    /// Where the rendered content is given to the lint call.
    pub rendered: RepoPath,
    /// The template entry that placed the file, by its identity.
    pub identity: TemplateIdentity,
    /// Whether the file holds exactly the content.
    pub exact: Exactness,
    /// The content to render.
    pub content: TemplateText,
    /// What it is rendered with.
    pub context: RenderContext,
}

/// What a placed file asks of the template engine: a content to render, or the `#template` error of a content that
/// cannot be given its context.
#[derive(Clone, Debug)]
pub enum Requested {
    /// A content to render.
    Render(Box<ContractRequest>),
    /// A param with no value: the content is not rendered.
    Failed(Finding),
}

/// What rendering a content made of a placed file.
#[derive(Clone, Debug)]
pub enum Rendered {
    /// The contract the file is compared against.
    Contract(tastemaker_values::domain::contracts::types::Contract),
    /// The content rendered to nothing: it asks nothing of the file.
    Nothing,
    /// The content did not render.
    Failed(Finding),
}
