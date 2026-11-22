//! The clauses of the lookup path, each from a source and the rows of one reference, rendered by source line: the same rows
//! from today's parse (`paths`) and from the semantic model (`stepped`).
use super::service::{paths, stepped};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    binding_results::types::ReadContext,
    model::flow::types::FlowContext,
    model::lexical::constants::MODULE,
    model::lexical::service::span_of,
    model::types::Declarations,
    model::view::types::{ModelPart, ModelRead, ModelView as _},
    syntax::service::{is_root, written},
    syntax::types::Rootness,
};
use crate::fixtures::reading::parsed;
use crate::outbound::syntax::fixtures::model_view::{ModelFixture, viewed};
use ryo::languages::{self, ByteSlice, FileId, Position, Tree};
/// A source, and the line and text of the reference whose rows are rendered.
type Input = (&'static str, u32, &'static str);
type Value = &'static str;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("P1 the block of the reference, then the module"),
        word: Word::Succeeds,
        input: || ("x = 1\ndef f():\n    return x\n", 3, "x"),
        expect: Expect::Value("d0 value scope=[2:def f():] | d1 value scope=[module]"),
    },
    Case {
        name: CaseName("P1 a lambda is a block of its own"),
        word: Word::Succeeds,
        input: || ("x = 1\nf = lambda: x\n", 2, "x"),
        expect: Expect::Value("d0 value scope=[2:lambda: x] | d1 value scope=[module]"),
    },
    Case {
        name: CaseName("P2 a method skips its class"),
        word: Word::Succeeds,
        input: || {
            (
                "x = 1\nclass A:\n    x = 2\n    def m(self): return x\n",
                4,
                "x",
            )
        },
        expect: Expect::Value("d0 value scope=[4:def m(self): return x] | d1 value scope=[module]"),
    },
    Case {
        name: CaseName("P2 a class body reads its own block first"),
        word: Word::Succeeds,
        input: || ("x = 1\nclass A:\n    x = 2\n    y = x\n", 4, "x"),
        expect: Expect::Value("d0 value scope=[2:class A:] | d1 value scope=[module]"),
    },
    Case {
        name: CaseName("P3 after a block that declares the name global, the module directly"),
        word: Word::Succeeds,
        input: || {
            (
                "x = 0\ndef h():\n    def f():\n        global x\n        def g(): return x\n",
                5,
                "x",
            )
        },
        expect: Expect::Value(
            "d0 value scope=[5:def g(): return x] | d1 value scope=[3:def f():] | d2 value scope=[module]",
        ),
    },
    Case {
        name: CaseName("P3 a deletion under global keeps its own block first"),
        word: Word::Succeeds,
        input: || ("def f():\n    global x\n    x = 1\n    del x\n", 4, "x"),
        expect: Expect::Value("d0 deletion scope=[1:def f():] | d1 deletion scope=[module]"),
    },
    Case {
        name: CaseName("P4 a comprehension's element is looked up in the comprehension first"),
        word: Word::Succeeds,
        input: || ("y = [a for a in b]\n", 1, "a"),
        expect: Expect::Value("d0 value scope=[1:[a for a in b]] | d1 value scope=[module]"),
    },
    Case {
        name: CaseName("P4 a comprehension's first iterable is looked up in the enclosing block"),
        word: Word::Succeeds,
        input: || ("y = [a for a in b]\n", 1, "b"),
        expect: Expect::Value("d0 value scope=[module]"),
    },
    Case {
        name: CaseName("P5 a header sees its owner's type parameter before the enclosing block"),
        word: Word::Succeeds,
        input: || ("T = 1\ndef f[T](a: T): pass\n", 2, "T"),
        expect: Expect::Value("d0 type scope=[2:T] | d1 type scope=[module]"),
    },
    Case {
        name: CaseName("P5 a decorator does not see the type parameters"),
        word: Word::Succeeds,
        input: || ("T = 1\n@d(T)\ndef f[T](a=T): pass\n", 2, "T"),
        expect: Expect::Value("d0 value scope=[module]"),
    },
    Case {
        name: CaseName(
            "P5 a method sees its class's type parameter between the class and the module",
        ),
        word: Word::Succeeds,
        input: || ("class C[T]:\n    def m(self): return T\n", 2, "T"),
        expect: Expect::Value(
            "d0 value scope=[2:def m(self): return T] | d1 value scope=[1:T] | d2 value scope=[module]",
        ),
    },
    Case {
        name: CaseName("P5 a default does not see the type parameters"),
        word: Word::Succeeds,
        input: || ("T = 1\ndef f[T](\n    a=T): pass\n", 3, "T"),
        expect: Expect::Value("d0 value scope=[module]"),
    },
    Case {
        name: CaseName("P5 an alias's value sees its type parameter before the enclosing block"),
        word: Word::Succeeds,
        input: || ("type A[T] = list[T]\n", 1, "T"),
        expect: Expect::Value("d0 value scope=[1:T] | d1 value scope=[module]"),
    },
    Case {
        name: CaseName("P5 a header whose type parameters lack the name keeps their depth"),
        word: Word::Succeeds,
        input: || ("x = 1\ndef f[T](a: x): pass\n", 2, "x"),
        expect: Expect::Value("d1 type scope=[module]"),
    },
];

/// The clause today's parse alone answers: the semantic model reads a file the parser refuses, by the tree it recovers.
const INCOMPLETE: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("P6 an incomplete file answers nothing, not an empty path"),
    word: Word::Succeeds,
    input: || ("if:\n    pass\n", 1, "x"),
    expect: Expect::Value("incomplete"),
}];

/// One line of a node: `module`, or its line and the first line of its text.
fn node(tree: &Tree<'_>, at: Position) -> String {
    if is_root(tree, at) == Rootness::Root {
        return "module".to_owned();
    }
    let start = usize::try_from(languages::start_at(tree.elements, at).0).unwrap_or(0);
    let line = tree
        .source
        .0
        .get(..start)
        .unwrap_or_default()
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1;
    let text = written(tree, at).0;
    format!("{line}:{}", text.lines().next().unwrap_or_default())
}

fn subject((source, line, text): Input) -> Result<Value, Failure> {
    use crate::inbound::contracts::parse::contract::ParseContract;
    use crate::outbound::{notes::factory::postcard, syntax::factory::ruff_python_parser};
    use ryo::languages::Parse as _;
    use std::sync::Arc;
    let Ok(mut file) = parsed(source.as_bytes()) else {
        return Err(());
    };
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: FileId(0),
    };
    let parser = ParseContract::new(
        crate::settings::PythonParserSettings::default(),
        Arc::new(ruff_python_parser(
            crate::settings::PythonParserSettings::default(),
        )),
        Arc::new(postcard()),
    );
    let mut model = Declarations::default();
    parser.file_model(&tree, &mut model);
    let Some(scratch) = paths(&tree, &model, &mut file.symbols) else {
        return Ok("incomplete");
    };
    let wanted = format!("{line}:{text}");
    let rows: Vec<String> = scratch
        .rows
        .iter()
        .filter(|row| node(&tree, row.reference) == wanted)
        .map(|row| {
            let context = match row.context {
                ReadContext::Value => "value",
                ReadContext::Type => "type",
                ReadContext::Deletion => "deletion",
            };
            format!(
                "d{} {context} scope=[{}]",
                row.depth.0,
                node(&tree, row.scope)
            )
        })
        .collect();
    // A test-only rendering: leaked so the cases table compares it as a static string.
    Ok(Box::leak(rows.join(" | ").into_boxed_str()))
}

/// One line of a node of the model's tree: `module`, or its line and the first line of its text.
fn modelled_node(read: ModelRead<'_>, source: &str, at: Position) -> String {
    if at == MODULE {
        return "module".to_owned();
    }
    let span = span_of(read.tree, at);
    let (start, end) = (
        usize::try_from(span.start.0).unwrap_or(0),
        usize::try_from(span.end.0).unwrap_or(0),
    );
    let line = source
        .get(..start)
        .unwrap_or_default()
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    let text = source.get(start..end).unwrap_or_default();
    format!("{line}:{}", text.lines().next().unwrap_or_default())
}

/// The same rows from the semantic model: the path of the read at the line and text, each step as the parse side shows it.
fn modelled((source, line, text): Input) -> Result<Value, Failure> {
    let wanted = format!("{line}:{text}");
    let mut rows: Vec<String> = Vec::new();
    viewed(
        &mut ModelFixture::default(),
        source,
        FileId(0),
        &mut |view| {
            let read = view.semantics(ModelPart::Flow);
            for each in &read.model.flow.reads {
                if modelled_node(read, source, each.at) != wanted {
                    continue;
                }
                let context = match each.context {
                    FlowContext::Value => "value",
                    FlowContext::Type => "type",
                    FlowContext::Deletion => "deletion",
                };
                stepped(read, each, &mut |scope, depth| {
                    rows.push(format!(
                        "d{} {context} scope=[{}]",
                        depth.0,
                        modelled_node(read, source, scope)
                    ));
                });
            }
        },
    );
    // A test-only rendering: leaked so the cases table compares it as a static string.
    Ok(Box::leak(rows.join(" | ").into_boxed_str()))
}

/// The clauses the semantic model alone answers: today's parse keeps the paths it had, until the swap retires it.
const MODELLED: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("P4 a deletion in a function looks only in the function: the name is its own"),
    word: Word::Succeeds,
    input: || ("x = 1\ndef f():\n    del x\n", 3, "x"),
    expect: Expect::Value("d0 deletion scope=[2:def f():]"),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    run(INCOMPLETE, Word::Succeeds, subject);
    run(CASES, Word::Succeeds, modelled);
    run(MODELLED, Word::Succeeds, modelled);
}
