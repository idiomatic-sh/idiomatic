//! The `rank` item's cases.

use ryo_language::domain::relations::types::{CellKind, Integer, Offset, Relation, Value as Cell};
use ryo_language::domain::symbols::service::intern;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};

use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::rank::item::RankItem;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{ParameterBinding, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput, ParameterId};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::QueryOperator;

type Value = &'static [(&'static str, i64)];
type Failure = OperatingError;
/// A scenario: the names ranked and the sets that rank them, answering each name kept with its place.
type Input = (&'static [&'static str], Option<ParameterBinding>);

const FROM: PlanInput = PlanInput::Relation(Relation::Comment);

fn sets(layers: &[&[&str]]) -> ParameterBinding {
    ParameterBinding::TextSets(
        layers
            .iter()
            .map(|set| set.iter().map(|each| LiteralText((*each).into())).collect())
            .collect(),
    )
}

/// The names the text holds, each followed by the place of its set, as the item says them.
fn subject((names, binding): Input) -> Result<Value, Failure> {
    let text: String = names.concat();
    let bytes = text.as_bytes();
    let mut symbols = Symbols::default();
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    let mut start = 0;
    for name in names {
        let end = start + u32::try_from(name.len()).unwrap_or(u32::MAX);
        push_row(
            &mut table,
            &[Cell::Symbol(intern(
                &mut symbols,
                ByteSlice(bytes),
                Offset(start),
                Offset(end),
            ))],
        );
        start = end;
    }
    let operation = Operation::Rank {
        from: FROM,
        column: ColumnIndex(0),
        sets: ParameterId(0),
    };
    let bound: Vec<ParameterBinding> = binding.into_iter().collect();
    let mut out = Table::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &bound,
        symbols: (&mut symbols).into(),
        source: SourceView(bytes),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    RankItem.compute(
        &operation,
        &[Rows::Computed(&table)],
        &mut context,
        &mut out,
    )?;
    let mut found = Vec::new();
    for row in out.cells.chunks(2) {
        if let &[Cell::Symbol(symbol), Cell::Number(Integer(place))] = row {
            let name = names
                .iter()
                .find(|each| {
                    ryo_language::domain::symbols::service::text_of(
                        &symbols,
                        ByteSlice(bytes),
                        symbol,
                    )
                    .is_some_and(|name_text| name_text.0 == **each)
                })
                .copied()
                .unwrap_or_default();
            found.push((name, place));
        }
    }
    Ok(Box::leak(found.into_boxed_slice()))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_text_takes_the_place_of_the_first_set_that_holds_it"),
        word: Word::Succeeds,
        input: || {
            (
                &["cli", "core", "sdk"],
                Some(sets(&[&["cli"], &["sdk", "core"], &["core"]])),
            )
        },
        expect: Expect::Value(&[("cli", 0), ("core", 1), ("sdk", 1)]),
    },
    Case {
        name: CaseName("a_text_no_set_holds_is_dropped"),
        word: Word::Succeeds,
        input: || (&["cli", "leaf"], Some(sets(&[&["cli"], &["core"]]))),
        expect: Expect::Value(&[("cli", 0)]),
    },
    Case {
        name: CaseName("no_sets_drop_every_row"),
        word: Word::Succeeds,
        input: || (&["cli"], Some(sets(&[]))),
        expect: Expect::Value(&[]),
    },
    Case {
        name: CaseName("a_parameter_nothing_bound_is_refused"),
        word: Word::Refuses,
        input: || (&["cli"], None),
        expect: Expect::Error(OperatingError::Unbound),
    },
    Case {
        name: CaseName("a_parameter_bound_to_a_text_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                &["cli"],
                Some(ParameterBinding::Text(LiteralText("cli".into()))),
            )
        },
        expect: Expect::Error(OperatingError::WrongBinding),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    crate::domain::operator::collections::operators::fixtures::contract::check(QueryOperator::Rank);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
