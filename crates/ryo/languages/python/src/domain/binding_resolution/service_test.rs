//! Clauses C1-C10 of schedule-order resolution, each from a source and the rows of one reference, rendered by source line.
use super::service::resolved;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    binding_results::types::{EventKind, ReadContext},
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
        name: CaseName("C1 the latest event in schedule order, alternatives not joined"),
        word: Word::Succeeds,
        input: || ("x = 1\nif c:\n    x = 2\nelse:\n    x = 3\nprint(x)\n", 6, "x"),
        expect: Expect::Value("d0 bind value scope=[module] event=[5:x] lexical=[module] ordinal=6"),
    },
    Case {
        name: CaseName("C1 a wildcard import is never in effect"),
        word: Word::Succeeds,
        input: || ("x = 1\nfrom m import *\nprint(x)\n", 3, "x"),
        expect: Expect::Value("d0 bind value scope=[module] event=[1:x] lexical=[module] ordinal=1"),
    },
    Case {
        name: CaseName("C2 a method skips its class"),
        word: Word::Succeeds,
        input: || ("x = 1\nclass A:\n    x = 2\n    def f(self): return x\n", 4, "x"),
        expect: Expect::Value("d1 bind value scope=[module] event=[1:x] lexical=[module] ordinal=1"),
    },
    Case {
        name: CaseName("C2 a class body reads its own namespace first"),
        word: Word::Succeeds,
        input: || ("x = 1\nclass A:\n    x = 2\n    y = x\n", 4, "x"),
        expect: Expect::Value("d0 bind value scope=[2:class A:] event=[3:x] lexical=[2:class A:] ordinal=2 | d1 bind value scope=[module] event=[1:x] lexical=[module] ordinal=1"),
    },
    Case {
        name: CaseName("C3 global: the declaration in f, then the module's store routed from f"),
        word: Word::Succeeds,
        input: || ("x = 0\ndef f():\n    global x\n    x = 1\n    return x\n", 5, "x"),
        expect: Expect::Value("d0 global value scope=[2:def f():] event=[3:x] lexical=[2:def f():] ordinal=5 | d1 bind value scope=[module] event=[4:x] lexical=[2:def f():] ordinal=6"),
    },
    Case {
        name: CaseName("C3 a store routed from another function is the module's event"),
        word: Word::Succeeds,
        input: || ("x = 0\ndef g():\n    global x\n    x = 2\ndef h(): return x\n", 5, "x"),
        expect: Expect::Value("d1 bind value scope=[module] event=[4:x] lexical=[2:def g():] ordinal=7"),
    },
    Case {
        name: CaseName("C3 a deletion's path starts at its lexical namespace"),
        word: Word::Succeeds,
        input: || ("def f():\n    global x\n    x = 1\n    del x\n", 4, "x"),
        expect: Expect::Value("d0 global deletion scope=[1:def f():] event=[2:x] lexical=[1:def f():] ordinal=4 | d1 bind deletion scope=[module] event=[3:x] lexical=[1:def f():] ordinal=5"),
    },
    Case {
        name: CaseName("C3 a conditional deletion in the declaring function is a module event"),
        word: Word::Succeeds,
        input: || {
            (
                "def f():\n    global x\n    x = 1\n    if c:\n        del x\n    return x\n",
                6,
                "x",
            )
        },
        expect: Expect::Value("d0 global value scope=[1:def f():] event=[2:x] lexical=[1:def f():] ordinal=4 | d1 delete value scope=[module] event=[5:x] lexical=[1:def f():] ordinal=8"),
    },
    Case {
        name: CaseName("C3 a declaration holds for its whole block, even before the statement"),
        word: Word::Succeeds,
        input: || ("def f():\n    value = 1\n    print(value)\n    global value\n", 3, "value"),
        expect: Expect::Value("d0 global value scope=[1:def f():] event=[4:value] lexical=[1:def f():] ordinal=7 | d1 bind value scope=[module] event=[2:value] lexical=[1:def f():] ordinal=4"),
    },
    Case {
        name: CaseName("C3 nonlocal continues along the ordinary chain"),
        word: Word::Succeeds,
        input: || {
            (
                "def f():\n    x = 1\n    def g():\n        nonlocal x\n        x = 2\n        return x\n",
                6,
                "x",
            )
        },
        expect: Expect::Value("d0 nonlocal value scope=[3:def g():] event=[4:x] lexical=[3:def g():] ordinal=8 | d1 bind value scope=[1:def f():] event=[5:x] lexical=[3:def g():] ordinal=9"),
    },
    Case {
        name: CaseName("C4 an assignment expression binds outside its comprehension"),
        word: Word::Succeeds,
        input: || ("[(y := i) for i in a]\nprint(y)\n", 2, "y"),
        expect: Expect::Value("d0 bind value scope=[module] event=[1:y] lexical=[module] ordinal=5"),
    },
    Case {
        name: CaseName("C5 a method reads its class's type parameter"),
        word: Word::Succeeds,
        input: || ("class C[T]:\n    def m(self): return T\n", 2, "T"),
        expect: Expect::Value("d1 bind value scope=[1:T] event=[1:T] lexical=[1:T] ordinal=0"),
    },
    Case {
        name: CaseName("C5 a header sees the owner's parameters before its own namespace"),
        word: Word::Succeeds,
        input: || ("T = 1\ndef f[T](a: T): pass\n", 2, "T"),
        expect: Expect::Value("d0 bind type scope=[2:T] event=[2:T] lexical=[2:T] ordinal=0 | d1 bind type scope=[module] event=[1:T] lexical=[module] ordinal=1"),
    },
    Case {
        name: CaseName("C5 decorators and defaults do not see type parameters"),
        word: Word::Succeeds,
        input: || ("T = 1\n@d(T)\ndef f[T](a=T): pass\n", 2, "T"),
        expect: Expect::Value("d0 bind value scope=[module] event=[1:T] lexical=[module] ordinal=1"),
    },
    Case {
        name: CaseName("C5 a default does not see type parameters"),
        word: Word::Succeeds,
        input: || ("T = 1\n@d(T)\ndef f[T](a=T): pass\n", 3, "T"),
        expect: Expect::Value("d0 bind value scope=[module] event=[1:T] lexical=[module] ordinal=1"),
    },
    Case {
        name: CaseName("C6 a bound is read at the end of the schedule"),
        word: Word::Succeeds,
        input: || ("class A[T: B]: pass\nB = int\n", 1, "B"),
        expect: Expect::Value("d2 bind value scope=[module] event=[2:B] lexical=[module] ordinal=5"),
    },
    Case {
        name: CaseName("C7 a bare annotation is an event"),
        word: Word::Succeeds,
        input: || ("x = 1\nx: int\nprint(x)\n", 3, "x"),
        expect: Expect::Value("d0 annotation value scope=[module] event=[2:x] lexical=[module] ordinal=2"),
    },
    Case {
        name: CaseName("C8 an except target's cleanup is an event, not a restore"),
        word: Word::Succeeds,
        input: || ("e = 1\ntry: pass\nexcept E as e: pass\nprint(e)\n", 4, "e"),
        expect: Expect::Value("d0 cleanup value scope=[module] event=[3:except E as e: pass] lexical=[module] ordinal=6"),
    },
    Case {
        name: CaseName("C9 a deletion reports the state strictly before it"),
        word: Word::Succeeds,
        input: || ("x = 1\ndel x\n", 2, "x"),
        expect: Expect::Value("d0 bind deletion scope=[module] event=[1:x] lexical=[module] ordinal=1"),
    },
    Case {
        name: CaseName("C10 an incomplete file answers nothing, not an empty resolution"),
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
    let line = tree.source.0.get(..start).unwrap_or_default().iter().filter(|byte| **byte == b'\n').count() + 1;
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
    let Some(scratch) = resolved(&tree, &model, &mut file.symbols) else {
        return Ok("incomplete");
    };
    let wanted = format!("{line}:{text}");
    let rows: Vec<String> = scratch
        .resolutions
        .iter()
        .filter(|row| node(&tree, row.reference) == wanted)
        .map(|row| {
            let kind = match row.kind {
                EventKind::Wildcard => "wildcard",
                EventKind::Read => "read",
                EventKind::Bind => "bind",
                EventKind::Annotation => "annotation",
                EventKind::Delete => "delete",
                EventKind::Cleanup => "cleanup",
                EventKind::Global => "global",
                EventKind::Nonlocal => "nonlocal",
            };
            let context = match row.context {
                ReadContext::Value => "value",
                ReadContext::Type => "type",
                ReadContext::Deletion => "deletion",
            };
            format!(
                "d{} {kind} {context} scope=[{}] event=[{}] lexical=[{}] ordinal={}",
                row.depth.0.0,
                node(&tree, row.scope),
                node(&tree, row.event),
                node(&tree, row.lexical),
                row.ordinal.0
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
