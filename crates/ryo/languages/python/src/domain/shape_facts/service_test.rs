//! The `shape_facts` service's cases: which expression statements of a source are written with one structure.

use super::service::shapes;
use super::types::ShapeBuild;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::literal_facts::types::TextBuffer;
use crate::domain::node::service::{id_of, typed_nodes, written};
use crate::fixtures::syntax_files::with_file;
use ruff_python_ast::AnyNodeRef;
use ryo::languages::{self, RetainedKeyIndex};

/// A source.
type Input = &'static str;
/// For each expression statement of the source, the text of the expression its shape's class is named by; joined by `|`.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("parentheses_and_spacing_are_no_structure_and_an_operator_is"),
        word: Word::Succeeds,
        input: || "a + b\n(a+b)\na - b\nb + a\n",
        expect: Expect::Value("a+b|a+b|a - b|b + a"),
    },
    Case {
        name: CaseName("a_string_is_its_decoded_value_and_adjacent_literal_pieces_are_one"),
        word: Word::Succeeds,
        input: || "'ab'\n\"ab\"\n'a' 'b'\nf'ab{x}'\nf'a' f'b{x}'\nf'ab{y}'\n",
        expect: Expect::Value("ab|ab|ab|f'a' f'b{x}'|f'a' f'b{x}'|f'ab{y}'"),
    },
    Case {
        name: CaseName("a_call_is_its_callee_its_arguments_and_its_keywords"),
        word: Word::Succeeds,
        input: || "f(a, k=1)\nf(a, k = 1)\nf(a, j=1)\nf(a)\nlambda x: x\nlambda x: x\nlambda y: y\n",
        expect: Expect::Value(
            "f(a, k = 1)|f(a, k = 1)|f(a, j=1)|f(a)|lambda x: x|lambda x: x|lambda y: y",
        ),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let mut shown: Vec<String> = Vec::new();
    with_file(source, &mut |file| {
        let mut index = RetainedKeyIndex::default();
        let mut words = languages::facts::file_words(
            &languages::facts::base_words(&languages::FixedNames::default(), &[])
                .expect("no fixed name to refuse"),
        );
        let mut text = TextBuffer::default();
        shapes(
            file,
            &mut ShapeBuild {
                index: &mut index,
                words: &mut words,
                text: &mut text,
            },
        );
        for numbered in typed_nodes(file) {
            if let AnyNodeRef::StmtExpr(statement) = numbered.node {
                let class = id_of(&*statement.value)
                    .and_then(|id| index.classes.get(usize::from(id)).copied().flatten());
                shown.push(
                    class.map_or_else(|| "-".to_owned(), |found| written(file, found).0.to_owned()),
                );
            }
        }
    });
    Ok(Box::leak(shown.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
