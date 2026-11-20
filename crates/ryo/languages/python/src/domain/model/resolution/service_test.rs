//! The `resolution` service's cases: the one definition the static rule gives the last name a parsed source reads.

use super::service::resolved_at;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::definitions::service::definition_of;
use crate::domain::model::lexical::types::{NameUse, NodeClass};
use crate::domain::model::view::types::{ModelPart, ModelView as _};
use crate::outbound::syntax::fixtures::model_view::{ModelFixture, viewed};
use ryo::languages::{FileId, Offset, Position};

/// The source.
type Input = &'static str;
/// Where the name of the definition the last read resolves to starts; none when it resolves to no definition.
type Value = Option<Offset>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("of_two_alternative_imports_the_later_one_in_source_order_answers"),
        word: Word::Succeeds,
        input: || "if c:\n    import a as m\nelse:\n    import b as m\nm\n",
        expect: Expect::Value(Some(Offset(46))),
    },
    Case {
        name: CaseName(
            "a_function_body_reads_the_last_module_definition_whatever_comes_after_the_function",
        ),
        word: Word::Succeeds,
        input: || "x = 1\ndef f():\n    return x\nx = 2\nf\ndef g():\n    return x\n",
        expect: Expect::Value(Some(Offset(28))),
    },
    Case {
        name: CaseName("the_value_of_an_assignment_does_not_read_its_own_target"),
        word: Word::Succeeds,
        input: || "x = 1\nx = x\n",
        expect: Expect::Value(Some(Offset(0))),
    },
    Case {
        name: CaseName("a_name_the_file_never_defines_resolves_to_nothing"),
        word: Word::Succeeds,
        input: || "print\n",
        expect: Expect::Value(None),
    },
];

fn subject(source: Input) -> Value {
    let mut answer = None;
    viewed(
        &mut ModelFixture::default(),
        source,
        FileId(0),
        &mut |view| {
            let model = view.semantics(ModelPart::Resolution).model;
            let last = (0..model.lexical.class.len()).rev().find(|number| {
                model.lexical.class.get(*number) == Some(&NodeClass::Name)
                    && model.lexical.usage.get(*number) == Some(&NameUse::Load)
            });
            answer = last
                .and_then(|number| {
                    resolved_at(
                        &model.resolution,
                        Position(u32::try_from(number).unwrap_or(u32::MAX)),
                    )
                })
                .and_then(|found| definition_of(&model.definitions, found))
                .map(|definition| definition.named.start);
        },
    );
    answer
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
