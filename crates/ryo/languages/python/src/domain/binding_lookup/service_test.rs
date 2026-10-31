//! Independent namespace alternatives distinguish builtin fallback, real absence, and lexical ownership.
use super::service::resolve;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    binding_results::types::ReadState,
    binding_states::service::{assign, join, open, wildcard},
    definitions::service::definition,
    model::types::{BindingEnvironments, BindingKey, Declarations},
    nodes::types::Node,
    qualification::service::{prepare, qualify},
    scopes::{
        service::{context, enclosing},
        types::Context,
    },
    syntax::{
        service::{kind, name_of},
        types::Kind,
    },
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Integer, Position, Tree};
#[derive(Clone, Copy, Debug)]
enum Environment {
    BeforeStore,
    ConditionalStore,
    Wildcard,
}
type Input = (&'static str, &'static str, Environment);
type Value = (ReadState, usize, &'static str);
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("builtin association before local store differs from lexical ownership"),
        word: Word::Succeeds,
        input: || {
            (
                "def f():\n    len(items)\n    len = custom\n",
                "len",
                Environment::BeforeStore,
            )
        },
        expect: Expect::Value((
            ReadState::Builtin {
                since_major: Integer(3),
                since_minor: Integer(0),
            },
            0,
            "local",
        )),
    },
    Case {
        name: CaseName("module conditional store retains builtin fallback on absent arm"),
        word: Word::Succeeds,
        input: || {
            (
                "if condition:\n    len = custom\nlen(items)\n",
                "len",
                Environment::ConditionalStore,
            )
        },
        expect: Expect::Value((
            ReadState::DeclarationOrBuiltin {
                since_major: Integer(3),
                since_minor: Integer(0),
            },
            1,
            "local",
        )),
    },
    Case {
        name: CaseName("function local absent arm does not become builtin fallback"),
        word: Word::Succeeds,
        input: || {
            (
                "def f():\n    if condition:\n        len = custom\n    len(items)\n",
                "len",
                Environment::ConditionalStore,
            )
        },
        expect: Expect::Value((ReadState::PossiblyUnbound, 1, "local")),
    },
    Case {
        name: CaseName("ordinary module absent arm remains possibly unbound"),
        word: Word::Succeeds,
        input: || {
            (
                "if condition:\n    value = custom\nuse(value)\n",
                "value",
                Environment::ConditionalStore,
            )
        },
        expect: Expect::Value((ReadState::PossiblyUnbound, 1, "local")),
    },
    Case {
        name: CaseName("newer builtin carries availability rather than current-target assumption"),
        word: Word::Succeeds,
        input: || {
            (
                "ExceptionGroup(message, errors)\n",
                "ExceptionGroup",
                Environment::BeforeStore,
            )
        },
        expect: Expect::Value((
            ReadState::Builtin {
                since_major: Integer(3),
                since_minor: Integer(11),
            },
            0,
            "builtin",
        )),
    },
    Case {
        name: CaseName("builtin minor versions remain numeric across 3.9 and 3.10"),
        word: Word::Succeeds,
        input: || ("aiter(items)\n", "aiter", Environment::BeforeStore),
        expect: Expect::Value((
            ReadState::Builtin {
                since_major: Integer(3),
                since_minor: Integer(10),
            },
            0,
            "builtin",
        )),
    },
    Case {
        name: CaseName("namespace-wide imports leave known builtin association intact"),
        word: Word::Succeeds,
        input: || {
            (
                "from m import *\nexec(code)\n",
                "exec",
                Environment::Wildcard,
            )
        },
        expect: Expect::Value((
            ReadState::Builtin {
                since_major: Integer(3),
                since_minor: Integer(0),
            },
            0,
            "builtin",
        )),
    },
    Case {
        name: CaseName("namespace-wide import uncertainty is not proven unbound"),
        word: Word::Succeeds,
        input: || {
            (
                "from m import *\nmissing()\n",
                "missing",
                Environment::Wildcard,
            )
        },
        expect: Expect::Value((ReadState::Uncertain, 0, "ambiguous")),
    },
];
fn subject((source, wanted, environment): Input) -> Result<Value, Failure> {
    use crate::inbound::contracts::parse::contract::ParseContract;
    use crate::outbound::{notes::factory::postcard, syntax::factory::ruff_python_parser};
    use ryo::languages::Parse as _;
    use std::sync::Arc;
    let mut file = parsed(source.as_bytes()).map_err(|_| ())?;
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let parser = ParseContract::new(Arc::new(ruff_python_parser()), Arc::new(postcard()));
    let mut model = Declarations::default();
    parser.file_model(&tree, &mut model);
    let reference = languages::positions(tree.elements)
        .find(|at| {
            kind(&tree, *at) == Kind::Node(Node::Name)
                && context(&tree, *at) == Context::Load
                && name_of(&tree, *at)
                    .and_then(|name| languages::text_of(&file.symbols, tree.source, name))
                    .is_some_and(|name| name.0 == wanted)
        })
        .ok_or(())?;
    let name = name_of(&tree, reference).ok_or(())?;
    let scope = enclosing(&tree, reference);
    let mut environments = BindingEnvironments::default();
    let frame = open(&mut environments, None, Position(0));
    if let Environment::ConditionalStore = environment {
        let declaration = languages::positions(tree.elements)
            .find(|at| definition(&tree, &model, *at).is_some_and(|value| value.name == name))
            .ok_or(())?;
        let arm = open(&mut environments, Some(frame), Position(0));
        assign(
            &mut environments,
            arm,
            BindingKey { scope, name },
            declaration,
        );
        join(&mut environments, frame, &[arm, frame]);
    }
    if let Environment::Wildcard = environment {
        wildcard(&mut environments, frame, scope);
    }
    let mut candidates = Vec::new();
    let observed = resolve(
        &tree,
        &model,
        &environments,
        frame,
        reference,
        name,
        &mut candidates,
    );
    let index = prepare(&tree, &model);
    let lexical = qualify(&tree, &model, &index, reference, &mut file.symbols);
    let lexical = languages::text_of(&file.symbols, tree.source, lexical.provenance).ok_or(())?;
    Ok((
        observed,
        candidates.len(),
        Box::leak(lexical.0.to_owned().into_boxed_str()),
    ))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
