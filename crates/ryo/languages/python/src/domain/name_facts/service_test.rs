//! The `name_facts` service's cases: the names the nodes of a source write.

use super::service::{base, decoration, decorator_ordinal, imported, parameter_place, passed};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::literal_facts::types::TextBuffer;
use crate::domain::node::service::typed_nodes;
use crate::domain::node::types::Numbered;
use crate::fixtures::syntax_files::with_file;
use ruff_python_ast::{AnyNodeRef, ArgOrKeyword};

/// A source.
type Input = &'static str;
/// What each import, decorator, base, argument and parameter of the source answers, in order, joined by `|`.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_import_binds_a_name_and_names_a_dotted_path"),
        word: Word::Succeeds,
        input: || "import a.b\nfrom ..c import d as e\nfrom f import *\n",
        expect: Expect::Value("a=a.b|e=..c.d|*=f"),
    },
    Case {
        name: CaseName("a_decorator_and_a_base_name_what_they_write"),
        word: Word::Succeeds,
        input: || "@first\n@second.third(1, key=2)\nclass Class(Base, Generic[T], *rest):\n    pass\n",
        expect: Expect::Value(
            "base Base|base Generic|base StarBase|@0 first()|@1 second.third(1, key=2)|passes -|passes -",
        ),
    },
    Case {
        name: CaseName("an_argument_hands_on_its_name_and_a_parameter_takes_its_argument_one_way"),
        word: Word::Succeeds,
        input: || "def function(a, /, b, *c, d, **e):\n    call(a, *c, key=d, other=1)\n",
        expect: Expect::Value(
            "positional-only #0|positional-or-keyword #1|var-positional #2|keyword-only #3|var-keyword #4|passes a|passes c|passes d|passes -",
        ),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let mut shown: Vec<String> = Vec::new();
    with_file(source, &mut |file| {
        let mut names = TextBuffer::default();
        for Numbered { id, node } in typed_nodes(file) {
            if let Some(found) = imported(file, id, &mut names) {
                let binding = found.binding.map_or("*", |name| name.0);
                shown.push(format!("{binding}={}", found.target.0));
            }
            if let Some(found) = decoration(file, id, &mut names) {
                let target = found.target.map_or("?", |name| name.0);
                let ordinal = decorator_ordinal(file, id).0;
                shown.push(format!("@{ordinal} {target}({})", found.arguments.0));
            }
            if let AnyNodeRef::StmtClassDef(class) = node {
                for expression in class.arguments.iter().flat_map(|arguments| &arguments.args) {
                    match base(expression, &mut names) {
                        Some(reason) => shown.push(format!("base {reason:?}")),
                        None => shown.push(format!("base {}", names.0)),
                    }
                }
            }
            if let AnyNodeRef::ExprCall(call) = node {
                for argument in call.arguments.iter_source_order() {
                    let expression = match argument {
                        ArgOrKeyword::Arg(expression) => expression,
                        ArgOrKeyword::Keyword(keyword) => &keyword.value,
                    };
                    shown.push(format!(
                        "passes {}",
                        passed(expression).map_or("-", |name| name.0)
                    ));
                }
            }
            if let Some(place) = parameter_place(file, id) {
                shown.push(format!("{} #{}", place.word.0, place.ordinal.0));
            }
        }
    });
    Ok(Box::leak(shown.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
