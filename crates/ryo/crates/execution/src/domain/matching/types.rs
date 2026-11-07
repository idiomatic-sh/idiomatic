//! Immutable matcher metadata prepared from the finite authored request.

pub(crate) mod prepared;

/// The matching semantics a parent hands down: the domain compiles and matches through it, and builds none.
#[derive(Clone, Copy)]
pub(crate) struct PatternMatcher<'a>(pub &'a dyn ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase);

impl std::fmt::Debug for PatternMatcher<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
