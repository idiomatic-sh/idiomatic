//! Linking consumes parsed programs and a bundle of sources, calling no parser.

use std::collections::BTreeMap;

use super::errors::LinkingError;
use super::service::link;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo_values::domain::program::service::{document_source, query_bundle, source_id};
use ryo_values::domain::program::types::{
    ByteOffset, ColumnName, ImportDeclaration, Program, QuerySpan, StepName, WrittenEmit,
    WrittenLiteral, WrittenName,
};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A program with nothing to expand is linked as it is.
    NothingToExpand,
    /// An import of a module the bundle does not hold.
    ImportOfAbsentModule,
    /// Two imports under one alias.
    DuplicateAlias,
}

/// Whether the program came back linked.
type Value = bool;
type Failure = LinkingError;
type Input = Scenario;

const SPAN: QuerySpan = QuerySpan {
    start: ByteOffset(3),
    end: ByteOffset(11),
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_program_with_nothing_to_expand_is_linked"),
        word: Word::Succeeds,
        input: || Scenario::NothingToExpand,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_import_of_an_absent_module_is_refused_with_its_span"),
        word: Word::Refuses,
        input: || Scenario::ImportOfAbsentModule,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LinkingError::Located { origin, .. } if origin.span == SPAN))
        }),
    },
    Case {
        name: CaseName("two_imports_under_one_alias_are_refused_with_the_second_span"),
        word: Word::Refuses,
        input: || Scenario::DuplicateAlias,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LinkingError::Located { origin, .. } if origin.span == SPAN))
        }),
    },
];

fn import(alias: &str, module: &str) -> ImportDeclaration {
    ImportDeclaration {
        alias: WrittenName(alias.into()),
        module: WrittenLiteral(module.into()),
        span: SPAN,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut program = Program {
        linked: false,
        imports: Vec::new(),
        origins: Vec::new(),
        fragment_contracts: Vec::new(),
        step_origins: BTreeMap::new(),
        observations: Vec::new(),
        emit_origin: None,
        parameters: Vec::new(),
        steps: Vec::new(),
        emit: WrittenEmit {
            step: StepName("rows".into(), None),
            subject: ColumnName("file".into()),
            key: vec![ColumnName("span".into())],
            span: SPAN,
        },
    };
    match scenario {
        Scenario::NothingToExpand => {}
        Scenario::ImportOfAbsentModule => program.imports = vec![import("absent", "absent")],
        Scenario::DuplicateAlias => {
            program.imports = vec![import("same", "one"), import("same", "two")];
        }
    }
    let root = document_source(source_id("root").expect("a named source"), "");
    let bundle = query_bundle(root, []).expect("a bundle of one source");
    link(&bundle, program, &BTreeMap::new()).map(|linked| linked.linked)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
