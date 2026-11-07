//! The operator port the union proof counts its semantic set calls through.
use super::super::*;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::set_rows::types::SetInput;
use ryo_operators::domain::set_rows::types::SetKind;
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;

#[derive(Debug, Default)]
pub(super) struct UnionSemanticCalls(pub(super) AtomicUsize);
impl ryo_execution::outbound::operators::interface::OperatorsPort for UnionSemanticCalls {
    fn operators(
        &self,
        call: OperatorCall<'_>,
    ) -> Result<OperatorAnswer, PortableOperatorFault> {
        if matches!(
            &call,
            OperatorCall::SetRow {
                input: SetInput::Values {
                    kind: SetKind::Union,
                    ..
                }
            } | OperatorCall::SetRow {
                input: SetInput::Positions {
                    kind: SetKind::Union,
                    ..
                }
            }
        ) {
            let _previous = self.0.fetch_add(1, Ordering::SeqCst);
        }
        OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())).operators(call)
    }
}
