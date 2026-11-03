//! every_relation_answered: the compiler accepts this.

use ryo_language::domain::answers::types::{ReadItem, Reader, Reads, Support};
use ryo_language::domain::elements::types::{Completion, TableBuilder, Tree};
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::parse::errors::ParseError;
use ryo_language::inbound::use_cases::parse::interface::ParseUseCase;
use ryo_language::inbound::use_cases::parse::types::ParseRequest;
use ryo_language::inbound::use_cases::project::errors::ProjectError;
use ryo_language::inbound::use_cases::project::interface::ProjectUseCase;
use ryo_language::inbound::use_cases::project::types::{ManifestName, ProjectRequest};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::collections::relations::function::item::{
    Function, FunctionRow,
};
use ryo_relations::domain::relations::types::{Count, Position, Rows};
use ryo_relations::domain::terms::types::UnknownName;

/// Answers every manifest relation as not in the language, for a language of this test that has no manifests.
macro_rules! no_manifests {
    (
        syntax { $( $syntax:tt )* }
        manifest { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        engine { $( $engine:tt )* }
    ) => {
        $(
            fn $module(
                &self,
            ) -> Support<
                ryo_language::inbound::use_cases::project::types::ManifestReader<
                    ryo_relations::domain::relations::collections::relations::$module::item::$variant,
                >,
            > {
                ryo_language::domain::answers::service::absent(AbsenceCause::NoManifests)
            }
        )*
    };
}

/// Answers every relation but `function` as not in the language, and `function` with a reader of its own relation.
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
        ) -> Support<Reader<ryo_relations::domain::relations::collections::relations::$module::item::$variant, Self::FileModel>> {
            ryo_language::domain::answers::service::absent(ryo_relations::domain::answers::types::AbsenceCause::NoSuchConstruct)
        }
    };
}

/// The reader of `function`: it is given the rows of `function` and writes none.
fn visit(
    _tree: &Tree<'_>,
    _model: &(),
    _symbols: &mut Symbols,
    _at: Position,
    _out: &mut dyn Rows<FunctionRow>,
) {
}

/// The item that reads `function`: the shape ryo owns, which the language implements.
struct FunctionItem;

impl ryo_language::domain::answers::types::declared::Syntax for FunctionItem {}

impl ryo_language::domain::answers::types::Visiting<Function, ()> for FunctionItem {
    fn visit(
        &self,
        tree: &Tree<'_>,
        model: &(),
        symbols: &mut Symbols,
        at: Position,
        out: &mut dyn Rows<FunctionRow>,
    ) {
        visit(tree, model, symbols, at, out);
    }
}

impl ReadItem for FunctionItem {
    type Relation = Function;
    type Model = ();

    const READER: Reader<Function, ()> = Reader {
        reads: Reads::Every,
        visit: &FunctionItem,
        expansions: ryo_language::domain::answers::types::Expansions::Ignored,
        relation: std::marker::PhantomData,
    };
}

struct Language;

impl ParseUseCase for Language {
    type FileModel = ();

    fn capabilities(
        &self,
    ) -> &'static [ryo_relations::domain::relations::types::OptionalCapability] {
        &[]
    }

    fn configuration_identity(
        &self,
    ) -> ryo_language::inbound::use_cases::parse::types::ParseConfigurationIdentity<'_> {
        ryo_language::inbound::use_cases::parse::types::ParseConfigurationIdentity("")
    }

    fn file_model(&self, _tree: &ryo_language::domain::elements::types::Tree<'_>, _model: &mut ()) {
    }

    fn recognizes(
        &self,
        _request: &ParseRequest<'_>,
    ) -> ryo_language::inbound::use_cases::parse::types::Recognition {
        ryo_language::inbound::use_cases::parse::types::Recognition::Declines
    }

    fn skips(&self) -> &'static [ryo_language::inbound::use_cases::parse::types::SkippedDirectory] {
        &[]
    }

    fn unknown_reasons(&self) -> &'static [UnknownName] {
        &[]
    }

    fn kind_count(&self) -> Count {
        Count(0)
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

    fn function(&self) -> Support<Reader<Function, ()>> {
        ryo_language::domain::answers::service::reads::<FunctionItem>()
    }

    ryo_relations::each_relation!(all_but_function);
}

impl ProjectUseCase for Language {
    type Derived = ();

    fn manifests(&self) -> &'static [ManifestName] {
        &[]
    }

    fn primary_role(
        &self,
        _path: ryo_language::domain::files::types::SourcePath<'_>,
    ) -> ryo_language::domain::files::types::PrimaryFileRole {
        ryo_language::domain::files::service::primary_role_from_manifests(
            _path,
            self.manifests().iter().map(|manifest| manifest.0),
        )
    }

    fn resolve(
        &self,
        _request: &ProjectRequest,
    ) -> Result<ryo_language::inbound::use_cases::project::types::Resolved, ProjectError> {
        Ok(ryo_language::inbound::use_cases::project::types::Resolved {
            refused: Vec::new(),
            unlisted: Vec::new(),
            consulted: Vec::new(),
            derived: (),
        })
    }

    ryo_relations::each_relation!(no_manifests);
}

fn main() {}
