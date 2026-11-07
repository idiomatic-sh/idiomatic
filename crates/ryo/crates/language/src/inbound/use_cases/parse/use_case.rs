//! The `parse` use case here is the language that reads nothing: it recognizes no file and answers every
//! relation as not in it. The contract runs against it, and a real language replaces it with its own.

use super::constants::NO_CONFIGURATION;
use super::errors::ParseError;
use super::interface::ParseUseCase;
use super::service::{absence_in, baseline_in, build_in, kinds_read_in, read_in, walk_in};
use super::types::ParseEngine;
use super::types::{KindNames, ParseConfigurationIdentity, ParseRequest, SkippedDirectory};
use crate::domain::elements::types::Elements;
use crate::domain::elements::types::{Completion, CompletionRequirement, TableBuilder, Tree};
use crate::domain::semantics::types::SemanticSlot;
use crate::domain::symbols::types::{ByteSlice, Symbols};
use crate::inbound::use_cases::parse::types::Recognition;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{Count, Position};
use ryo_relations::domain::relations::types::{FileId, KindBits, Relation, RowWriter};
use ryo_relations::domain::terms::types::UnknownName;

/// A language that reads no file.
#[derive(Clone, Copy, Debug)]
pub struct ParseUseCaseImpl;

impl ParseUseCase for ParseUseCaseImpl {
    type FileModel = ();

    fn capabilities(
        &self,
    ) -> &'static [ryo_relations::domain::relations::types::OptionalCapability] {
        &[]
    }

    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        ParseConfigurationIdentity(NO_CONFIGURATION)
    }

    fn file_model(&self, _tree: &Tree<'_>, _model: &mut ()) {}

    fn begin_semantics(
        &self,
        _request: &ParseRequest<'_>,
        _tree: &Elements,
        _slot: &mut SemanticSlot,
    ) {
    }

    fn completion_requirement(&self) -> CompletionRequirement {
        CompletionRequirement::Independent
    }

    fn extend_file_model(&self, _tree: &Tree<'_>, _from: Position, _model: &mut ()) {}

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

    fn read_tree(
        &self,
        _request: &ParseRequest<'_>,
        _builder: &mut TableBuilder<'_>,
    ) -> Result<(), ParseError> {
        Err(ParseError::Unsupported)
    }

    fn complete(&self, _request: &ParseRequest<'_>, _completion: &mut Completion<'_>) {}

    ryo_relations::each_relation!(absent_syntax);
}

/// Every `ParseUseCase` is a `ParseEngine`: the engine reads it through the use case's own drive.
impl<P: ParseUseCase> ParseEngine for P {
    fn capabilities(
        &self,
    ) -> &'static [ryo_relations::domain::relations::types::OptionalCapability] {
        ParseUseCase::capabilities(self)
    }

    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        ParseUseCase::configuration_identity(self)
    }

    fn begin_semantics(
        &self,
        request: &ParseRequest<'_>,
        tree: &Elements,
        slot: &mut SemanticSlot,
    ) {
        ParseUseCase::begin_semantics(self, request, tree, slot);
    }

    fn recognition(&self, request: &ParseRequest<'_>) -> Recognition {
        self.recognizes(request)
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

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        ParseUseCase::unknown_reasons(self)
    }

    fn reader_kinds(&self) -> Vec<KindBits> {
        kinds_read_in(self)
    }

    fn build_file(
        &self,
        request: ParseRequest<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
    ) -> Result<(), ParseError> {
        build_in(self, request, table, symbols)
    }

    fn baseline_file(&self, request: &ParseRequest<'_>) -> Result<(), ParseError> {
        baseline_in(self, request)
    }

    fn read_file(
        &self,
        request: ParseRequest<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
        wanted: &[Relation],
        out: &mut dyn RowWriter,
    ) -> Result<(), ParseError> {
        read_in(self, request, table, symbols, wanted, out)
    }

    fn read_file_explicit(
        &self,
        request: ParseRequest<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
        wanted: &[Relation],
        out: &mut dyn RowWriter,
    ) -> Result<(), ParseError> {
        super::service::read_explicit_in(self, request, table, symbols, wanted, out)
    }

    fn walk_file(
        &self,
        elements: &mut Elements,
        symbols: &mut Symbols,
        source: ByteSlice<'_>,
        file: FileId,
        wanted: &[Relation],
        out: &mut dyn RowWriter,
    ) {
        walk_in(self, elements, symbols, source, file, wanted, out);
    }

    fn absence_of(&self, relation: Relation) -> Option<AbsenceCause> {
        absence_in(self, relation)
    }
}
