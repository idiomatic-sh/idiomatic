//! The test data of `linter lint` that fixes capability provider.

use crate::domain::capacity::types::MemoryBudget;
use crate::domain::capacity::types::WorkerPool;
use crate::domain::language::types::Language;
use crate::domain::language::types::LanguageBuild;
use crate::domain::language::types::LanguageName;
use crate::domain::language::types::Languages;
use crate::resources::linter::lint::types::Cache;
use crate::resources::linter::lint::types::Linter;
use crate::resources::linter::lint::types::Workers;
use ryo_relations::domain::catalog::types::OptionalCapability;
use std::num::NonZeroUsize;

/// A provider with the same name/build whose advertised optional contract changes.
pub(crate) struct CapabilityProvider(
    pub(crate) &'static [ryo_relations::domain::catalog::types::OptionalCapability],
    pub(crate) &'static str,
);

impl ryo_language::inbound::use_cases::parse::types::ParseEngine for CapabilityProvider {
    fn capabilities(&self) -> Vec<OptionalCapability> {
        self.0.to_vec()
    }
    fn configuration_identity(
        &self,
    ) -> ryo_language::inbound::use_cases::parse::types::ParseConfigurationIdentity<'_> {
        ryo_language::inbound::use_cases::parse::types::ParseConfigurationIdentity(self.1)
    }
    fn recognition(
        &self,
        _: &ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
    ) -> ryo_language::inbound::use_cases::parse::types::Recognition {
        ryo_language::inbound::use_cases::parse::types::Recognition::Reads
    }
    fn skips(&self) -> &'static [ryo_language::inbound::use_cases::parse::types::SkippedDirectory] {
        &[]
    }
    fn kind_count(&self) -> ryo_relations::domain::relations::types::Count {
        ryo_relations::domain::relations::types::Count(1)
    }
    fn kind_names(&self) -> ryo_language::inbound::use_cases::parse::types::KindNames {
        ryo_language::inbound::use_cases::parse::types::KindNames(vec![
            ryo_relations::domain::terms::types::NameText("root"),
        ])
    }
    fn unknown_reasons(&self) -> &'static [ryo_relations::domain::terms::types::UnknownName] {
        &[]
    }
}

/// A linter over one fixture provider, retaining into `cache`: `build` and `identity` are what its cache identity is made of, `present`
/// the name its identifier answers.
pub(crate) fn provider_linter(
    build: &str,
    identity: &'static str,
    capabilities: &'static [OptionalCapability],
    present: &'static str,
    cache: Cache,
) -> Linter {
    Linter::new(
        Languages(vec![Language::from_parts(
            LanguageName("fixture".into()),
            LanguageBuild(build.into()),
            std::sync::Arc::new(CapabilityProvider(capabilities, identity)),
            std::sync::Arc::new(
                ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl,
            ),
        )]),
        WorkerPool::from(Workers(NonZeroUsize::MIN)),
        MemoryBudget::Machine,
        cache,
    )
}
