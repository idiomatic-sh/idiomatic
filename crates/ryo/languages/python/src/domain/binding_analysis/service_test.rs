//! Composed static event analysis with independent source-level branch, timing and cleanup expectations.
use super::service::{build, indexed};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    binding_results::types::ReadState,
    model::types::{BindingAnalysisIndex, BindingAnalysisState, Declarations},
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};
type Input = (&'static str, &'static str);
type Value = &'static str;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("initializer before first store is explicitly unbound"),
        word: Word::Succeeds,
        input: || ("x = x\n", "x"),
        expect: Expect::Value("unbound:0"),
    },
    Case {
        name: CaseName("later value read sees a real assignment declaration"),
        word: Word::Succeeds,
        input: || ("x = 1\nuse(x)\n", "x"),
        expect: Expect::Value("unique:1"),
    },
    Case {
        name: CaseName("two bound branches remain two candidates without absent path"),
        word: Word::Succeeds,
        input: || ("if condition:\n    x = 1\nelse:\n    x = 2\nuse(x)\n", "x"),
        expect: Expect::Value("alternatives:2"),
    },
    Case {
        name: CaseName("bound versus deleted branches preserve absent path"),
        word: Word::Succeeds,
        input: || {
            (
                "x = 0\nif condition:\n    x = 1\nelse:\n    del x\nuse(x)\n",
                "x",
            )
        },
        expect: Expect::Value("unique:1|possibly-unbound:1"),
    },
    Case {
        name: CaseName("missing else preserves the prior real declaration"),
        word: Word::Succeeds,
        input: || ("x = 0\nif condition:\n    x = 1\nuse(x)\n", "x"),
        expect: Expect::Value("alternatives:2"),
    },
    Case {
        name: CaseName("deferred closure reads final outer declaration"),
        word: Word::Succeeds,
        input: || ("def f():\n    return x\nx = 1\n", "x"),
        expect: Expect::Value("unique:1"),
    },
    Case {
        name: CaseName("default read precedes later module declaration"),
        word: Word::Succeeds,
        input: || ("def f(arg=x):\n    return arg\nx = 1\n", "x"),
        expect: Expect::Value("unbound:0"),
    },
    Case {
        name: CaseName("handler cleanup does not leave its target initialized"),
        word: Word::Succeeds,
        input: || {
            (
                "try:\n    action()\nexcept Error as error:\n    use(error)\nafter(error)\n",
                "error",
            )
        },
        expect: Expect::Value("unique:1|unbound:0"),
    },
    Case {
        name: CaseName("module conditional builtin store retains versioned fallback"),
        word: Word::Succeeds,
        input: || ("if condition:\n    len = custom\nlen(items)\n", "len"),
        expect: Expect::Value("declaration-or-builtin:3.0:1"),
    },
    Case {
        name: CaseName("function conditional builtin-name store retains local absence"),
        word: Word::Succeeds,
        input: || {
            (
                "def f():\n    if condition:\n        len = custom\n    len(items)\n",
                "len",
            )
        },
        expect: Expect::Value("possibly-unbound:1"),
    },
    Case {
        name: CaseName("short circuit assignment is not unconditional"),
        word: Word::Succeeds,
        input: || ("condition and (x := 1)\nuse(x)\n", "x"),
        expect: Expect::Value("possibly-unbound:1"),
    },
    Case {
        name: CaseName("empty comprehension path preserves absent external walrus target"),
        word: Word::Succeeds,
        input: || ("[(x := item) for item in items]\nuse(x)\n", "x"),
        expect: Expect::Value("possibly-unbound:1"),
    },
    Case {
        name: CaseName(
            "loop read before later store retains real candidate and explicit uncertainty",
        ),
        word: Word::Succeeds,
        input: || ("for item in items:\n    use(x)\n    x = 1\n", "x"),
        expect: Expect::Value("uncertain:1"),
    },
    Case {
        name: CaseName(
            "loop-carried replacement preserves incoming and later declaration candidates",
        ),
        word: Word::Succeeds,
        input: || ("x = 0\nfor item in items:\n    use(x)\n    x = 1\n", "x"),
        expect: Expect::Value("uncertain:2"),
    },
    Case {
        name: CaseName("current iteration unconditional store makes its later read precise"),
        word: Word::Succeeds,
        input: || ("for item in items:\n    x = 1\n    use(x)\n", "x"),
        expect: Expect::Value("unique:1"),
    },
    Case {
        name: CaseName("for iterable read does not inherit later iteration bindings"),
        word: Word::Succeeds,
        input: || ("for item in x:\n    x = 1\n", "x"),
        expect: Expect::Value("unbound:0"),
    },
    Case {
        name: CaseName("handler may observe a partially executed try body"),
        word: Word::Succeeds,
        input: || {
            (
                "try:\n    x = compute()\n    action()\nexcept Error:\n    use(x)\n",
                "x",
            )
        },
        expect: Expect::Value("uncertain:1"),
    },
    Case {
        name: CaseName("handler-local store makes later handler use precise"),
        word: Word::Succeeds,
        input: || {
            (
                "try:\n    x = compute()\nexcept Error:\n    x = 1\n    use(x)\n",
                "x",
            )
        },
        expect: Expect::Value("unique:1"),
    },
    Case {
        name: CaseName("local bare annotation blocks outer value only after its declaration"),
        word: Word::Succeeds,
        input: || ("x = 1\ndef f():\n    use(x)\n    x: int\n    use(x)\n", "x"),
        expect: Expect::Value("unique:1|unbound:0"),
    },
    Case {
        name: CaseName("local bare annotation blocks builtin fallback after declaration"),
        word: Word::Succeeds,
        input: || {
            (
                "def f():\n    len(items)\n    len: int\n    len(items)\n",
                "len",
            )
        },
        expect: Expect::Value("builtin:3.0:0|unbound:0"),
    },
    Case {
        name: CaseName("module bare annotation preserves builtin fallback"),
        word: Word::Succeeds,
        input: || ("len: int\nlen(items)\n", "len"),
        expect: Expect::Value("builtin:3.0:0"),
    },
    Case {
        name: CaseName("module bare annotation does not initialize a missing value"),
        word: Word::Succeeds,
        input: || ("x: int\nuse(x)\n", "x"),
        expect: Expect::Value("unbound:0"),
    },
    Case {
        name: CaseName("annotation-only subscript target reads its base expression"),
        word: Word::Succeeds,
        input: || ("values[0]: 1\n", "values"),
        expect: Expect::Value("unbound:0"),
    },
    Case {
        name: CaseName("annotation-only attribute target reads its base expression"),
        word: Word::Succeeds,
        input: || ("container.value: int\n", "container"),
        expect: Expect::Value("unbound:0"),
    },
    Case {
        name: CaseName("assignment initializes a previously annotation-only local"),
        word: Word::Succeeds,
        input: || ("x = 0\ndef f():\n    x: int\n    x = 1\n    use(x)\n", "x"),
        expect: Expect::Value("unique:1"),
    },
    Case {
        name: CaseName("annotation does not replace an already initialized module value"),
        word: Word::Succeeds,
        input: || ("x = 1\nx: int\nuse(x)\n", "x"),
        expect: Expect::Value("unique:1"),
    },
    Case {
        name: CaseName("annotation does not replace an already initialized local value"),
        word: Word::Succeeds,
        input: || ("def f():\n    x = 1\n    x: int\n    use(x)\n", "x"),
        expect: Expect::Value("unique:1"),
    },
];
fn subject((source, wanted): Input) -> Result<Value, Failure> {
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
        Arc::new(ruff_python_parser()),
        Arc::new(postcard()),
    );
    let mut model = Declarations::default();
    parser.file_model(&tree, &mut model);
    let mut index = BindingAnalysisIndex::default();
    build(&tree, &model, &mut file.symbols, &mut index);
    if index.state != BindingAnalysisState::Complete {
        return Err(());
    }
    let first = index.clone();
    build(&tree, &model, &mut file.symbols, &mut index);
    assert_eq!(
        first, index,
        "rebuilding does not retain prior events or aliases"
    );
    let mut values = Vec::new();
    for read in &index.reads {
        if languages::text_of(&file.symbols, tree.source, read.name)
            .is_none_or(|name| name.0 != wanted)
        {
            continue;
        }
        let count = index
            .uses
            .iter()
            .filter(|usage| usage.reference == read.reference)
            .count();
        let label = match read.state {
            ReadState::UniqueDeclaration => "unique".to_owned(),
            ReadState::AlternativeDeclarations => "alternatives".to_owned(),
            ReadState::PossiblyUnbound => "possibly-unbound".to_owned(),
            ReadState::Unbound => "unbound".to_owned(),
            ReadState::Uncertain => "uncertain".to_owned(),
            ReadState::Builtin {
                since_major,
                since_minor,
            } => format!("builtin:{}.{}", since_major.0, since_minor.0),
            ReadState::DeclarationOrBuiltin {
                since_major,
                since_minor,
            } => format!("declaration-or-builtin:{}.{}", since_major.0, since_minor.0),
        };
        values.push(format!("{label}:{count}"));
    }
    Ok(Box::leak(values.join("|").into_boxed_str()))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    run(CHECKS, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}

fn retained_source_revisions_invalidate_complete_incomplete_and_empty_binding_results() {
    use crate::inbound::contracts::parse::contract::ParseContract;
    use crate::outbound::{notes::factory::postcard, syntax::factory::ruff_python_parser};
    use ryo::languages::Parse as _;
    use std::sync::Arc;
    let parser = ParseContract::new(
        crate::settings::PythonParserSettings::default(),
        Arc::new(ruff_python_parser()),
        Arc::new(postcard()),
    );
    let mut model = Declarations::default();
    let mut observed = Vec::new();
    for source in [
        "x = 1\nuse(x)\n",
        "if:\n    pass\n",
        "",
        "missing\n",
        "x = 1\nuse(x)\n",
    ] {
        let mut file = parsed(source.as_bytes()).expect("the retained revision parses");
        let tree = Tree {
            elements: &file.elements,
            source: ByteSlice(file.bytes),
            file: languages::FileId(0),
        };
        parser.file_model(&tree, &mut model);
        assert_eq!(
            model.binding_analysis.borrow().state,
            BindingAnalysisState::Fresh,
            "ordinary model preparation must not eagerly execute binding analysis"
        );
        let first = indexed(&tree, &model, &mut file.symbols).map(|index| index.reads.clone());
        let second = indexed(&tree, &model, &mut file.symbols).map(|index| index.reads.clone());
        assert_eq!(
            first, second,
            "reader request order must not duplicate or alter observations"
        );
        observed.push(first.map_or_else(
            || "incomplete".to_owned(),
            |reads| format!("complete:{}", reads.len()),
        ));
    }
    assert_eq!(
        observed.join("|"),
        "complete:2|incomplete|complete:0|complete:1|complete:2"
    );
}

/// Each check below states its own source and asserts its own outcome.
const CHECKS: &[Case<fn(), (), ()>] = &[Case {
    name: CaseName("retained source revisions invalidate complete incomplete and empty binding results"),
    word: Word::Succeeds,
    input: || retained_source_revisions_invalidate_complete_incomplete_and_empty_binding_results,
    expect: Expect::Value(()),
}];
