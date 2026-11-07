//! The wiring of `relation describe`: the edge takes the parts its caller supplies, chooses each
//! implementation, and builds the service it calls. It only constructs: the verb opens no client, so it
//! has no connections. Nothing here implements a port.

use std::sync::Arc;

use ryo_core::inbound::use_cases::describe_relation::interface::DescribeRelationUseCase;
use ryo_core::outbound::language;

use crate::domain::language::types::Language;

/// The dependencies the caller supplies: how the service is built over the languages linked.
pub(crate) struct RelationDescribeParts<Construct> {
    /// Builds the service.
    pub service: Construct,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<Service, Construct>(parts: RelationDescribeParts<Construct>) -> Service
where
    Service: DescribeRelationUseCase,
    Construct: FnOnce() -> Service,
{
    (parts.service)()
}

/// Chooses the service over what each linked language answers for the relation. The catalog is ryo's own; the languages linked say
/// whether they read each relation.
pub(crate) fn build(languages: Vec<Language>) -> impl DescribeRelationUseCase {
    let handles = languages
        .into_iter()
        .map(language::types::LanguageHandles::from)
        .collect();
    let language = language::factory::use_case(handles);
    compose(RelationDescribeParts {
        service: move || {
            ryo_core::inbound::use_cases::describe_relation::use_case::DescribeRelationUseCaseImpl::new(
                Arc::new(language),
            )
        },
    })
}
