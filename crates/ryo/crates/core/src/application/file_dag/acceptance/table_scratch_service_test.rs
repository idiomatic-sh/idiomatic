//! The `table_scratch` service's cases, on values alone.

use ryo_relations::domain::relations::types::KindBits;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_model::types::SetMembership;
use crate::domain::table_scratch::service::kind_has;
use crate::domain::table_scratch::types::{KindMask, KindSet};

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
fn initialized(scratch: &crate::domain::table_scratch::types::OperatorScratch) -> usize {
    [
        scratch.select_scratch.0.get().is_some(),
        scratch.join_scratch.0.get().is_some(),
        scratch.walk_scratch.0.get().is_some(),
    ]
    .into_iter()
    .filter(|made| *made)
    .count()
}

/// Pure projection needs only cheap rows; distinct projection constructs just its membership group.
#[test]
fn projection_constructs_only_the_group_it_uses_and_keeps_bags() {
    use crate::application::file_dag::testing::TestOperator as _;
    use crate::domain::operating::types::OperatorContext;
    use crate::domain::operator::collections::operators::project::item::ProjectItem;
    use crate::domain::table_model::service::push_row;
    use crate::domain::table_model::types::{Rows, Table};
    use crate::domain::table_texts::types::SourceView;
    use ryo_relations::domain::relations::types::{CellKind, Integer, Relation, Value};
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
    let mut scratch = crate::domain::table_scratch::types::OperatorScratch::default();
    assert_eq!(initialized(&scratch), 0);
    let mut symbols = Symbols::default();
    let mut memo = Vec::new();
    for (distinct, expected, groups) in [
        (RepeatedRows::Kept, vec![2, 1, 2], 0),
        (RepeatedRows::Dropped, vec![2, 1], 1),
    ] {
        let mut out = Table::default();
        let mut context = OperatorContext {
            semantic: None,
            prepared_kinds: None,
            prepared_matchers: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
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
        assert_eq!(
            initialized(&scratch),
            0,
            "projection uses engine-owned workspace, not legacy operator clusters"
        );
        assert_eq!(scratch.observation.owners_initialized, groups);
        assert!(scratch.observation.local_resources_released);
    }
    // A new rule owns a fresh workspace; no idle group or large payload crosses the boundary.
    assert_eq!(
        initialized(&crate::domain::table_scratch::types::OperatorScratch::default()),
        0
    );
}

/// Dynamic pattern errors remain row-lazy; failed matchers are never cached.
#[test]
fn dynamic_regex_initialization_is_execution_local_and_lazy() {
    use crate::application::file_dag::testing::TestOperator as _;
    use crate::domain::operating::types::OperatorContext;
    use crate::domain::operator::collections::operators::select::item::SelectItem;
    use crate::domain::table_model::service::push_row;
    use crate::domain::table_model::types::{Rows, Table};
    use crate::domain::table_texts::types::SourceView;
    use ryo_relations::domain::relations::types::{CellKind, Relation, Value};
    use ryo_language::domain::symbols::service::intern_derived;
    use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
    use ryo_relations::domain::terms::types::NameText;
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
        let mut scratch = crate::domain::table_scratch::types::OperatorScratch::default();
        let mut memo = Vec::new();
        let mut context = OperatorContext {
            semantic: None,
            prepared_kinds: None,
            prepared_matchers: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
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
        assert_eq!(
            scratch.observation.regexes, 0,
            "invalid patterns cannot enter the cache"
        );
        assert!(scratch.join_scratch.0.get().is_none());
        assert!(scratch.walk_scratch.0.get().is_none());
        assert!(scratch.observation.local_resources_released);
    }
}

/// Aggregations own one real invocation resource and release it after producing
/// independent nonempty results, without recreating operator-local scratch rooms.
#[test]
fn aggregation_resources_are_engine_owned_and_released() {
    use crate::application::file_dag::acceptance::operator_contract_test::fixtures::{
        SOURCE, assert_expected, fixture,
    };
    use crate::application::file_dag::testing::TestOperator as _;
    use crate::domain::operating::types::OperatorContext;
    use crate::domain::operator::service::registered;
    use crate::domain::table_model::types::{Rows, Table};
    use crate::domain::table_texts::types::SourceView;
    use ryo_values::domain::plans::types::OperationKind;
    for kind in [OperationKind::GroupCount, OperationKind::GroupDigest] {
        let mut case = fixture(kind);
        let inputs = case.inputs.iter().map(Rows::Computed).collect::<Vec<_>>();
        let mut scratch = crate::domain::table_scratch::types::OperatorScratch::default();
        let mut out = Table::default();
        let mut context = OperatorContext {
            semantic: None,
            prepared_kinds: None,
            prepared_matchers: None,
            bindings: &case.bindings,
            symbols: (&mut case.symbols).into(),
            source: SourceView(SOURCE),
            paths: &[],
            arenas: &[],
            memo: &mut Vec::new(),
            scratch: &mut scratch,
        };
        registered(kind)
            .compute(&case.operation, &inputs, &mut context, &mut out)
            .unwrap();
        let rows = out
            .cells
            .chunks_exact(out.kinds.len())
            .map(<[_]>::to_vec)
            .collect::<Vec<_>>();
        assert_expected(&case.expected, &rows, &case.symbols);
        assert_eq!(initialized(&scratch), 0);
        assert_eq!(scratch.observation.owners_initialized, 1, "{kind:?}");
        assert!(scratch.observation.local_resources_released, "{kind:?}");
    }
}
