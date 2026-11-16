//! The language that records the order of completion and model initialization.

use crate::inbound::use_cases::parse::types::Recognition;
use ryo_relations::domain::terms::types::UnknownName;
use std::sync::Mutex;

use crate::domain::answers::types::{Reader, Support};
use crate::domain::builders::service::{enter_node, leave_node, token};
use crate::domain::elements::types::{Completion, TableBuilder, Tree};
use crate::inbound::use_cases::parse::errors::ParseError;
use crate::inbound::use_cases::parse::fixtures::small::{Seen, Small};
use crate::inbound::use_cases::parse::interface::ParseUseCase;
use crate::inbound::use_cases::parse::types::ParseRequest;
use ryo_relations::domain::relations::types::{Count, KindBits, Offset};

#[derive(Default)]
pub(crate) struct Ordered {
    pub(crate) events: Mutex<Vec<&'static str>>,
}
macro_rules! forward {
    (syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* } manifest { $( $manifest:tt )* } engine { $( $engine:tt )* }) => {
        $(fn $module(&self) -> Support<Reader<ryo_relations::domain::relations::collections::relations::$module::item::$variant, Seen>> { Small.$module() })*
    };
}
impl ParseUseCase for Ordered {
    type FileModel = Seen;

    fn begin_semantics(
        &self,
        _request: &ParseRequest<'_>,
        _tree: &crate::domain::elements::types::Elements,
        _slot: &mut crate::domain::semantics::types::SemanticSlot,
    ) {
    }

    fn semantics(&self) -> crate::domain::answers::types::SemanticAnswers<'_, Seen> {
        crate::domain::answers::types::SemanticAnswers::Absent
    }

    fn completion_requirement(&self) -> crate::domain::elements::types::CompletionRequirement {
        crate::domain::elements::types::CompletionRequirement::Independent
    }

    fn extend_file_model(
        &self,
        _tree: &Tree<'_>,
        _from: ryo_relations::domain::relations::types::Position,
        _model: &mut Seen,
    ) {
    }

    fn recognizes_explicit(&self, _request: &ParseRequest<'_>) -> Recognition {
        Recognition::Declines
    }
    fn capabilities(&self) -> &'static [ryo_relations::domain::catalog::types::OptionalCapability] {
        Small.capabilities()
    }
    fn configuration_identity(
        &self,
    ) -> crate::inbound::use_cases::parse::types::ParseConfigurationIdentity<'_> {
        crate::inbound::use_cases::parse::types::ParseConfigurationIdentity("ordinary-provider")
    }
    fn file_model(&self, tree: &Tree<'_>, model: &mut Seen) {
        let mut events = self.events.lock().unwrap();
        assert_eq!(events.first(), Some(&"complete"));
        events.push("model");
        Small.file_model(tree, model);
    }
    fn recognizes(&self, request: &ParseRequest<'_>) -> Recognition {
        Small.recognizes(request)
    }
    fn skips(&self) -> &'static [crate::inbound::use_cases::parse::types::SkippedDirectory] {
        Small.skips()
    }
    fn unknown_reasons(&self) -> &'static [UnknownName] {
        Small.unknown_reasons()
    }
    fn kind_count(&self) -> Count {
        Small.kind_count()
    }
    fn kind_names(&self) -> crate::inbound::use_cases::parse::types::KindNames {
        Small.kind_names()
    }
    fn read_tree(
        &self,
        _request: &ParseRequest<'_>,
        builder: &mut TableBuilder<'_>,
    ) -> Result<(), ParseError> {
        enter_node(builder, KindBits(1), Offset(0));
        token(builder, KindBits(2), Offset(0), Offset(1));
        leave_node(builder, Offset(1));
        Ok(())
    }
    fn complete(&self, _request: &ParseRequest<'_>, _completion: &mut Completion<'_>) {
        self.events.lock().unwrap().push("complete");
    }
    ryo_relations::each_relation!(@declared ryo_relations::baseline_relations, forward);
}
