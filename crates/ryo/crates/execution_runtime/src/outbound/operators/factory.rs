//! Compose the portable operator adapter at an SDK edge.
use crate::outbound::operators::adapters::use_case::adapter::OperatorsUseCaseAdapter;
use crate::outbound::operators::interface::OperatorsPort;

/// The adapter over the operators use cases; the edge constructs it once per lint service.
///
/// # Examples
///
/// ```
/// use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
/// use ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
/// use ryo_operators::inbound::use_cases::run::use_case::RunUseCaseImpl;
/// let matching = || std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case());
/// let _port = ryo_execution_runtime::outbound::operators::factory::use_case(
///     OperatorsUseCaseImpl::new(matching()),
///     PlanUseCaseImpl::new(),
///     RunUseCaseImpl::new(matching()),
/// );
/// ```
#[must_use]
pub fn use_case(
    operators: impl ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase
    + Send
    + Sync
    + std::fmt::Debug,
    planner: impl ryo_operators::inbound::use_cases::plan::interface::PlanUseCase
    + Send
    + Sync
    + std::fmt::Debug,
    runner: impl ryo_operators::inbound::use_cases::run::interface::RunUseCase
    + Send
    + Sync
    + std::fmt::Debug,
) -> impl OperatorsPort {
    OperatorsUseCaseAdapter::new(operators, planner, runner)
}
