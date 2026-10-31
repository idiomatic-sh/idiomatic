//! parse_missing_a_relation: the compiler rejects this.

use ryo_language::domain::terms::types::UnknownName;
use ryo_language::domain::elements::types::{Elements, TableBuilder, Completion};
use ryo_language::domain::answers::types::{Reader, AbsenceCause, Support};
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::parse::errors::ParseError;
use ryo_language::inbound::use_cases::parse::interface::ParseUseCase;
use ryo_language::inbound::use_cases::parse::types::ParseRequest;

/// Answers every syntax relation of the catalog but `function`.
macro_rules! all_but_function {
    (
        syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        manifest { $( $manifest:tt )* }
        engine { $( $engine:tt )* }
    ) => {
        $( all_but_function!(@one $module $variant); )*
    };
    (@one function Function) => {};
    (@one $module:ident $variant:ident) => {
        fn $module(
            &self,
        ) -> Support<Reader<ryo_language::domain::relations::collections::relations::$module::item::$variant, Self::FileModel>> {
            ryo_language::domain::answers::service::absent(AbsenceCause::NoSuchConstruct)
        }
    };
}

struct Missing;

impl ParseUseCase for Missing {
    type FileModel = ();

    fn capabilities(&self) -> &'static [ryo_language::domain::relations::types::OptionalCapability] { &[] }

    fn file_model(&self, _tree: &ryo_language::domain::elements::types::Tree<'_>, _model: &mut ()) {}

    fn recognizes(&self, _request: &ParseRequest<'_>) -> ryo_language::inbound::use_cases::parse::types::Recognition {
        ryo_language::inbound::use_cases::parse::types::Recognition::Declines
    }

    fn skips(&self) -> &'static [ryo_language::inbound::use_cases::parse::types::SkippedDirectory] {
        &[]
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
    }

    fn kind_count(&self) -> ryo_language::domain::relations::types::Count {
        ryo_language::domain::relations::types::Count(0)
    }

    fn kind_names(&self) -> ryo_language::inbound::use_cases::parse::types::KindNames {
        ryo_language::inbound::use_cases::parse::types::KindNames(Vec::new())
    }

    fn read_tree(
        &self,
        _request: &ParseRequest<'_>,
        _builder: &mut TableBuilder<'_>,
    ) -> Result<(), ParseError> {
        Ok(())
    }

    fn complete(&self, _request: &ParseRequest<'_>, _completion: &mut Completion<'_>) {}

    ryo_language::each_relation!(all_but_function);
}

fn main() {}
