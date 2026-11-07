//! Required portable semantic use cases, without execution ownership.
use ryo_derived::inbound::use_cases::{
    build::interface::BuildUseCase, prepare::interface::PrepareUseCase,
    view::interface::ViewUseCase,
};

/// The three portable derived capabilities an engine needs, held together.
pub trait DerivedPort: PrepareUseCase + BuildUseCase + ViewUseCase + Send + Sync + std::fmt::Debug {}
impl<T: PrepareUseCase + BuildUseCase + ViewUseCase + Send + Sync + std::fmt::Debug> DerivedPort
    for T
{
}
