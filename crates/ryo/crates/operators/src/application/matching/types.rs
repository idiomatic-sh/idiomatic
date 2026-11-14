//! The types of the `matching` application operation.

use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase;

/// The matching use case a port lends, as the domain's pattern semantics.
#[derive(Clone, Copy)]
pub(crate) struct PortSemantics<'a>(pub(crate) &'a dyn MatchingUseCase);
