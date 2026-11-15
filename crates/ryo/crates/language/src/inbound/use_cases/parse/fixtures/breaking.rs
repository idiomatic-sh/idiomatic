//! The language whose library fails.

use crate::inbound::use_cases::parse::types::Recognition;
use ryo_relations::domain::terms::types::UnknownName;

use crate::domain::answers::service::absent;
use crate::domain::answers::types::{Reader, Support};
use crate::domain::elements::types::{Completion, TableBuilder, Tree};
use crate::domain::reasons::types::ToolMessage;
use crate::inbound::use_cases::parse::errors::ParseError;
use crate::inbound::use_cases::parse::interface::ParseUseCase;
use crate::inbound::use_cases::parse::types::ParseRequest;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::Count;

/// A language whose library fails: on bytes that are not text it says so, on any other it names the failure.
pub(crate) struct Breaking;

/// Answers every relation as not in the language.
macro_rules! absent_all {
    (
        syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        manifest { $( $manifest:tt )* }
        engine { $( $engine:tt )* }
    ) => {
        $(
            fn $module(
                &self,
            ) -> Support<Reader<ryo_relations::domain::relations::collections::relations::$module::item::$variant, Self::FileModel>> {
                absent(AbsenceCause::NoSuchConstruct)
            }
        )*
    };
}

impl ParseUseCase for Breaking {
    type FileModel = ();

    fn begin_semantics(
        &self,
        _request: &ParseRequest<'_>,
        _tree: &crate::domain::elements::types::Elements,
        _slot: &mut crate::domain::semantics::types::SemanticSlot,
    ) {
    }

    fn semantics(&self) -> Option<&dyn crate::domain::answers::types::Semantics> {
        None
    }

    fn completion_requirement(&self) -> crate::domain::elements::types::CompletionRequirement {
        crate::domain::elements::types::CompletionRequirement::Independent
    }

    fn extend_file_model(
        &self,
        _tree: &Tree<'_>,
        _from: ryo_relations::domain::relations::types::Position,
        _model: &mut (),
    ) {
    }

    fn recognizes_explicit(&self, _request: &ParseRequest<'_>) -> Recognition {
        Recognition::Declines
    }

    fn capabilities(&self) -> &'static [ryo_relations::domain::catalog::types::OptionalCapability] {
        &[]
    }

    fn configuration_identity(
        &self,
    ) -> crate::inbound::use_cases::parse::types::ParseConfigurationIdentity<'_> {
        crate::inbound::use_cases::parse::types::ParseConfigurationIdentity("")
    }

    fn file_model(&self, _tree: &Tree<'_>, _model: &mut ()) {}

    fn recognizes(&self, _request: &ParseRequest<'_>) -> Recognition {
        Recognition::Reads
    }

    fn skips(&self) -> &'static [crate::inbound::use_cases::parse::types::SkippedDirectory] {
        &[]
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
    }

    fn kind_count(&self) -> Count {
        Count(0)
    }

    fn kind_names(&self) -> crate::inbound::use_cases::parse::types::KindNames {
        crate::inbound::use_cases::parse::types::KindNames(Vec::new())
    }

    fn read_tree(
        &self,
        _request: &ParseRequest<'_>,
        _builder: &mut TableBuilder<'_>,
    ) -> Result<(), ParseError> {
        Err(ParseError::Library {
            reason: ToolMessage("the library failed".into()),
        })
    }

    fn complete(&self, _request: &ParseRequest<'_>, _completion: &mut Completion<'_>) {}

    ryo_relations::each_relation!(@declared ryo_relations::baseline_relations, absent_all);
}
