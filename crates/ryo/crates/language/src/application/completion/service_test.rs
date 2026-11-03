//! Real preparation/append algorithms: identity, interning, ordinary-provider and failure controls.
use super::service::*;
use crate::domain::{
    completion::service::completion,
    elements::{service::*, types::*},
    expansions::service::{append_embedded, write_embedded},
    relations::{constants::NO_SYMBOL, types::*},
    symbols::{
        service::text_of,
        types::{ByteSlice, Symbols},
    },
    terms::types::NameText,
};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Default)]
struct Model {
    preparations: usize,
    extensions: usize,
}
fn native(elements: &mut Elements, symbols: &mut Symbols) {
    let mut builder = table_builder(elements, symbols, ByteSlice(b"a"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    leave_node(&mut builder, Offset(1));
}
fn append_name(c: &mut Completion<'_>) {
    append_embedded(c, |builder| {
        write_embedded(
            builder,
            Position(0),
            ByteSlice(b"Name"),
            EmbeddedMapping::WholeHost,
            |logical| {
                enter_node(logical, KindBits(3), Offset(0));
                node_property(logical, PropertyId(0), NameText("Name"));
                token(logical, KindBits(4), Offset(0), Offset(4));
                leave_node(logical, Offset(4));
            },
        )
    });
}
#[test]
fn initialized_model_survives_append_and_decoded_tail_without_native_identity_changes() {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    native(&mut elements, &mut symbols);
    let native_end = real_count(&elements);
    let token_before = (
        kind_at(&elements, Position(1)),
        start_at(&elements, Position(1)),
        end_at(&elements, Position(1)),
        parent_at(&elements, Position(1)),
    );
    assert_eq!(
        with_file_model::<Model, _>(
            &mut completion(&mut elements, &mut symbols, ByteSlice(b"a"), FileId(7)),
            |_, _, m| m.preparations
        ),
        None
    );
    complete::<Model>(
        completion(&mut elements, &mut symbols, ByteSlice(b"a"), FileId(7)),
        "target=v1",
        true,
        |_, model| model.preparations += 1,
        |c| {
            assert_eq!(
                with_file_model::<Model, _>(c, |_, _, model| model.preparations),
                Some(1)
            );
            append_name(c);
            append_embedded(c, |_| {});
        },
        |_, from, model| {
            assert_eq!(from, Position(2));
            model.extensions += 1;
        },
    );
    assert_eq!(real_count(&elements), native_end);
    assert_eq!(
        token_before,
        (
            kind_at(&elements, Position(1)),
            start_at(&elements, Position(1)),
            end_at(&elements, Position(1)),
            parent_at(&elements, Position(1))
        )
    );
    let interned = text_at(&elements, Position(2));
    assert_ne!(interned, NO_SYMBOL);
    assert_eq!(
        text_of(&symbols, ByteSlice(b"a"), interned),
        Some(NameText("Name"))
    );
    let mut model = take_model::<Model>(&mut elements);
    prepare(
        &mut elements,
        ByteSlice(b"a"),
        FileId(7),
        "target=v1",
        &mut *model,
        |_, _| panic!("append must not rerun native resolution"),
    );
    assert_eq!((model.preparations, model.extensions), (1, 1));
    keep_model(&mut elements, model);
    assert_eq!(engine(&elements).decoded.len(), 1);
    assert_eq!(
        engine(&elements).decoded_intern_visits,
        1,
        "new decoded tail is visited once; empty append cannot revisit it"
    );
    assert_eq!(text_at(&elements, Position(2)), interned);
}
#[test]
fn configuration_and_equal_sized_new_population_cannot_reuse_old_readiness() {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    native(&mut elements, &mut symbols);
    let mut model = Model::default();
    for (configuration, expected) in [("v1", 1), ("v1", 1), ("v2", 2)] {
        prepare(
            &mut elements,
            ByteSlice(b"a"),
            FileId(0),
            configuration,
            &mut model,
            |_, m| m.preparations += 1,
        );
        assert_eq!(model.preparations, expected);
    }
    clear_elements(&mut elements);
    assert!(!engine(&elements).model_preparation.ready);
    native(&mut elements, &mut symbols);
    prepare(
        &mut elements,
        ByteSlice(b"a"),
        FileId(0),
        "v2",
        &mut model,
        |_, m| m.preparations += 1,
    );
    assert_eq!(model.preparations, 3);
}
#[test]
fn initialization_completion_and_incremental_failure_never_leave_a_prepared_marker() {
    for failing_phase in 0..3 {
        let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
        native(&mut elements, &mut symbols);
        let result = catch_unwind(AssertUnwindSafe(|| {
            complete::<Model>(
                completion(&mut elements, &mut symbols, ByteSlice(b"a"), FileId(0)),
                "v1",
                true,
                |_, m| {
                    assert_ne!(failing_phase, 0, "initialization failure");
                    m.preparations += 1;
                },
                |c| {
                    assert_ne!(failing_phase, 1, "completion failure");
                    append_name(c);
                },
                |_, _, _| {
                    assert_ne!(failing_phase, 2, "incremental mapping failure");
                },
            )
        }));
        assert!(result.is_err());
        assert!(!engine(&elements).model_preparation.ready);
        assert!(
            with_file_model::<Model, _>(
                &mut completion(&mut elements, &mut symbols, ByteSlice(b"a"), FileId(0)),
                |_, _, m| m.preparations
            )
            .is_none()
        );
    }
}
