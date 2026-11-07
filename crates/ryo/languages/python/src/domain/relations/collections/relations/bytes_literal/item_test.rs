//! Missing payload is partial only when the optional bytes relation is demanded.

use crate::cases::{Case, CaseName, Expect, Word, run};
use super::super::fixtures::contract::check;
use super::item::BytesLiteralItem;
use crate::domain::{
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
    tokens::types::Token,
};
use crate::fixtures::reading::reading;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::syntax::{
    interface::SyntaxPort,
    types::{PythonSourceKind, SourceText},
};
use ryo::languages::{self, ByteSlice, TableBuilder};
use std::sync::Arc;

/// Deliberately incomplete parser scalar payload, retaining a real bytes token and interval.
struct MissingPayload(Option<&'static str>, Option<&'static str>);
impl SyntaxPort for MissingPayload {
    fn expand_annotations(
        &self,
        _source: SourceText<'_>,
        carriers: &[crate::outbound::syntax::types::DeferredAnnotation],
        _builder: &mut ryo::languages::TableBuilder<'_>,
    ) {
        assert!(
            carriers.is_empty(),
            "this syntax fixture supplies no deferred type carrier"
        );
    }

    fn read(
        &self,
        _text: SourceText<'_>,
        _source_kind: PythonSourceKind,
        builder: &mut TableBuilder<'_>,
    ) {
        languages::set_syntax_acceptance(builder, languages::SyntaxAcceptance::Accepted);
        languages::enter_node(
            builder,
            kind_bits(Kind::Node(Node::Module)),
            languages::Offset(0),
        );
        languages::enter_node(
            builder,
            kind_bits(Kind::Node(Node::Constant)),
            languages::Offset(0),
        );
        if let Some(kind) = self.0 {
            languages::node_property(
                builder,
                crate::domain::syntax::constants::SCALAR_KIND,
                languages::NameText(kind),
            );
        }
        if let Some(value) = self.1 {
            languages::node_property(
                builder,
                crate::domain::syntax::constants::SCALAR_VALUE,
                languages::NameText(value),
            );
        }
        languages::token(
            builder,
            kind_bits(Kind::Token(Token::String)),
            languages::Offset(0),
            languages::Offset(3),
        );
        languages::leave_node(builder, languages::Offset(3));
        languages::leave_node(builder, languages::Offset(3));
    }
}

fn decoded_bytes_keep_octets_and_join_adjacent_pieces() {
    check::<BytesLiteralItem>();
    let result = reading::<BytesLiteralItem>("b'\\x00\\xff' b'A'\nb''\nbr'\\n'\n'not bytes'\n");
    assert!(result.causes.is_empty(), "{:?}", result.causes);
    let values: Vec<_> = result.rows.iter().map(|row| row[2]).collect();
    assert_eq!(
        values,
        [
            languages::contract::want_name("00ff41"),
            languages::contract::want_name(""),
            languages::contract::want_name("5c6e")
        ]
    );
}

fn unrequested_payload_is_unread_and_requested_missing_payload_is_partial() {
    for payload in [
        MissingPayload(None, None),
        MissingPayload(Some(crate::domain::syntax::constants::BYTES_SCALAR), None),
        MissingPayload(
            Some(crate::domain::syntax::constants::BYTES_SCALAR),
            Some("0fF0"),
        ),
        MissingPayload(
            Some(crate::domain::syntax::constants::BYTES_SCALAR),
            Some("f"),
        ),
    ] {
        let parse = ParseContract::new(
            crate::settings::PythonParserSettings::default(),
            Arc::new(payload),
            Arc::new(crate::outbound::notes::factory::postcard()),
        );
        for (wanted, partial) in [
            (&[][..], false),
            (&[languages::Relation::BytesLiteral][..], true),
        ] {
            let mut elements = languages::Elements::default();
            let mut symbols = languages::Symbols::default();
            let mut out = languages::contract::Collect::default();
            languages::contract::read(
                &parse,
                languages::ParseRequest {
                    path: languages::SourcePath("item.py"),
                    file: languages::FileId(0),
                    bytes: ByteSlice(b"b\"\""),
                    notes: languages::NoteRequest::Unwanted,
                    fields: languages::FieldRequest::Wanted,
                },
                &mut elements,
                &mut symbols,
                wanted,
                &mut out,
            )
            .expect("incomplete bytes data is represented as coverage");
            let written = languages::contract::written(
                out,
                symbols,
                (&[], b"b\"\""),
                (
                    languages::SourcePath("item.py"),
                    languages::Relation::BytesLiteral,
                ),
                &[],
            );
            assert_eq!(!written.causes.is_empty(), partial);
            assert!(written.rows.is_empty());
        }
    }
}

fn synthetic_fstring_text_is_not_a_primitive_bytes_payload() {
    let result = reading::<BytesLiteralItem>("''.strip(f'aa')\nf'{value}text'\nb'aa'\n");
    assert!(result.causes.is_empty(), "{:?}", result.causes);
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0][2], languages::contract::want_name("6161"));
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("decoded_bytes_keep_octets_and_join_adjacent_pieces"),
        word: Word::Succeeds,
        input: || decoded_bytes_keep_octets_and_join_adjacent_pieces,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unrequested_payload_is_unread_and_requested_missing_payload_is_partial"),
        word: Word::Succeeds,
        input: || unrequested_payload_is_unread_and_requested_missing_payload_is_partial,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("synthetic_fstring_text_is_not_a_primitive_bytes_payload"),
        word: Word::Succeeds,
        input: || synthetic_fstring_text_is_not_a_primitive_bytes_payload,
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}
