//! Required borrowed semantic use case; execution authority never crosses it.
pub trait OperatorsPort:
    ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase
    + Send
    + Sync
    + std::fmt::Debug
{
}
impl<
    T: ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase
        + Send
        + Sync
        + std::fmt::Debug,
> OperatorsPort for T
{
}
