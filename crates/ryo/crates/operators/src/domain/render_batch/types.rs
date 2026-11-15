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

/// Interns each known rendering as derived text and emits its candidate row with it appended.
pub(crate) struct RenderMinting<'a> {
    /// The engine's driver, which reads each candidate, interns each rendering and takes each row.
    pub driver: &'a mut dyn crate::domain::contract::types::RowDriver,
    /// The candidate row being read.
    pub row: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    /// The candidate row with its rendering appended.
    pub joined: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    /// The driver's refusal, which outranks what the delivery answers.
    pub outcome: Result<(), crate::domain::contract::types::PortableOperatorFault>,
}

impl crate::domain::render_batch::types::RenderedOutput for RenderMinting<'_> {
    fn text(
        &mut self,
        position: ryo_values::domain::rows::types::RowIndex,
        text: ryo_values::domain::row_texts::types::CellText<'_>,
    ) -> Result<(), crate::domain::contract::errors::ContractError> {
        use ryo_values::domain::plan_parts::types::InputNumber;
        if self
            .driver
            .population_row(InputNumber(0), position, self.row)
            != crate::domain::contract::types::RowRead::Read
        {
            self.outcome = Err(crate::domain::contract::types::PortableOperatorFault::NotRun);
            return Err(crate::domain::contract::errors::ContractError::Interrupted);
        }
        let made = self.driver.mint(text);
        self.joined.clear();
        self.joined.extend_from_slice(self.row);
        self.joined.push(made);
        self.driver
            .emit(self.joined)
            .map_err(|_| crate::domain::contract::errors::ContractError::Interrupted)
    }
}

/// What a render lowers to: the node column and the role it is rendered in.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RenderParts {
    /// The node column.
    pub(crate) node: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The role the node is embedded in.
    pub(crate) context: ryo_language::domain::semantics::types::RenderContext,
}
