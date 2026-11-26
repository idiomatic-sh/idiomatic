//! The `parse` use case here is the language that reads nothing: it recognizes no file and answers every
//! relation as not in it. The contract runs against it, and a real language replaces it with its own.

use super::constants::NO_CONFIGURATION;
use super::interface::ParseUseCase;
use super::types::{KindNames, ParseConfigurationIdentity, ParseEngine, ParseRequest, SkippedDirectory};
use crate::inbound::use_cases::parse::types::Recognition;
use ryo_relations::domain::relations::types::Count;
use ryo_relations::domain::terms::types::UnknownName;

#[derive(Clone, Copy, Debug)]
pub struct ParseUseCaseImpl;

impl ParseUseCase for ParseUseCaseImpl {
    fn capabilities(&self) -> &'static [ryo_relations::domain::catalog::types::OptionalCapability] {
        &[]
    }

    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        ParseConfigurationIdentity(NO_CONFIGURATION)
    }

    fn recognizes_explicit(&self, _request: &ParseRequest<'_>) -> Recognition {
        Recognition::Declines
    }

    fn recognizes(&self, _request: &ParseRequest<'_>) -> Recognition {
        Recognition::Declines
    }

    fn skips(&self) -> &'static [SkippedDirectory] {
        &[]
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
    }

    fn kind_count(&self) -> Count {
        Count(0)
    }

    fn kind_names(&self) -> KindNames {
        KindNames(Vec::new())
    }
}

impl<P: ParseUseCase> ParseEngine for P {
    fn source_words(&self) -> Option<super::named::SourceWords> {
        ParseUseCase::source_words(self)
    }

    fn file_relations(&self) -> Option<&dyn super::named::FileRelations> {
        ParseUseCase::file_relations(self)
    }

    fn capabilities(&self) -> Vec<ryo_relations::domain::catalog::types::OptionalCapability> {
        ParseUseCase::capabilities(self).to_vec()
    }

    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        ParseUseCase::configuration_identity(self)
    }

    fn recognition(&self, request: &ParseRequest<'_>) -> Recognition {
        ParseUseCase::recognizes(self, request)
    }

    fn skips(&self) -> &'static [SkippedDirectory] {
        ParseUseCase::skips(self)
    }

    fn kind_count(&self) -> Count {
        ParseUseCase::kind_count(self)
    }

    fn kind_names(&self) -> KindNames {
        ParseUseCase::kind_names(self)
    }

    fn stable_fields(&self) -> crate::domain::fillers::types::FieldNames {
        ParseUseCase::stable_fields(self)
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        ParseUseCase::unknown_reasons(self)
    }
}
