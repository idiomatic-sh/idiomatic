//! The operators fakes the execution use case's `suite` boundary scenarios inject.

#[derive(Debug, Default)]
pub struct ReductionAudit {
    pub reductions: std::sync::atomic::AtomicUsize,
    pub emissions: std::sync::atomic::AtomicUsize,
}

impl crate::outbound::operators::interface::OperatorsPort for ReductionAudit {
    fn operators(
        &self,
        mut call: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
        use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
        use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "the audit counts reductions and group emissions; every other operator call passes through unchanged"
        )]
        match &mut call {
            OperatorCall::ReduceCount { .. } => {
                self.reductions
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            OperatorCall::Evaluate {
                operation:
                    ryo_values::domain::plans::types::Operation::GroupCount { .. }
                    | ryo_values::domain::plans::types::Operation::GroupDigest { .. },
                driver,
                ..
            } => {
                // One aggregate is formatted for each prepared group.
                let groups = driver.groups().map_or(0, |groups| groups.0);
                self.emissions
                    .fetch_add(groups, std::sync::atomic::Ordering::Relaxed);
            }
            _ => {}
        }
        OperatorsUseCase::operators(
            &OperatorsUseCaseImpl::new(std::sync::Arc::new(
                ryo_operators::outbound::matching::factory::use_case(),
            )),
            call,
        )
    }
}
