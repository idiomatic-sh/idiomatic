//! Closed, ordered demand code interpreted only by the common application invocation.
use super::super::{DemandAction, InputId};
use crate::domain::operator::contract::types::{DemandOrder, SemanticContract};
use ryo_values::domain::plan_parts::types::InputCount;

/// The ordered admissions of one instruction's inputs.
#[derive(Clone, Debug)]
pub struct DemandProgram {
    actions: Box<[DemandAction]>,
}

impl DemandProgram {
    /// The admissions the compiler orders for one instruction.
    pub(in crate::domain::executable_program) fn new(actions: Box<[DemandAction]>) -> Self {
        Self { actions }
    }
    /// The admissions an operator's semantic contract orders over its `used` inputs. Explicit validation and fault points
    /// remain code, not implicit runtime defaults.
    pub(in crate::domain::executable_program) fn admissions(
        contract: SemanticContract,
        InputCount(used): InputCount,
    ) -> Vec<DemandAction> {
        match contract.demand {
            DemandOrder::None => Vec::new(),
            DemandOrder::Unary => vec![DemandAction::Admit(InputId(0))],
            DemandOrder::LeftThenRight => (0..used)
                .map(|input| DemandAction::Admit(InputId(input)))
                .collect(),
        }
    }
    /// The demand program of an operator's semantic contract over its `used` inputs.
    pub(in crate::domain::executable_program) fn compiled(
        contract: SemanticContract,
        used: InputCount,
    ) -> Self {
        Self::new(Self::admissions(contract, used).into_boxed_slice())
    }
    /// The admissions, in order.
    pub fn actions(&self) -> &[DemandAction] {
        &self.actions
    }
}
