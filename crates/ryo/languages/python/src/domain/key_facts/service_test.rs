//! The `key_facts` service's cases: which expression statements of a source are constants of one value.

use super::service::keyed;
use super::types::KeyBuild;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::literal_facts::types::TextBuffer;
use crate::domain::node::service::{id_of, typed_nodes, written};
use crate::fixtures::syntax_files::with_file;
use ruff_python_ast::AnyNodeRef;
use ryo::languages::{self, RetainedKeyIndex};

/// A source.
type Input = &'static str;
/// For each expression statement of the source, the text of the constant its class is named by, or `-` for one with no key;
/// joined by `|`.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("numbers_of_one_value_have_one_key_across_their_types"),
        word: Word::Succeeds,
        input: || "1\n1.0\nTrue\n2\n1 + 0j\n",
        expect: Expect::Value("1|1|1|2|1"),
    },
    Case {
        name: CaseName("a_sign_is_folded_and_a_string_is_keyed_by_its_decoded_value"),
        word: Word::Succeeds,
        input: || "-1\n- 1\n'a'\n\"a\"\n'\\x61'\nb'a'\n",
        expect: Expect::Value("- 1|- 1|'\\x61'|'\\x61'|'\\x61'|b'a'"),
    },
    Case {
        name: CaseName("a_tuple_is_keyed_by_its_elements_and_a_name_has_no_key"),
        word: Word::Succeeds,
        input: || "(1, 'a')\n(1.0, 'a')\n(1, a)\na\nNone\n...\n",
        expect: Expect::Value("(1.0, 'a')|(1.0, 'a')|-|-|None|..."),
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
        keyed(
            file,
            &mut KeyBuild {
                index: &mut index,
                words: &mut words,
                text: &mut text,
            },
        );
        for numbered in typed_nodes(file) {
            if let AnyNodeRef::StmtExpr(statement) = numbered.node {
                let class = id_of(&*statement.value)
                    .and_then(|id| index.classes.get(usize::from(id)).copied().flatten());
                shown.push(class.map_or_else(
                    || "-".to_owned(),
                    |found| written(file, found.representative).0.to_owned(),
                ));
            }
        }
    });
    Ok(Box::leak(shown.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
