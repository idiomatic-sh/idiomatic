//! The input lists of a logical program's nodes, one per node, held by the caller for as long as the program borrows them.

use crate::domain::executable_program::types::SourceAbsence;
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
    /// Scope of the prepared variant; callers may execute it only after this file proof.
    pub(crate) absence: Option<SourceAbsence>,
    /// Relation whose unique selected and union roots the program may share.
    pub(crate) extended: Option<ryo_relations::domain::relations::types::Relation>,
    /// Each node's inputs, in the order its kind reads them.
    pub(crate) lists: Vec<Vec<PlanInput>>,
    /// The columns of each relation the host declares, by its place: a hosted step's schema.
    pub(crate) hosted: Vec<Vec<CellKind>>,
}

impl ProgramInputs {
    /// Prepare under a complete file-source condition. The unconditioned plan remains the fallback.
    pub fn assuming(
        &mut self,
        absence: Option<SourceAbsence>,
        extended: Option<ryo_relations::domain::relations::types::Relation>,
    ) {
        self.absence = absence;
        self.extended = extended;
    }

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
            absence: None,
            extended: None,
            lists: Vec::new(),
            hosted,
        }
    }
}
