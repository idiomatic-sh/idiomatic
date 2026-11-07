//! Required borrowed semantic use case; execution authority never crosses it.
/// What the engine asks of the operators capability: one borrowed call in, one pure meaning out.
pub trait OperatorsPort:
    ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase
    + Send
    + Sync
    + std::fmt::Debug
{
}
