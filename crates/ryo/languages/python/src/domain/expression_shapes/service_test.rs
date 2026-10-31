//! Independent AST identity examples; no reference linter supplies these expected classes.
use super::service::{clear, intern, prepare, shape_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::types::{ExpressionShapeIndex, ShapeHash, ShapeState, ShapeWord};
use crate::domain::{
    model::types::Declarations,
    nodes::types::Category,
    syntax::service::{category, written},
};
use crate::fixtures::reading::{initialize_model, parsed};
use ryo::languages::{self, ByteSlice, Tree};
type Input = (&'static str, &'static str, &'static str);
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "relocated literal values compare logically and inactive candidates stay hidden",
        ),
        word: Word::Succeeds,
        input: || ("embedding witness", "", ""),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("arbitrary precision integers preserve their exact magnitude"),
        word: Word::Succeeds,
        input: || {
            (
                "184467440737095516160\n0xA0000000000000000\n",
                "184467440737095516160",
                "0xA0000000000000000",
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unequal large integer payloads do not saturate"),
        word: Word::Succeeds,
        input: || {
            (
                "184467440737095516160\n184467440737095516161\n",
                "184467440737095516160",
                "184467440737095516161",
            )
        },
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("large binary integer normalizes to exact decimal magnitude"),
        word: Word::Succeeds,
        input: || {
            (
                "18446744073709551616\n0b10000000000000000000000000000000000000000000000000000000000000000\n",
                "18446744073709551616",
                "0b10000000000000000000000000000000000000000000000000000000000000000",
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("large octal integer normalizes to exact decimal magnitude"),
        word: Word::Succeeds,
        input: || {
            (
                "18446744073709551616\n0o2000000000000000000000\n",
                "18446744073709551616",
                "0o2000000000000000000000",
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("large hexadecimal separators do not change magnitude"),
        word: Word::Succeeds,
        input: || {
            (
                "18446744073709551616\n0x1_0000_0000_0000_0000\n",
                "18446744073709551616",
                "0x1_0000_0000_0000_0000",
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("float spelling normalizes through typed bits"),
        word: Word::Succeeds,
        input: || ("1e0\n1.0\n", "1e0", "1.0"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("complex literal remains distinct from real float"),
        word: Word::Succeeds,
        input: || ("1j\n1.0\n", "1j", "1.0"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("decoded byte escapes normalize"),
        word: Word::Succeeds,
        input: || ("b\"a\"\nb\"\\x61\"\n", "b\"a\"", "b\"\\x61\""),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("semantic Name in a simple annotation matches native identity"),
        word: Word::Succeeds,
        input: || ("x\nvalue: \"x\"\n", "x", "x"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("nested semantic expression ignores formatting"),
        word: Word::Succeeds,
        input: || ("a+b\nvalue: \"a + b\"\n", "a+b", "a + b"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("lambda parameter categories preserve positional only separator"),
        word: Word::Succeeds,
        input: || {
            (
                "lambda x: x\nlambda x, /: x\n",
                "lambda x: x",
                "lambda x, /: x",
            )
        },
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("comprehension async flag is structural"),
        word: Word::Succeeds,
        input: || {
            (
                "[x for x in xs]\n[x async for x in xs]\n",
                "[x for x in xs]",
                "[x async for x in xs]",
            )
        },
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("full key comparison survives deliberately colliding hashes"),
        word: Word::Succeeds,
        input: || ("collision witness", "", ""),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("valid invalid empty valid reparses cannot leak classes"),
        word: Word::Succeeds,
        input: || ("reparse witness", "", ""),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("redundant parentheses and whitespace are not expression fields"),
        word: Word::Succeeds,
        input: || ("(a + b)\na+b\n", "a + b", "a+b"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("operand order remains significant"),
        word: Word::Succeeds,
        input: || ("a+b\nb+a\n", "a+b", "b+a"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("operator remains significant"),
        word: Word::Succeeds,
        input: || ("a+b\na-b\n", "a+b", "a-b"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("read and write contexts are ignored"),
        word: Word::Succeeds,
        input: || ("x=0\nx\n", "x", "x"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("NFKC identifiers share lexical identity"),
        word: Word::Succeeds,
        input: || ("𝗲𝘅𝗲𝗰\nexec\n", "𝗲𝘅𝗲𝗰", "exec"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("small integer radix is normalized"),
        word: Word::Succeeds,
        input: || ("16\n0x10\n", "16", "0x10"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("integer and floating literal kinds are distinct"),
        word: Word::Succeeds,
        input: || ("1\n1.0\n", "1", "1.0"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("boolean is not an integer shape"),
        word: Word::Succeeds,
        input: || ("True\n1\n", "True", "1"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("implicit string concatenation is one decoded literal"),
        word: Word::Succeeds,
        input: || ("'a' 'b'\n'ab'\n", "'a' 'b'", "'ab'"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("bytes and text are distinct literal kinds"),
        word: Word::Succeeds,
        input: || ("b'ab'\n'ab'\n", "b'ab'", "'ab'"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("call positional and keyword fields are grouped independently"),
        word: Word::Succeeds,
        input: || ("f(k=1,*xs)\nf(*xs,k=1)\n", "f(k=1,*xs)", "f(*xs,k=1)"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("keyword order is preserved within the keyword field"),
        word: Word::Succeeds,
        input: || ("f(a=1,b=2)\nf(b=2,a=1)\n", "f(a=1,b=2)", "f(b=2,a=1)"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("dictionary unpack positions are significant"),
        word: Word::Succeeds,
        input: || ("{'a':x,**y}\n{**x,'a':y}\n", "{'a':x,**y}", "{**x,'a':y}"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("slice omitted fields do not collapse"),
        word: Word::Succeeds,
        input: || ("x[:a]\nx[a:]\n", "x[:a]", "x[a:]"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("formatted string conversion is significant"),
        word: Word::Succeeds,
        input: || ("f'{x!s}'\nf'{x!r}'\n", "f'{x!s}'", "f'{x!r}'"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("formatted string empty format is not absent format"),
        word: Word::Succeeds,
        input: || ("f'{x}'\nf'{x:}'\n", "f'{x}'", "f'{x:}'"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("formatted string debug source contributes to its value"),
        word: Word::Succeeds,
        input: || ("f'{x=}'\nf'{x =}'\n", "f'{x=}'", "f'{x =}'"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("formatted string adjacent literal segments merge"),
        word: Word::Succeeds,
        input: || ("f'{x}a' 'b'\nf'{x}ab'\n", "f'{x}a' 'b'", "f'{x}ab'"),
        expect: Expect::Value(true),
    },
];
fn subject((source, left, right): Input) -> Result<Value, Failure> {
    if source == "embedding witness" {
        return embedded_shapes();
    }
    if source == "collision witness" {
        let mut index = ExpressionShapeIndex::default();
        index
            .scratch
            .push(ShapeWord::Identifier(languages::Symbol(1)));
        let a = intern(&mut index, languages::Position(2), ShapeHash(0));
        index.scratch.clear();
        index
            .scratch
            .push(ShapeWord::Identifier(languages::Symbol(2)));
        let b = intern(&mut index, languages::Position(6), ShapeHash(0));
        index.scratch.clear();
        index
            .scratch
            .push(ShapeWord::Identifier(languages::Symbol(1)));
        let again = intern(&mut index, languages::Position(9), ShapeHash(0));
        return Ok(a != b && a == again && index.records.len() == 2);
    }
    if source == "reparse witness" {
        let mut model = Declarations::default();
        for (source, complete) in [
            ("x\nx\n", true),
            ("x +\ny *\n", false),
            ("", true),
            ("y\nx\n", true),
        ] {
            clear(model.expression_shapes.get_mut());
            let file = parsed(source.as_bytes()).expect("parser retains diagnostics");
            let tree = Tree {
                elements: &file.elements,
                source: ByteSlice(file.bytes),
                file: languages::FileId(0),
            };
            let mut symbols = file.symbols;
            initialize_model(&tree, &mut model);
            assert_eq!(prepare(&tree, &model, &mut symbols).is_some(), complete);
            if !complete {
                assert!(model.expression_shapes.borrow().classes.is_empty());
                assert_eq!(model.expression_shapes.borrow().state, ShapeState::Partial);
            }
        }
        return Ok(true);
    }

    let file = parsed(source.as_bytes()).expect("independently authored source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut symbols = file.symbols;
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    assert!(
        model.expression_shapes.borrow().classes.is_empty(),
        "parsing alone performs no structural indexing"
    );
    let first = crate::domain::semantic_syntax::service::positions(&tree)
        .find(|at| category(&tree, *at) == Category::Expression && written(&tree, *at).0 == left)
        .expect("left occurrence");
    let last = crate::domain::semantic_syntax::service::positions(&tree)
        .filter(|at| category(&tree, *at) == Category::Expression && written(&tree, *at).0 == right)
        .last()
        .expect("right occurrence");
    let index = prepare(&tree, &model, &mut symbols).unwrap_or_else(|| {
        panic!("complete source has shapes: {source:?}; left={left:?}; right={right:?}")
    });
    let a = shape_of(&index, first).expect("left shape");
    let b = shape_of(&index, last).expect("right shape");
    let reentrant = prepare(&tree, &model, &mut symbols).expect("immutable read can reenter");
    assert_eq!(Some(a), shape_of(&reentrant, first));
    drop(reentrant);
    drop(index);
    let repeated = prepare(&tree, &model, &mut symbols).expect("repeat read is complete");
    assert_eq!(Some(a), shape_of(&repeated, first));
    drop(repeated);
    clear(&mut model.expression_shapes.borrow_mut());
    assert!(model.expression_shapes.borrow().classes.is_empty());
    let reparsed = prepare(&tree, &model, &mut symbols).expect("cleared index rebuilds");
    assert_eq!(Some(a), shape_of(&reparsed, first));
    Ok(a == b)
}

/// Whole-host relocation must not use carrier text as the embedded literal value.
fn embedded_shapes() -> Result<Value, Failure> {
    use crate::domain::{
        semantic_syntax::{
            service::{positions, visibility},
            types::Visibility,
        },
        syntax::constants::{BOOLEAN_SCALAR, SCALAR_KIND, SCALAR_VALUE},
    };
    let file = parsed(b"True\nvalue: \"tuple[Tr\\x75e, False]\"\nordinary('hidden + value')\n")
        .expect("valid independently authored source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut symbols = file.symbols;
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let expected: Vec<_> = positions(&tree)
        .map(|at| (at, visibility(&tree, &model, at, &mut symbols)))
        .collect();
    assert!(
        model.expression_shapes.borrow().classes.is_empty(),
        "semantic reads do not build structural keys"
    );
    let first = prepare(&tree, &model, &mut symbols).expect("complete semantic shapes");
    let values: Vec<_> = expected
        .iter()
        .filter_map(|(at, visible)| {
            if *visible != Visibility::Visible
                || languages::property_at(tree.elements, *at, SCALAR_KIND).map(|value| value.0)
                    != Some(BOOLEAN_SCALAR)
            {
                return None;
            }
            Some((
                languages::property_at(tree.elements, *at, SCALAR_VALUE)
                    .expect("boolean payload")
                    .0,
                shape_of(&first, *at).expect("visible boolean shape"),
            ))
        })
        .collect();
    let truths: Vec<_> = values
        .iter()
        .filter(|(value, _)| *value == "true")
        .map(|(_, shape)| *shape)
        .collect();
    let falsehoods: Vec<_> = values
        .iter()
        .filter(|(value, _)| *value == "false")
        .map(|(_, shape)| *shape)
        .collect();
    assert_eq!(truths.len(), 2);
    assert_eq!(falsehoods.len(), 1);
    assert_eq!(truths.first(), truths.last());
    assert_ne!(truths.first(), falsehoods.first());
    for (at, visible) in &expected {
        if *visible == Visibility::Hidden {
            assert!(shape_of(&first, *at).is_none());
        }
    }
    let classes = first.classes.clone();
    drop(first);
    initialize_model(&tree, &mut model);
    let shape_first = prepare(&tree, &model, &mut symbols)
        .expect("shape-first read initializes semantic visibility");
    assert_eq!(classes, shape_first.classes);
    Ok(true)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
