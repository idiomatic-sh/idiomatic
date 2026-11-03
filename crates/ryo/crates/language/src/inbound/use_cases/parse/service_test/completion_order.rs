//! Non-refining providers retain their original completed-tree/per-walk initialization order.
use super::*;
use std::sync::Mutex;

#[derive(Default)]
struct Ordered {
    events: Mutex<Vec<&'static str>>,
}
macro_rules! forward {
    (syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* } manifest { $( $manifest:tt )* } engine { $( $engine:tt )* }) => {
        $(fn $module(&self) -> Support<Reader<ryo_relations::domain::relations::collections::relations::$module::item::$variant, Seen>> { Small.$module() })*
    };
}
impl ParseUseCase for Ordered {
    type FileModel = Seen;
    fn capabilities(&self) -> &'static [ryo_relations::domain::relations::types::OptionalCapability] {
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
    ryo_relations::each_relation!(forward);
}
#[test]
fn default_requirement_preserves_completion_before_each_model_and_real_reader_results() {
    let provider = Ordered::default();
    assert_eq!(
        provider.completion_requirement(),
        crate::domain::elements::types::CompletionRequirement::Independent
    );
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    build(
        &provider,
        ParseRequest {
            path: SourcePath("order.small"),
            file: FileId(0),
            bytes: ByteSlice(b"x"),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        },
        &mut elements,
        &mut symbols,
    )
    .unwrap();
    assert_eq!(*provider.events.lock().unwrap(), ["complete"]);
    for expected in [2, 4] {
        let mut out = Collect::default();
        walk(
            &provider,
            &mut elements,
            &mut symbols,
            ByteSlice(b"x"),
            FileId(0),
            &[Relation::Function],
            &mut out,
        );
        assert_eq!(out.count(Relation::Function), 1);
        assert_eq!(out.rows()[0].1[0], expected);
    }
    assert_eq!(
        *provider.events.lock().unwrap(),
        ["complete", "model", "model"]
    );
}
