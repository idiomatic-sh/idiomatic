//! Independently authored annotation cases verify context, provenance, and physical spans.
use super::fixtures::{ReplaySample, reparse};
use super::service::{positions, visibility};
use super::types::Visibility;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    bindings::service::index,
    model::types::Declarations,
    nodes::types::{Category, Node},
    qualification::service::{prepare, qualify},
    scopes::{
        service::{declare, nth},
        types::CategoryIndex,
    },
    syntax::{service::kind, types::Kind},
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Position, Tree};
/// Source text only; no linter supplies expected facts.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// A fixed independent source example.
    Source(&'static str),
    /// Compare named native and embedded calls with explicit provenance categories.
    NamedCalls(&'static str, &'static str),
    /// One retained table and model move between unrelated files.
    Reparse,
}
/// What a row is given.
type Input = Given;
/// Qualified embedded calls and their physical byte intervals.
type Value = (usize, &'static str, u32, u32);
type Failure = ();
/// Each expectation is fixed independently of the parser and relation implementation.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("embedded lambda parameter shadows builtin"),
        word: Word::Succeeds,
        input: || Given::NamedCalls("x: \"(lambda exec: exec(code))(custom)\"\n", "exec"),
        expect: Expect::Value((1, "local:", 18, 28)),
    },
    Case {
        name: CaseName("embedded comprehension target shadows builtin"),
        word: Word::Succeeds,
        input: || Given::NamedCalls("x: \"[exec(code) for exec in values]\"\n", "exec"),
        expect: Expect::Value((1, "local:", 5, 15)),
    },
    Case {
        name: CaseName("embedded walrus target shadows builtin"),
        word: Word::Succeeds,
        input: || Given::NamedCalls("x: \"(exec := custom, exec(code))\"\n", "exec"),
        expect: Expect::Value((1, "local:", 21, 31)),
    },
    Case {
        name: CaseName("embedded assignments cannot change native builtin"),
        word: Word::Succeeds,
        input: || Given::NamedCalls("x: \"(exec := custom, exec(code))\"\nexec(code)\n", "exec"),
        expect: Expect::Value((2, "builtin:builtins.exec|local:", 34, 44)),
    },
    Case {
        name: CaseName("embedded assignment cannot hide native outer import"),
        word: Word::Succeeds,
        input: || {
            Given::NamedCalls(
                "def f():\n    x: \"(alias := custom, alias())\"\n    alias()\nfrom alpha import run as alias\n",
                "alias",
            )
        },
        expect: Expect::Value((2, "import:alpha.run|local:", 49, 56)),
    },
    Case {
        name: CaseName("embedded parameter default evaluates outer scope"),
        word: Word::Succeeds,
        input: || Given::NamedCalls("x: \"(lambda exec=exec(code): exec(code))()\"\n", "exec"),
        expect: Expect::Value((2, "builtin:builtins.exec|local:", 17, 27)),
    },
    Case {
        name: CaseName("comprehension first iterable evaluates outer scope"),
        word: Word::Succeeds,
        input: || Given::NamedCalls("x: \"[exec(code) for exec in exec(code)]\"\n", "exec"),
        expect: Expect::Value((2, "local:|builtin:builtins.exec", 5, 15)),
    },
    Case {
        name: CaseName("cast keyword type operand"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import cast\nimport alpha\nx = cast(typ=\"alpha.run()\", val=value)\n",
            )
        },
        expect: Expect::Value((1, "alpha.run", 51, 62)),
    },
    Case {
        name: CaseName("NewType positional operand"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import NewType\nimport alpha\nT = NewType(\"T\", \"alpha.run()\")\n",
            )
        },
        expect: Expect::Value((1, "alpha.run", 58, 69)),
    },
    Case {
        name: CaseName("NewType keyword operand"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import NewType\nimport alpha\nT = NewType(name=\"T\", tp=\"alpha.run()\")\n",
            )
        },
        expect: Expect::Value((1, "alpha.run", 66, 77)),
    },
    Case {
        name: CaseName("NewType name remains value"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import NewType\nimport alpha\nT = NewType(\"alpha.run()\", int)\n",
            )
        },
        expect: Expect::Value((0, "", 0, 0)),
    },
    Case {
        name: CaseName("TypeVar default type operand"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import TypeVar\nimport alpha\nT = TypeVar(\"T\", default=\"alpha.run()\")\n",
            )
        },
        expect: Expect::Value((1, "alpha.run", 66, 77)),
    },
    Case {
        name: CaseName("cast type operand"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import cast\nimport alpha\nx = cast(\"alpha.run()\", value)\n",
            )
        },
        expect: Expect::Value((1, "alpha.run", 47, 58)),
    },
    Case {
        name: CaseName("cast value operand"),
        word: Word::Succeeds,
        input: || {
            Given::Source("from typing import cast\nimport alpha\nx = cast(int, \"alpha.run()\")\n")
        },
        expect: Expect::Value((0, "", 0, 0)),
    },
    Case {
        name: CaseName("shadowed cast"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import cast\nimport alpha\ncast = custom\nx = cast(\"alpha.run()\", value)\n",
            )
        },
        expect: Expect::Value((0, "", 0, 0)),
    },
    Case {
        name: CaseName("type variable constraint"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import TypeVar\nimport alpha\nT = TypeVar(\"T\", \"alpha.run()\")\n",
            )
        },
        expect: Expect::Value((1, "alpha.run", 58, 69)),
    },
    Case {
        name: CaseName("type variable bound"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import TypeVar\nimport alpha\nT = TypeVar(\"T\", bound=\"alpha.run()\")\n",
            )
        },
        expect: Expect::Value((1, "alpha.run", 64, 75)),
    },
    Case {
        name: CaseName("type variable name is a value"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import TypeVar\nimport alpha\nT = TypeVar(\"alpha.run()\")\n",
            )
        },
        expect: Expect::Value((0, "", 0, 0)),
    },
    Case {
        name: CaseName("shadowed type variable constructor"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import TypeVar\nimport alpha\nTypeVar = custom\nT = TypeVar(\"T\", bound=\"alpha.run()\")\n",
            )
        },
        expect: Expect::Value((0, "", 0, 0)),
    },
    Case {
        name: CaseName("explicit type alias"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import TypeAlias\nimport alpha\nT: TypeAlias = \"alpha.run()\"\n",
            )
        },
        expect: Expect::Value((1, "alpha.run", 58, 69)),
    },
    Case {
        name: CaseName("shadowed type alias marker"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import TypeAlias\nimport alpha\nTypeAlias = custom\nT: TypeAlias = \"alpha.run()\"\n",
            )
        },
        expect: Expect::Value((0, "", 0, 0)),
    },
    Case {
        name: CaseName(
            "retained model forgets embedded identities across valid invalid empty and ordinary files",
        ),
        word: Word::Succeeds,
        input: || Given::Reparse,
        expect: Expect::Value((2, "fresh", 0, 0)),
    },
    Case {
        name: CaseName("simple content offsets"),
        word: Word::Succeeds,
        input: || Given::Source("import alpha\nx: \"alpha.run()\"\n"),
        expect: Expect::Value((1, "alpha.run", 17, 28)),
    },
    Case {
        name: CaseName("escaped logical tokens retain whole-host physical span"),
        word: Word::Succeeds,
        input: || Given::Source("import alpha\nx: \"alpha.r\\x75n()\"\n"),
        expect: Expect::Value((1, "alpha.run", 16, 32)),
    },
    Case {
        name: CaseName("concatenation retains whole-host physical span"),
        word: Word::Succeeds,
        input: || Given::Source("import alpha\nx: \"alpha.\" \"run()\"\n"),
        expect: Expect::Value((1, "alpha.run", 16, 32)),
    },
    Case {
        name: CaseName("quoted annotation resolves a later module import"),
        word: Word::Succeeds,
        input: || Given::Source("x: \"alpha.run()\"\nimport alpha\n"),
        expect: Expect::Value((1, "alpha.run", 4, 15)),
    },
    Case {
        name: CaseName("ordinary runtime strings are not expressions"),
        word: Word::Succeeds,
        input: || Given::Source("import alpha\nx = \"alpha.run()\"\n"),
        expect: Expect::Value((0, "", 0, 0)),
    },
    Case {
        name: CaseName("Literal arguments are values"),
        word: Word::Succeeds,
        input: || {
            Given::Source("from typing import Literal as L\nimport alpha\nx: L[\"alpha.run()\"]\n")
        },
        expect: Expect::Value((0, "", 0, 0)),
    },
    Case {
        name: CaseName("Annotated metadata is not a type"),
        word: Word::Succeeds,
        input: || {
            Given::Source(
                "from typing import Annotated as A\nimport alpha\nx: A[int, \"alpha.run()\"]\n",
            )
        },
        expect: Expect::Value((0, "", 0, 0)),
    },
    Case {
        name: CaseName("invalid quoted syntax gives no invented expression"),
        word: Word::Succeeds,
        input: || Given::Source("x: \"alpha.(\"\n"),
        expect: Expect::Value((0, "", 0, 0)),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let (source, selected) = match input {
        Given::Source(source) => (source, None),
        Given::NamedCalls(source, name) => (source, Some(name)),
        Given::Reparse => {
            return reparse(
                &[
                    ReplaySample(languages::NameText("x: \"one()\"\n"), languages::Count(1)),
                    ReplaySample(
                        languages::NameText("x: \"invalid [\"\n"),
                        languages::Count(0),
                    ),
                    ReplaySample(languages::NameText(""), languages::Count(0)),
                    ReplaySample(
                        languages::NameText("x = \"ordinary()\"\n"),
                        languages::Count(0),
                    ),
                    ReplaySample(languages::NameText("x: \"two()\"\n"), languages::Count(1)),
                ],
                languages::SourcePath("reused.py"),
            )
            .map(|total| (total.0, "fresh", 0, 0));
        }
    };
    let mut parsed = parsed(source.as_bytes()).map_err(|_error| ())?;
    let tree = Tree {
        elements: &parsed.elements,
        source: ByteSlice(parsed.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    declare(&tree, &mut model);
    index(&tree, &mut model);
    let provenance = prepare(&tree, &model);
    // Provenance requested before semantic visibility must agree after its lazy cache is populated.
    let before: Vec<_> = positions(&tree)
        .filter(|at| kind(&tree, *at) == Kind::Node(Node::Call))
        .filter_map(|at| nth(&tree, at, Category::Expression, CategoryIndex(0)))
        .map(|function| {
            (
                function,
                qualify(&tree, &model, &provenance, function, &mut parsed.symbols),
            )
        })
        .collect();
    let mut found = Vec::new();
    for at in positions(&tree) {
        if (selected.is_none()
            && !matches!(
                languages::origin_at(tree.elements, at),
                languages::ElementOrigin::Embedded { .. }
            ))
            || kind(&tree, at) != Kind::Node(Node::Call)
            || visibility(&tree, &model, at, &mut parsed.symbols) != Visibility::Visible
        {
            continue;
        }
        let function =
            nth(&tree, at, Category::Expression, CategoryIndex(0)).unwrap_or(Position(0));
        if selected.is_some_and(|name| {
            kind(&tree, function) != Kind::Node(Node::Name)
                || crate::domain::syntax::service::identifier_text(&tree, function).0 != name
        }) {
            continue;
        }
        let answer = qualify(&tree, &model, &provenance, function, &mut parsed.symbols);
        if before
            .iter()
            .find(|(occurrence, _)| *occurrence == function)
            .map(|(_, previous)| *previous)
            != Some(answer)
        {
            return Err(());
        }
        let path =
            languages::text_of(&parsed.symbols, tree.source, answer.path).map_or("", |text| text.0);
        let path = if selected.is_some() {
            let origin = languages::text_of(&parsed.symbols, tree.source, answer.provenance)
                .map_or("", |text| text.0);
            format!("{origin}:{path}")
        } else {
            path.to_owned()
        };
        found.push((
            path,
            languages::start_at(tree.elements, at).0,
            languages::end_at(tree.elements, at).0,
        ));
    }
    let count = found.len();
    let paths = found
        .iter()
        .map(|(path, _, _)| path.as_str())
        .collect::<Vec<_>>()
        .join("|");
    Ok(found
        .into_iter()
        .next()
        .map_or((0, "", 0, 0), |(_, start, end)| {
            (count, Box::leak(paths.into_boxed_str()), start, end)
        }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[cfg(test)]
mod comparisons_test;
#[cfg(test)]
mod nested_test;
