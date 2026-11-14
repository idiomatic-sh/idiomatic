//! A rule written in RyoQL that joins the elements with the edges naming them as children, keeping the edge's field, then joins that
//! with the parents the edges name, answers each edge's field: the text is compiled through the query-language port by the real
//! compiler, prepared, and file 0 run through the public file owner over a tree whose one child has a role.
use super::super::super::{fixtures, interface::*};
use crate::outbound::query_language::interface::QueryLanguagePort as _;
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_field, name_kind, write_fields};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;
use ryo_relations::domain::relations::constants::{NO_FIELD, NO_SYMBOL};
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_relations::domain::relations::types::{FieldId, FileId, KindBits, Offset, Position};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
};
use ryo_values::domain::query_tokens::types::{
    QueryBundle, QueryDocumentSource, QuerySourceId, QuerySourceText,
};
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use std::borrow::Cow;
use std::sync::Arc;

/// The rule: each element with a parent, its file, its parent and the role it has under the parent; then those rows joined with
/// the parent element, keeping only the first join's columns.
const RULE: &str = r#"let p = select l.id as id, l.file as file, r.parent as owner, r.field as field from "syntax-tree" as l join "syntax-child" as r on l.id = r.child;
let o = select l.id as id, l.file as file, l.owner as owner, l.field as field from p as l join "syntax-tree" as r on l.owner = r.id;
emit o subject file key(id);"#;

/// The place of `field` among the columns the rule emits: `id`, `file`, `owner`, `field`.
const FIELD: usize = 3;

/// What a `field` cell of the answer is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Role {
    /// The role `pat`, the one the tree gives.
    Pat,
    /// No role: the element has none under its parent.
    None,
    /// A role the tree does not give.
    Another,
    /// Not a role at all: a cell of another kind, such as a file.
    NotARole,
}
type Input = ();
type Value = Cow<'static, [Role]>;
type Failure = OperatingError;

/// A file whose root holds a node and a token, the node holding a token; the node, alone, has the role `pat` under the root.
fn tree() -> (
    Elements,
    Symbols,
    ryo_relations::domain::terms::types::Symbol,
) {
    let source = ByteSlice(b"ab");
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, source);
    enter_node(&mut builder, KindBits(1), Offset(0));
    enter_node(&mut builder, KindBits(3), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    leave_node(&mut builder, Offset(1));
    token(&mut builder, KindBits(2), Offset(1), Offset(2));
    leave_node(&mut builder, Offset(2));
    let mut done = completion(&mut elements, &mut symbols, source, FileId(0));
    for (bits, name) in [(1, "file"), (2, "word"), (3, "pattern")] {
        name_kind(&mut done, KindBits(bits), NameText(name));
    }
    write_fields(&mut done, |_, at| {
        if at == Position(1) {
            FieldId(1)
        } else {
            NO_FIELD
        }
    });
    name_field(&mut done, FieldId(1), NameText("pat"));
    let pat = intern_derived(&mut symbols, source, NameText("pat"));
    (elements, symbols, pat)
}

fn an_edge_field_kept_through_a_parent_lookup_is_the_edge_field(
    (): Input,
) -> Result<Value, Failure> {
    let compiler = crate::outbound::query_language::factory::use_case(
        Arc::new(CompileUseCaseImpl::new(
            Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
            Arc::new(ryo_query_language::outbound::catalog::factory::language()),
        )),
        Arc::new(TypecheckUseCaseImpl::new(
            Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
            Arc::new(ryo_query_language::outbound::catalog::factory::language()),
        )),
    );
    let rule = RuleSpec {
        id: RuleId("edge-field".into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId("edge-field".into()),
                text: QuerySourceText(RULE.into()),
            },
            modules: Default::default(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters("".into()),
        include: vec![],
        exclude: vec![],
    };
    let plan = compiler
        .compile(&rule.query, &[], &[])
        .expect("the rule compiles");
    let bindings = ryo_values::domain::rule_requests::service::bind(&plan, &rule)
        .expect("the rule has no parameters to bind");
    let (_engine, prepared) =
        fixtures::requests::prepared_many(vec![plan], vec![bindings], vec![])?;
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("lib.rs".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(2),
        modified: Nanoseconds(0),
    })];
    let request = fixtures::requests::execution().execution(&prepared, &corpus, None.into());
    let (elements, mut symbols, pat) = tree();
    let facts = fixtures::TableFacts::new(&[]);
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::DiagnosticAndObservations,
    }];
    let rules = fixtures::requests::run_file(
        &request,
        fixtures::requests::RunFile {
            source: b"ab",
            elements: &elements,
            intents: &intents,
            semantic: None,
        },
        &mut symbols,
        &facts,
    )
    .rules;
    request.finish();
    let table = match rules.into_iter().next().map(|rule| rule.emit) {
        Some(EmittedRoot::Delivered(RootOutcome::Complete(table))) => table,
        Some(EmittedRoot::Delivered(RootOutcome::Failed(error))) => return Err(error),
        Some(EmittedRoot::Delivered(_) | EmittedRoot::Withheld) | None => {
            panic!("the rule delivers its rows or its exact failure")
        }
    };
    let mut roles: Vec<Role> = table
        .cells
        .chunks(table.kinds.len())
        .map(|row| match row.get(FIELD) {
            Some(Cell::Symbol(symbol)) if *symbol == pat => Role::Pat,
            Some(Cell::Symbol(symbol)) if *symbol == NO_SYMBOL => Role::None,
            Some(Cell::Symbol(_)) => Role::Another,
            _ => Role::NotARole,
        })
        .collect();
    roles.sort();
    Ok(Cow::Owned(roles))
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an_edge_field_kept_through_a_parent_lookup_is_the_edge_field"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Cow::Borrowed(&[Role::Pat, Role::None, Role::None])),
}];
fn subject(input: Input) -> Result<Value, Failure> {
    an_edge_field_kept_through_a_parent_lookup_is_the_edge_field(input)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
