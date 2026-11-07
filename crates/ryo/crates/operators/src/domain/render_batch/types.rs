//! One provider answer per request and the engine-owned delivery of the known ones.
use crate::domain::contract::errors::ContractError;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::RowIndex;

/// What a language provider answered for one render request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderAnswer<'a> {
    /// The rendered text.
    Known(CellText<'a>),
    /// The node has no rendering here; no row follows.
    NotApplicable,
    /// The provider cannot render this syntax.
    Unsupported,
}

/// Receives the rendered text of the request at a position; the engine interns it and publishes the row.
pub trait RenderedOutput {
    /// One known rendering.
    ///
    /// # Errors
    ///
    /// The engine retained an error that stops the delivery.
    fn text(&mut self, position: RowIndex, text: CellText<'_>) -> Result<(), ContractError>;
}

/// What a language provider answered for one render request, without its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderKind {
    /// A text was rendered.
    Known,
    /// The node has no rendering here.
    NotApplicable,
    /// The provider cannot render this syntax.
    Unsupported,
}
