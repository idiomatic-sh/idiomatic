//! The `table_scratch` service's cases, on values alone.

use ryo_language::domain::relations::types::KindBits;

use super::service::kind_has;
use super::types::{KindMask, KindSet};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_model::types::SetMembership;

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, ()>;
/// What a scenario answers: a yes as one.
type Value = usize;
/// This part has no failure of its own.
type Failure = ();

/// A kind is in a set when its bit is, and a kind beyond the set is out.
fn a_kind_is_in_the_set_by_its_bit() -> Value {
    let set = KindSet {
        bits: KindMask(vec![0b100]),
        kinds: vec![KindBits(2)],
    };
    usize::from(
        kind_has(&set, KindBits(2)) == SetMembership::In
            && kind_has(&set, KindBits(3)) == SetMembership::Out
            && kind_has(&set, KindBits(130)) == SetMembership::Out,
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_kind_is_in_the_set_by_its_bit"),
    word: Word::Succeeds,
    input: || || Ok(a_kind_is_in_the_set_by_its_bit()),
    expect: Expect::Value(1),
}];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// Count actual heavy-group construction without allocating or inspecting their payloads.
fn initialized(scratch: &super::types::OperatorScratch) -> usize {
    [
        scratch.select_scratch.0.get().is_some(),
        scratch.join_scratch.0.get().is_some(),
        scratch.walk_scratch.0.get().is_some(),
        scratch.groups_scratch.0.get().is_some(),
        scratch.capture_scratch.0.get().is_some(),
        scratch.cycle_scratch.0.get().is_some(),
    ]
    .into_iter()
    .filter(|made| *made)
    .count()
}

/// Pure projection needs only cheap rows; distinct projection constructs just its membership group.
#[test]
fn projection_constructs_only_the_group_it_uses_and_keeps_bags() {
    use crate::domain::operating::types::OperatorContext;
    use crate::domain::operator::collections::operators::interface::OperatorsItem as _;
    use crate::domain::operator::collections::operators::project::item::ProjectItem;
    use crate::domain::table_model::service::push_row;
    use crate::domain::table_model::types::{Rows, Table};
    use crate::domain::table_texts::types::SourceView;
    use ryo_language::domain::relations::types::{CellKind, Integer, Relation, Value};
    use ryo_language::domain::symbols::types::Symbols;
    use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, RepeatedRows};
    use ryo_values::domain::plans::types::Operation;
    let mut input = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    for number in [2, 1, 2] {
        push_row(&mut input, &[Value::Number(Integer(number))]);
    }
    let mut scratch = super::types::OperatorScratch::default();
    assert_eq!(initialized(&scratch), 0);
    let mut symbols = Symbols::default();
    let mut memo = Vec::new();
    for (distinct, expected, groups) in [
        (RepeatedRows::Kept, vec![2, 1, 2], 0),
        (RepeatedRows::Dropped, vec![2, 1], 1),
    ] {
        let mut out = Table::default();
        let mut context = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            stored_indexes: None,
            bindings: &[],
            symbols: &mut symbols,
            source: SourceView(&[]),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        ProjectItem
            .compute(
                &Operation::Project {
                    from: Input::Relation(Relation::Function),
                    columns: vec![ColumnIndex(0)],
                    distinct,
                },
                &[Rows::Computed(&input)],
                &mut context,
                &mut out,
            )
            .unwrap();
        assert_eq!(
            out.cells,
            expected
                .into_iter()
                .map(|n| Value::Number(Integer(n)))
                .collect::<Vec<_>>()
        );
        assert_eq!(initialized(&scratch), groups);
    }
    // A new rule owns a fresh workspace; no idle group or large payload crosses the boundary.
    assert_eq!(initialized(&super::types::OperatorScratch::default()), 0);
}

/// A failed dynamic row pattern initializes only select work; an empty input never compiles it.
#[test]
fn dynamic_regex_initialization_is_execution_local_and_lazy() {
    use crate::domain::operating::types::OperatorContext;
    use crate::domain::operator::collections::operators::interface::OperatorsItem as _;
    use crate::domain::operator::collections::operators::select::item::SelectItem;
    use crate::domain::table_model::service::push_row;
    use crate::domain::table_model::types::{Rows, Table};
    use crate::domain::table_texts::types::SourceView;
    use ryo_language::domain::relations::types::{CellKind, Relation, Value};
    use ryo_language::domain::symbols::service::intern_derived;
    use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
    use ryo_language::domain::terms::types::NameText;
    use ryo_values::domain::plan_parts::types::{ColumnIndex, Comparison, Input, Operand, Test};
    use ryo_values::domain::plans::types::Operation;
    let operation = Operation::Select {
        from: Input::Relation(Relation::Comment),
        comparisons: vec![Comparison {
            test: Test::Matches,
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Column(ColumnIndex(0)),
            origin: None,
        }],
        kept: vec![ColumnIndex(0)],
    };
    let mut symbols = Symbols::default();
    let invalid = intern_derived(&mut symbols, ByteSlice(&[]), NameText("["));
    for populated in [false, true, false] {
        let mut input = Table {
            kinds: vec![CellKind::Symbol],
            ..Table::default()
        };
        if populated {
            push_row(&mut input, &[Value::Symbol(invalid)]);
        }
        let mut scratch = super::types::OperatorScratch::default();
        let mut memo = Vec::new();
        let mut context = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            stored_indexes: None,
            bindings: &[],
            symbols: &mut symbols,
            source: SourceView(&[]),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        let result = SelectItem.compute(
            &operation,
            &[Rows::Computed(&input)],
            &mut context,
            &mut Table::default(),
        );
        assert_eq!(result.is_err(), populated);
        assert_eq!(initialized(&scratch), usize::from(populated));
        assert!(scratch.join_scratch.0.get().is_none());
        assert!(scratch.walk_scratch.0.get().is_none());
        assert!(scratch.cycle_scratch.0.get().is_none());
    }
}
