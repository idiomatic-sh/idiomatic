//! Qualification fixture lifecycle: parse, preserve native tokens and reuse one model across unrelated files.
use super::{
    constants::BUILTIN_NAMES,
    service::{clear, prepare, qualify},
};
use crate::domain::{
    bindings::service::index,
    model::types::Declarations,
    nodes::types::Node,
    scopes::{
        service::{context, declare},
        types::Context,
    },
    syntax::{
        service::{kind, written},
        types::Kind,
    },
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};
type Input = (&'static str, &'static str);
type Value = (Option<&'static str>, &'static str, i64, i64);
type FixtureResult = Result<Value, String>;
fn observe(input: Input, model: &mut Declarations) -> FixtureResult {
    clear(model.qualification.get_mut());
    let file = parsed(input.0.as_bytes()).map_err(|error| error.to_string())?;
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    declare(&tree, model);
    index(&tree, model);
    let mut symbols = file.symbols;
    for token in languages::positions(tree.elements).filter(|at| {
        matches!(
            kind(&tree, *at),
            Kind::Token(crate::domain::tokens::types::Token::Name)
        )
    }) {
        assert_eq!(
            languages::text_of(
                &symbols,
                tree.source,
                languages::text_at(tree.elements, token)
            )
            .map(|value| value.0),
            Some(written(&tree, token).0)
        );
    }

    let at = languages::positions(tree.elements)
        .filter(|at| {
            matches!(kind(&tree, *at), Kind::Node(Node::Name | Node::Attribute))
                && context(&tree, *at) == Context::Load
                && written(&tree, *at).0 == input.1
        })
        .last()
        .ok_or_else(|| "fixture names a load expression".to_owned())?;
    let events = prepare(&tree, model);
    let repeated = prepare(&tree, model);
    assert_eq!(events.hazards, repeated.hazards);
    let answer = qualify(&tree, model, &events, at, &mut symbols);
    let provenance = languages::text_of(&symbols, tree.source, answer.provenance)
        .ok_or_else(|| "known provenance category".to_owned())?;
    Ok((
        languages::text_of(&symbols, tree.source, answer.path)
            .map(|text| &*Box::leak(text.0.to_owned().into_boxed_str())),
        Box::leak(provenance.0.to_owned().into_boxed_str()),
        answer.since.major.0,
        answer.since.minor.0,
    ))
}
pub(super) fn replay(input: Input) -> FixtureResult {
    assert_eq!(
        BUILTIN_NAMES.iter().map(|name| name.0).collect::<Vec<_>>(),
        include_str!("../../../tests/fixtures/standard/builtin_names.txt")
            .lines()
            .collect::<Vec<_>>()
    );
    let mut model = Declarations::default();
    let first = observe(input, &mut model)?;
    let _: Value = observe(("from m import *\nexec(x)\n", "exec"), &mut model)?;
    clear(model.qualification.get_mut());
    let empty = parsed(b"").map_err(|error| error.to_string())?;
    let tree = Tree {
        elements: &empty.elements,
        source: ByteSlice(empty.bytes),
        file: languages::FileId(0),
    };
    declare(&tree, &mut model);
    index(&tree, &mut model);
    assert!(prepare(&tree, &model).hazards.is_empty());
    assert_eq!(first, observe(input, &mut model)?);
    // Numeric target guards: a minor version of 10 must follow 9, not sort lexically before it.
    if input.1 == "aiter" || input.1 == "anext" {
        assert!((first.2, first.3) > (3, 9));
        assert!((first.2, first.3) <= (3, 10));
    }
    if input.1 == "ExceptionGroup" {
        assert!((first.2, first.3) > (3, 10));
        assert!((first.2, first.3) <= (3, 11));
    }
    Ok(first)
}
