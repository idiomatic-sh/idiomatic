//! The input lists of a logical program's nodes, one per node, held by the caller for as long as the program borrows them.

use ryo_operators::domain::physical_plan::types::PlanInput;
use ryo_relations::domain::relations::types::CellKind;

/// The input lists of a logical program's nodes, in node order.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// use ryo_execution_domain::domain::executable_program::service::logical;
/// let mut inputs = ProgramInputs::default();
/// assert!(logical(&[], &mut inputs).is_ok());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProgramInputs {
    /// Each node's inputs, in the order its kind reads them.
    pub(crate) lists: Vec<Vec<PlanInput>>,
    /// The columns of each relation the host declares, by its place: a hosted step's schema.
    pub(crate) hosted: Vec<Vec<CellKind>>,
}

impl ProgramInputs {
    /// Lists for a request whose host declares relations of `hosted`'s columns, by place.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
    /// use ryo_execution_domain::domain::executable_program::service::logical;
    /// use ryo_relations::domain::relations::types::CellKind;
    /// let mut inputs = ProgramInputs::hosting(vec![vec![CellKind::File]]);
    /// assert!(logical(&[], &mut inputs).is_ok());
    /// ```
    #[must_use]
    pub fn hosting(hosted: Vec<Vec<CellKind>>) -> Self {
        Self {
            lists: Vec::new(),
            hosted,
        }
    }
}
