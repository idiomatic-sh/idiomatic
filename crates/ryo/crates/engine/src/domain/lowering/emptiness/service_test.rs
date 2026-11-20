//! The cases of the `emptiness` part: the formula of each operation, one row per operation, over inputs whose formulas are
//! three rows of three filled relations; and the formula of a relation read with a literal.

use super::service::{derived, literal_read, relation_read};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::fixtures::growth::growth_plan;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::collections::relations::syntax_error::item::SyntaxErrorColumn;
use ryo_relations::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
use crate::domain::steps::types::AnchorKinds;
use ryo_relations::domain::relations::types::{Column, KindBits, Relation};
use ryo_values::domain::plans::service::{inputs_of, kind_of, named};
use ryo_values::domain::query_operators::types::OperationKind;
use std::borrow::Cow;

/// The kind of operation whose formula a row asks.
type Input = OperationKind;
/// The formula, said: its debug text.
type Value = Cow<'static, str>;
type Failure = ();

const fn row(name: &'static str, kind: fn() -> Input, said: &'static str) -> Case<Input, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Succeeds,
        input: kind,
        expect: Expect::Value(Cow::Borrowed(said)),
    }
}

/// The rows of this part's cases: filled in from a run.
const CASES: &[Case<Input, Value, Failure>] = &[row("Select", || OperationKind::Select, "")];

/// The formula of the first step of `kind` in its growth plan, its inputs' formulas being a row of `Function`, of `Comment`
/// and of `Scope`, in input order, as many as the operation reads.
fn subject(kind: Input) -> Result<Value, Failure> {
    let plan = growth_plan(kind);
    let step = plan.steps.iter().find(|step| kind_of(&step.operation) == kind).ok_or(())?;
    let inputs = [Relation::Function, Relation::Comment, Relation::Scope];
    let read = named(&inputs_of(&step.operation)).len();
    let formulas = inputs.into_iter().take(read).map(relation_read).collect();
    Ok(Cow::Owned(format!("{:?}", derived(&step.operation, formulas))))
}

#[test]
fn succeeds() {
    if CASES.len() < 2 {
        for kind in OperationKind::ALL {
            println!("    row(\"{kind:?}\", || OperationKind::{kind:?}, {:?}),", subject(*kind).unwrap_or_default());
        }
    }
    run(CASES, Word::Succeeds, subject);
}

/// A relation read whole, and with a literal: a kind of the node table when the vocabulary names it, a row of a filled
/// relation holding the value.
#[test]
fn a_literal_read_names_its_kind_or_its_row() {
    let kind = Column::SyntaxTree(SyntaxTreeColumn::Kind);
    let code = Column::SyntaxError(SyntaxErrorColumn::Id);
    let said = [
        format!("{:?}", relation_read(Relation::SyntaxTree)),
        format!("{:?}", relation_read(Relation::SyntaxError)),
        format!("{:?}", literal_read(Relation::SyntaxTree, kind, CellWord(3), None)),
        format!("{:?}", literal_read(Relation::SyntaxError, code, CellWord(3), None)),
    ];
    let expected = [
        "Unknown",
        "Row(SyntaxError, None)",
        "Unknown",
        "Row(SyntaxError, Some(RowValue { column: SyntaxError(Id), value: CellWord(3) }))",
    ];
    assert_eq!(said, expected);
    let kinds = AnchorKinds([KindBits(4)].into());
    let named = literal_read(Relation::SyntaxTree, kind, CellWord(3), Some(kinds));
    assert_eq!(format!("{named:?}"), "Kind(AnchorKinds([KindBits(4)]))");
}
