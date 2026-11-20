//! The clauses of the lookup path, each from a source and the rows of one reference, rendered by source line.
use super::service::paths;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    binding_results::types::ReadContext,
    model::types::Declarations,
    syntax::service::{is_root, written},
    syntax::types::Rootness,
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Position, Tree};
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
        name: CaseName("P6 an incomplete file answers nothing, not an empty path"),
        word: Word::Succeeds,
        input: || ("if:\n    pass\n", 1, "x"),
        expect: Expect::Value("incomplete"),
    },
];

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
        file: languages::FileId(0),
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

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
