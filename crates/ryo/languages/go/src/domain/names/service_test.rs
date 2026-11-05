//! The `names` service's cases: the reasons are declared in their order, and the dotted names and import targets it makes.

use super::service::{
    base_of, blankness, callee_of, comment_text, dotted, embedded_name, import_binding,
    import_target, unknown,
};
use super::types::{Blankness, Unresolved};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::language::constants::UNKNOWN_REASONS;
use crate::domain::nodes::types::GoNode;
use crate::domain::syntax::service::kind;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};
use strum::IntoEnumIterator as _;

/// A source.
type Input = &'static str;
/// The dotted names of its calls' callees, the bindings and targets of its imports, and what else it says: what each call calls,
/// the blank names, the names embedded fields get, the bases of types and the text of comments.
type Value = (
    &'static [&'static str],
    &'static [&'static str],
    &'static [&'static str],
);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_selection_chain_is_one_dotted_name"),
        word: Word::Succeeds,
        input: || "func f() { a.b.c() }\n",
        expect: Expect::Value((&["a.b.c"], &[], &["Named"])),
    },
    Case {
        name: CaseName("an_indexed_callee_is_no_dotted_name_but_an_instantiated_one_is"),
        word: Word::Succeeds,
        input: || "func f() { xs[0](); m[int, string]() }\n",
        expect: Expect::Value((&["m"], &[], &["Dynamic", "Named"])),
    },
    Case {
        name: CaseName("an_import_is_bound_by_its_name_when_it_has_one"),
        word: Word::Succeeds,
        input: || "import (\n\t\"fmt\"\n\tstr \"strings\"\n\t. \"math\"\n)\n",
        expect: Expect::Value((
            &[],
            &["declared-by-package fmt", "str strings", "dot-import math"],
            &[],
        )),
    },
    Case {
        name: CaseName("a_type_written_out_is_converted_to_and_not_called"),
        word: Word::Succeeds,
        input: || "func f() { []byte(s); map[string]int(m); g()() }\n",
        expect: Expect::Value((
            &["g"],
            &[],
            &["Conversion", "Conversion", "Dynamic", "Named"],
        )),
    },
    Case {
        name: CaseName("the_blank_identifier_is_blank_and_any_other_name_is_named"),
        word: Word::Succeeds,
        input: || "func f() { _, a := 1, 2 }\n",
        expect: Expect::Value((&[], &[], &["blank"])),
    },
    Case {
        name: CaseName("an_embedded_field_is_named_by_the_base_of_its_type"),
        word: Word::Succeeds,
        input: || "type S struct {\n\t*pkg.Other[int]\n\tBase\n}\n",
        expect: Expect::Value((
            &[],
            &[],
            &[
                "embedded Other",
                "base pkg.Other",
                "embedded Base",
                "base Base",
            ],
        )),
    },
    Case {
        name: CaseName("a_comment_is_its_text_without_its_delimiters"),
        word: Word::Succeeds,
        input: || "// a\nfunc f() { /* b */ }\n",
        expect: Expect::Value((&[], &[], &["comment  a", "comment  b "])),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let reasons: Vec<&'static str> = Unresolved::iter().map(<&'static str>::from).collect();
    let declared: Vec<&'static str> = UNKNOWN_REASONS.iter().map(|each| each.text.0).collect();
    let numbered = Unresolved::iter()
        .zip(UNKNOWN_REASONS)
        .all(|(reason, each)| unknown(reason) == languages::unknown_symbol(each.reason));
    if !numbered
        || reasons != declared
        || unknown(Unresolved::DynamicCallee) == unknown(Unresolved::Unnamed)
    {
        return Err(());
    }
    let mut file = parsed(source.as_bytes()).map_err(drop)?;
    let elements = std::mem::take(&mut file.elements);
    let tree = Tree {
        elements: &elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let text = |symbols: &languages::Symbols, symbol| {
        Unresolved::iter()
            .find(|reason| unknown(*reason) == symbol)
            .map_or_else(
                || {
                    languages::text_of(symbols, tree.source, symbol)
                        .map_or_else(String::new, |each| each.0.to_owned())
                },
                |reason| <&'static str>::from(reason).to_owned(),
            )
    };
    let mut names = Vec::new();
    let mut imports = Vec::new();
    for at in languages::positions(&elements) {
        if kind(&tree, at) == Kind::Node(GoNode::CallExpr)
            && let Some(callee) = crate::domain::syntax::service::nodes(&tree, at).next()
            && let Some(name) = dotted(&tree, &mut file.symbols, callee)
        {
            names.push(text(&file.symbols, name));
        }
        if kind(&tree, at) == Kind::Node(GoNode::ImportSpec)
            && let Some(target) = import_target(&tree, &mut file.symbols, at)
        {
            imports.push(format!(
                "{} {}",
                text(&file.symbols, import_binding(&tree, at)),
                text(&file.symbols, target)
            ));
        }
    }
    let mut facts = Vec::new();
    for at in languages::positions(&elements) {
        match kind(&tree, at) {
            Kind::Node(GoNode::CallExpr) => {
                if let Some(callee) = crate::domain::syntax::service::nodes(&tree, at).next() {
                    facts.push(format!("{:?}", callee_of(&tree, callee)));
                }
            }
            Kind::Node(GoNode::Ident) => {
                if blankness(&tree, at) == Blankness::Blank {
                    facts.push("blank".to_owned());
                }
            }
            Kind::Node(GoNode::Field) => {
                if let Some(embedded) =
                    crate::domain::fields::service::type_of(&tree, at).filter(|_| {
                        crate::domain::fields::service::names(&tree, at)
                            .next()
                            .is_none()
                    })
                {
                    if let Some(name) = embedded_name(&tree, embedded) {
                        facts.push(format!("embedded {}", text(&file.symbols, name)));
                    }
                    if let Some(base) = base_of(&tree, &mut file.symbols, embedded) {
                        facts.push(format!("base {}", text(&file.symbols, base)));
                    }
                }
            }
            Kind::Token(Token::Comment) => {
                let said = comment_text(&tree, &mut file.symbols, at);
                facts.push(format!("comment {}", text(&file.symbols, said)));
            }
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
        }
    }
    let leak = |list: Vec<String>| -> &'static [&'static str] {
        let leaked: Vec<&'static str> = list
            .into_iter()
            .map(|each| &*Box::leak(each.into_boxed_str()))
            .collect();
        Box::leak(leaked.into_boxed_slice())
    };
    Ok((leak(names), leak(imports), leak(facts)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
