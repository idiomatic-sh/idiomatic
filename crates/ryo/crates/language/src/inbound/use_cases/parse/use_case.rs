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

/// The file relations of the language that reads nothing: it declares none and lends no file.
impl super::named::FileRelations for ParseUseCaseImpl {
    fn numbered(&self) -> &dyn super::named::NumberedVisit {
        self
    }

    fn declarations(&self) -> &'static [super::named::FileRelation] {
        &[]
    }

    fn read(
        &self,
        relations: &[&super::named::FileRelation],
        _: ParseRequest<'_>,
        _: &crate::domain::elements::types::Elements,
        _: &crate::domain::node_table::types::NodeTable,
        _: &mut crate::domain::symbols::types::Symbols,
        _: Option<&dyn super::named::FileProject>,
    ) -> Vec<Result<Vec<ryo_relations::domain::relations::types::FactColumn>, super::errors::ParseError>> {
        relations.iter().map(|_| Err(super::errors::ParseError::Unsupported)).collect()
    }
}

/// The numbered visit of the language that reads nothing: it lends no file.
impl super::named::NumberedVisit for ParseUseCaseImpl {
    fn open(&self) -> Box<dyn super::named::NumberedReader + '_> {
        Box::new(Nothing)
    }
}

struct Nothing;

impl super::named::NumberedReader for Nothing {
    fn visit(
        &mut self,
        _: ParseRequest<'_>,
        _: &[ryo_relations::domain::relations::types::Relation],
        _: &[&super::named::FileRelation],
        _: Option<&dyn super::named::FileProject>,
        _: &mut crate::domain::symbols::types::Symbols,
        _: &mut dyn FnMut(super::named::NumberedFile<'_>, &mut crate::domain::symbols::types::Symbols),
    ) -> Result<(), super::errors::ParseError> {
        Err(super::errors::ParseError::Unsupported)
    }
}

impl ParseUseCase for ParseUseCaseImpl {
    fn file_relations(&self) -> &dyn super::named::FileRelations {
        self
    }

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

    fn file_relations(&self) -> &dyn super::named::FileRelations {
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
