//! Required portable semantic use case, without execution ownership.
pub trait DerivedPort:
    ryo_derived::inbound::use_cases::derived::interface::DerivedUseCase + Send + Sync + std::fmt::Debug
{
}
impl<
    T: ryo_derived::inbound::use_cases::derived::interface::DerivedUseCase
        + Send
        + Sync
        + std::fmt::Debug,
> DerivedPort for T
{
}
