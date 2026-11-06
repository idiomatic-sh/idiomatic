//! Missing payload is partial only when the optional numeric relation is demanded.

use crate::cases::{Case, CaseName, Expect, Word, run};
use super::super::fixtures::contract::check;
use super::item::NumericLiteralItem;
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

/// Deliberately incomplete parser scalar payload, retaining a real numeric token and interval.
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
        _settings: &crate::settings::PythonParserSettings,
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
            kind_bits(Kind::Token(Token::Number)),
            languages::Offset(0),
            languages::Offset(1),
        );
        languages::leave_node(builder, languages::Offset(1));
        languages::leave_node(builder, languages::Offset(1));
    }
}

fn numeric_reader_has_valid_rows_and_preserves_validation_completeness() {
    check::<NumericLiteralItem>();
    let result = reading::<NumericLiteralItem>("nonlocal x\n0x10\n1.0\nTrue\n1j\n");
    assert!(result.causes.is_empty());
    assert_eq!(result.rows.len(), 2);
}

fn unrequested_payload_is_unread_and_requested_missing_payload_is_partial() {
    for payload in [
        MissingPayload(None, None),
        MissingPayload(Some(crate::domain::syntax::constants::INTEGER_SCALAR), None),
        MissingPayload(
            Some(crate::domain::syntax::constants::INTEGER_SCALAR),
            Some("0xff"),
        ),
        MissingPayload(
            Some(crate::domain::syntax::constants::FLOAT_SCALAR),
            Some("not-bits"),
        ),
    ] {
        let parse = ParseContract::new(
            crate::settings::PythonParserSettings::default(),
            Arc::new(payload),
            Arc::new(crate::outbound::notes::factory::postcard()),
        );
        for (wanted, partial) in [
            (&[][..], false),
            (&[languages::Relation::NumericLiteral][..], true),
        ] {
            let mut elements = languages::Elements::default();
            let mut symbols = languages::Symbols::default();
            let mut out = languages::contract::Collect::default();
            languages::contract::read(
                &parse,
                languages::ParseRequest {
                    path: languages::SourcePath("item.py"),
                    file: languages::FileId(0),
                    bytes: ByteSlice(b"1"),
                    notes: languages::NoteRequest::Unwanted,
                    fields: languages::FieldRequest::Wanted,
                },
                &mut elements,
                &mut symbols,
                wanted,
                &mut out,
            )
            .expect("incomplete numeric data is represented as coverage");
            let written = languages::contract::written(
                out,
                symbols,
                (&[], b"1"),
                (
                    languages::SourcePath("item.py"),
                    languages::Relation::NumericLiteral,
                ),
                &[],
            );
            assert_eq!(!written.causes.is_empty(), partial);
            assert!(written.rows.is_empty());
        }
    }
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("numeric_reader_has_valid_rows_and_preserves_validation_completeness"),
        word: Word::Succeeds,
        input: || numeric_reader_has_valid_rows_and_preserves_validation_completeness,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unrequested_payload_is_unread_and_requested_missing_payload_is_partial"),
        word: Word::Succeeds,
        input: || unrequested_payload_is_unread_and_requested_missing_payload_is_partial,
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
