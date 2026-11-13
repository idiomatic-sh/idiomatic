//! The `imports` service's cases: for each alias of a parsed import, whether it imports everything, the name it binds, the path it
//! imports and the text its binding spans.

use super::service::{binding_at, import_binding, import_target, imported};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::names::types::Imported;
use crate::domain::nodes::types::Node;
use crate::domain::semantic_syntax::service::positions;
use crate::domain::syntax::{service::kind, service::written, types::Kind};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Symbol, Symbols, Tree};

/// A source of imports.
type Input = &'static str;
/// Each alias as `binds <- imports @ span`, `*` marking one that imports everything, joined by `|`.
type Value = &'static str;
/// A source the parser refuses.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("import_binds_the_first_part_of_a_dotted_module"),
        word: Word::Succeeds,
        input: || "import a.b\n",
        expect: Expect::Value("a <- a.b @ a"),
    },
    Case {
        name: CaseName("an_as_name_is_the_binding"),
        word: Word::Succeeds,
        input: || "import a.b as c\nfrom m import x as y\n",
        expect: Expect::Value("c <- a.b @ c|y <- m.x @ y"),
    },
    Case {
        name: CaseName("relative_imports_keep_their_dots"),
        word: Word::Succeeds,
        input: || "from ..m import x\nfrom . import x\nfrom ... import z\n",
        expect: Expect::Value("x <- ..m.x @ x|x <- .x @ x|z <- ...z @ z"),
    },
    Case {
        name: CaseName("a_wildcard_imports_its_module_and_binds_an_unknown"),
        word: Word::Succeeds,
        input: || "from m import *\n",
        expect: Expect::Value("*unknown <- m @ *"),
    },
];

/// A name as its text, or `unknown` for a name no source spells.
fn text(symbols: &Symbols, source: ByteSlice<'_>, name: Option<Symbol>) -> String {
    name.and_then(|each| languages::text_of(symbols, source, each))
        .map_or_else(|| "unknown".to_owned(), |each| each.0.to_owned())
}

/// Reads every alias of the source.
fn subject(source: Input) -> Result<Value, Failure> {
    let mut file = parsed(source.as_bytes()).map_err(|_| ())?;
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let aliases: Vec<_> = positions(&tree)
        .filter(|at| kind(&tree, *at) == Kind::Node(Node::Alias))
        .collect();
    let mut texts = Vec::new();
    for alias in aliases {
        let everything = if imported(&tree, alias) == Imported::Everything {
            "*"
        } else {
            ""
        };
        let binding = import_binding(&tree, alias);
        let target = import_target(&tree, &mut file.symbols, alias);
        texts.push(format!(
            "{everything}{} <- {} @ {}",
            text(&file.symbols, tree.source, binding),
            text(&file.symbols, tree.source, target),
            written(&tree, binding_at(&tree, alias)).0
        ));
    }
    Ok(Box::leak(texts.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
