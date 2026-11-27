//! The types of the `filters` part: the parameters a program's conditions may name.

use ryo_values::domain::finish_plans::expressions::types::PlanConstant;
use hashbrown::HashMap;

/// The parameter values supplied to a rule or a fragment, by declared name.
#[derive(Clone, Debug, Default)]
pub(crate) struct Parameters {
    /// Each parameter's value.
    pub(crate) values: HashMap<Box<str>, PlanConstant>,
}
