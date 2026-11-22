//! The types of the `filters` part: the parameters a fragment's conditions may name.

use crate::domain::finish_plans::expressions::types::PlanConstant;
use hashbrown::HashMap;

/// The parameters of a fragment as its caller gave them, by name: a rule has none.
#[derive(Clone, Debug, Default)]
pub(crate) struct Parameters {
    /// Each parameter's value.
    pub(crate) values: HashMap<Box<str>, PlanConstant>,
}
