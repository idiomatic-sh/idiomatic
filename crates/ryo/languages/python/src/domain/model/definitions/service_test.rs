//! The `definitions` service's cases: the definitions a parsed source makes of a name in its module, in source order.

use super::service::definitions_of;
use super::types::DefinitionWord;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::lexical::constants::MODULE;
use crate::domain::model::names::service::find;
use crate::domain::model::view::types::{ModelPart, ModelView as _};
use crate::outbound::syntax::fixtures::model_view::{ModelFixture, viewed};
use ryo::languages::{FileId, NameText};

/// The source and the name asked about.
type Input = (&'static str, &'static str);
/// What each definition of the name in the module is, in source order.
type Value = &'static [DefinitionWord];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "an_import_of_a_dotted_name_a_function_and_an_assignment_define_the_same_name",
        ),
        word: Word::Succeeds,
        input: || ("import a.b\nasync def a(): pass\na = 1\n", "a"),
        expect: Expect::Value(&[
            DefinitionWord::Import,
            DefinitionWord::AsyncFunction,
            DefinitionWord::Variable,
        ]),
    },
    Case {
        name: CaseName("a_global_assignment_in_a_function_defines_the_modules_name"),
        word: Word::Succeeds,
        input: || ("def f():\n    global total\n    total = 1\n", "total"),
        expect: Expect::Value(&[DefinitionWord::Variable]),
    },
    Case {
        name: CaseName("a_parameter_is_not_a_definition_of_the_module"),
        word: Word::Succeeds,
        input: || ("def f(first, /, *rest):\n    return first\n", "first"),
        expect: Expect::Value(&[]),
    },
];

fn subject((source, name): Input) -> Value {
    let mut words = Vec::new();
    viewed(
        &mut ModelFixture::default(),
        source,
        FileId(0),
        &mut |view| {
            let model = view.semantics(ModelPart::Lexical).model;
            let Some(code) = find(&model.names, NameText(name)) else {
                return;
            };
            for definition in definitions_of(&model.definitions, MODULE, code) {
                words.extend(
                    model
                        .definitions
                        .all
                        .get(usize::from(*definition))
                        .map(|found| found.word),
                );
            }
        },
    );
    Box::leak(words.into_boxed_slice())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
