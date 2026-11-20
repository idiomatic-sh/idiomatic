//! Clauses A1-A4 of annotation evaluation timing, each from a source and its rows rendered per annotation by source text.
use super::service::evaluated;
use super::types::{Evaluation, Mode};
use crate::domain::binding_schedule::constants::LAZY_ANNOTATIONS_FROM as LAZY;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    model::types::Declarations,
    standard_library::types::{PythonMajor, PythonMinor, TargetVersion},
    syntax::service::written,
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};
/// A source, and whether the target is Python 3.14 (native lazy annotations) rather than Python 3.10.
type Input = (&'static str, bool);
type Value = &'static str;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("A1 parameter and return annotations are evaluated with the definition (§8.7)"),
        word: Word::Succeeds,
        input: || ("def f(a: int, *b: str) -> bool: pass\n", false),
        expect: Expect::Value("eager/eager [bool]: bool bool | eager/eager [int]: int int | eager/eager [str]: str str"),
    },
    Case {
        name: CaseName("A1 defaults, decorators, bases and targets are no annotation"),
        word: Word::Succeeds,
        input: || ("@d\ndef f(a=1): pass\nclass C(B): pass\nx = 1\n", false),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("A1 a lambda's parameters have no annotation and a lambda is no function definition"),
        word: Word::Succeeds,
        input: || ("f = lambda a: a\n", false),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("A3 module and class variable annotations are evaluated; function-local ones are not (§7.2.2)"),
        word: Word::Succeeds,
        input: || ("x: int = 1\nclass C:\n    y: str\ndef f():\n    z: bool = True\n", false),
        expect: Expect::Value("eager/eager [int]: int int | eager/eager [str]: str str | unevaluated/eager [bool]: bool bool"),
    },
    Case {
        name: CaseName("A3 a parenthesized target is not simple and is still evaluated eagerly"),
        word: Word::Succeeds,
        input: || ("(x): int = 1\na.b: str = 2\n", false),
        expect: Expect::Value("eager/eager [int]: int int | eager/eager [str]: str str"),
    },
    Case {
        name: CaseName("A3 the annotations future postpones signature and simple annotations (PEP 563)"),
        word: Word::Succeeds,
        input: || ("from __future__ import annotations\ndef f(a: int): pass\nx: int\n(y): int\ndef g():\n    z: int\n", false),
        expect: Expect::Value("postponed/lazy [int]: int int | postponed/lazy [int]: int int | unevaluated/lazy [int]: int int | unevaluated/lazy [int]: int int"),
    },
    Case {
        name: CaseName("A3 Python 3.14 defers annotations natively (PEP 649, PEP 749)"),
        word: Word::Succeeds,
        input: || ("def f(a: int): pass\nx: int\n(y): int\ndef g():\n    z: int\n", true),
        expect: Expect::Value("deferred/lazy [int]: int int | deferred/lazy [int]: int int | unevaluated/lazy [int]: int int | unevaluated/lazy [int]: int int"),
    },
    Case {
        name: CaseName("A2 a lambda body and a generator's element and later clauses are evaluated on their own (§6.2.8, §6.14)"),
        word: Word::Succeeds,
        input: || ("def f(a: (lambda: q), b: (e for e in s if t)): pass\n", false),
        expect: Expect::Value("eager/eager [lambda: q]: lambda: q lambda : | eager/eager [(e for e in s if t)]: (e for e in s if t) ( for e in s if t s s )"),
    },
    Case {
        name: CaseName("A4 a file is independent of its suffix: no stub policy here"),
        word: Word::Succeeds,
        input: || ("x: Final = 1\n", false),
        expect: Expect::Value("eager/eager [Final]: Final Final"),
    },
];
fn subject((source, lazy): Input) -> Result<Value, Failure> {
    use crate::inbound::contracts::parse::contract::ParseContract;
    use crate::outbound::{notes::factory::postcard, syntax::factory::ruff_python_parser};
    use ryo::languages::Parse as _;
    use std::sync::Arc;
    let Ok(file) = parsed(source.as_bytes()) else {
        return Err(());
    };
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let parser = ParseContract::new(
        crate::settings::PythonParserSettings::default(),
        Arc::new(ruff_python_parser(crate::settings::PythonParserSettings::default())),
        Arc::new(postcard()),
    );
    let mut model = Declarations::default();
    parser.file_model(&tree, &mut model);
    model.language_target = if lazy {
        LAZY
    } else {
        TargetVersion {
            major: PythonMajor(3),
            minor: PythonMinor(10),
        }
    };
    let Some(scratch) = evaluated(&tree, &model) else {
        return Ok("unknown");
    };
    let mut groups: Vec<String> = Vec::new();
    let mut current = None;
    for row in &scratch.members {
        if current != Some(row.root) {
            current = Some(row.root);
            let evaluation = match row.evaluation {
                Evaluation::Eager => "eager",
                Evaluation::Postponed => "postponed",
                Evaluation::Deferred => "deferred",
                Evaluation::Unevaluated => "unevaluated",
            };
            let mode = match row.mode {
                Mode::Lazy => "lazy",
                Mode::Eager => "eager",
            };
            groups.push(format!("{evaluation}/{mode} [{}]:", written(&tree, row.root).0));
        }
        let text = written(&tree, row.id).0;
        if !text.trim().is_empty()
            && let Some(last) = groups.last_mut()
        {
            last.push(' ');
            last.push_str(text);
        }
    }
    // A test-only rendering: leaked so the cases table compares it as a static string.
    Ok(Box::leak(groups.join(" | ").into_boxed_str()))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
