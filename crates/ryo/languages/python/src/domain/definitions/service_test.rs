//! The `definitions` service's cases: what each element of a source defines, as Python's word for it and its name, in the order of
//! the file.

use super::service::definition;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::types::Declarations;
use crate::domain::scopes::service::declare;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

/// A source.
type Input = &'static str;
/// Each definition of the source, as its word and its name: `method f`.
type Value = &'static [&'static str];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_function_in_a_class_is_a_method_and_its_parameters_say_how_they_take_their_argument",
        ),
        word: Word::Succeeds,
        input: || "class C:\n    async def f(self, a, /, b, *c, d, **e): pass\n",
        expect: Expect::Value(&[
            "class C",
            "async-method f",
            "positional-only self",
            "positional-only a",
            "positional-or-keyword b",
            "var-positional c",
            "keyword-only d",
            "var-keyword e",
        ]),
    },
    Case {
        name: CaseName("imports_assignments_and_type_parameters_are_definitions"),
        word: Word::Succeeds,
        input: || "import a.b\nfrom m import x as y\ntype T[U, *V, **W] = int\nz = 1\n",
        expect: Expect::Value(&[
            "import a",
            "from-import y",
            "type-alias T",
            "type-var U",
            "type-var-tuple V",
            "param-spec W",
            "variable z",
        ]),
    },
];

fn subject(source: Input) -> Value {
    let file = parsed(source.as_bytes()).expect("the source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    declare(&tree, &mut model);
    let found: Vec<&'static str> = languages::positions(&file.elements)
        .filter_map(|at| definition(&tree, &model, at))
        .map(|defined| {
            let name = languages::text_of(&file.symbols, tree.source, defined.name)
                .map_or(String::new(), |text| text.0.to_owned());
            &*Box::leak(format!("{} {name}", defined.defined.word.0).into_boxed_str())
        })
        .collect();
    Box::leak(found.into_boxed_slice())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
