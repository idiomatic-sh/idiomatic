//! A rule's file demand in every demand mode, compiled once and read by the selected mode.
use crate::domain::executable_program::file_demand::types::{
    FileDemand, FileMode, FrontierSelection, NamedOutputCount, RootSelection,
};

/// A rule's file demand in every demand mode.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::file_demand::constants::NO_FILE_DEMAND;
/// use ryo_execution_domain::domain::executable_program::types::file_rule_demand::value::FileRuleDemand;
/// fn readers(demand: &FileRuleDemand) -> usize {
///     demand.selected(NO_FILE_DEMAND).readers.len()
/// }
/// ```
#[derive(Debug)]
pub struct FileRuleDemand {
    /// How many named outputs the rule has.
    pub outputs: NamedOutputCount,
    modes: Box<[FileMode]>,
}

impl FileRuleDemand {
    /// The demand of a rule with `outputs` named outputs over its compiled modes, one per demand,
    /// in the order `selected` reads them.
    pub(crate) fn compiled(outputs: NamedOutputCount, modes: Box<[FileMode]>) -> Self {
        Self { outputs, modes }
    }
    /// The file mode the demand selects.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::executable_program::file_demand::constants::NO_FILE_DEMAND;
    /// use ryo_execution_domain::domain::executable_program::types::file_rule_demand::value::FileRuleDemand;
    /// fn readers(demand: &FileRuleDemand) -> usize {
    ///     demand.selected(NO_FILE_DEMAND).readers.len()
    /// }
    /// ```
    pub fn selected(&self, demand: FileDemand) -> &FileMode {
        let diagnostic = match demand.diagnostic {
            RootSelection::Undemanded => 0,
            RootSelection::Required => 6,
        };
        let observations = match demand.observations {
            RootSelection::Undemanded => 0,
            RootSelection::Required => 3,
        };
        let capture = match demand.capture {
            FrontierSelection::Disabled => 0,
            FrontierSelection::Observations => 1,
            FrontierSelection::All => 2,
        };
        &self.modes[diagnostic + observations + capture]
    }
}
