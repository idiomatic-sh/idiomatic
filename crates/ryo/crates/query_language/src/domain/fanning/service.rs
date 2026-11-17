//! The `fanning` service: what each join's keys are proven to pair, from the distinctness of its sides and the kind of its keys,
//! stamped on the plan once its steps are placed.

use super::types::JoinKeys;
use crate::domain::distinctness::service::{distinct_on, keyed_on_file_alone};
use crate::domain::distinctness::types::{DistinctMemo, FileKeyed};
use crate::domain::lineage::types::LineageMemo;
use crate::domain::lowering::types::Uniqueness;
use ryo_values::domain::band_joins::types::BandJoin;
use ryo_values::domain::plan_parts::service::view_source;
use ryo_values::domain::plan_parts::types::{ColumnIndex, KeyFan};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a join's keys are proven to pair: as written for an `all-pairs join`; `Unique` when either side is distinct on its keys
/// (the right through its view); `FileOnly` when the left keys are the file alone; `Shared` otherwise.
pub(crate) fn key_fan(
    steps: &[PlanStep],
    distinct: &mut DistinctMemo,
    kinds: &mut LineageMemo,
    join: JoinKeys<'_>,
) -> KeyFan {
    let JoinKeys {
        left,
        left_on,
        right,
        right_on,
        view,
        written,
    } = join;
    if written == KeyFan::AllPairs {
        return KeyFan::AllPairs;
    }
    let right_keys: Option<Vec<ColumnIndex>> =
        right_on.iter().map(|key| view_source(view, *key)).collect();
    if right_keys.is_some_and(|keys| {
        !keys.is_empty() && distinct_on(steps, distinct, right, &keys) == Uniqueness::Unique
    }) || (!left_on.is_empty()
        && distinct_on(steps, distinct, left, left_on) == Uniqueness::Unique)
    {
        KeyFan::Unique
    } else if keyed_on_file_alone(steps, kinds, left, left_on) == FileKeyed::Alone {
        KeyFan::FileOnly
    } else {
        KeyFan::Shared
    }
}

/// The plan with each join's fan proven by `key_fan`: run once the plan's steps are placed, before it crosses to the operators.
#[must_use]
pub(crate) fn fanned(mut steps: Vec<PlanStep>) -> Vec<PlanStep> {
    let mut distinct = DistinctMemo::default();
    let mut kinds = LineageMemo::default();
    let proven: Vec<Option<KeyFan>> = steps
        .iter()
        .map(|step| match &step.operation {
            Operation::Join {
                left,
                right,
                left_on,
                right_on,
                view,
                fan,
                reach: None,
                ..
            }
            | Operation::BandJoin(BandJoin {
                left,
                right,
                left_on,
                right_on,
                view,
                fan,
                ..
            }) => Some(key_fan(
                &steps,
                &mut distinct,
                &mut kinds,
                JoinKeys {
                    left: *left,
                    left_on,
                    right: *right,
                    right_on,
                    view,
                    written: *fan,
                },
            )),
            _ => None,
        })
        .collect();
    for (step, found) in steps.iter_mut().zip(proven) {
        if let (
            Operation::Join { fan, .. } | Operation::BandJoin(BandJoin { fan, .. }),
            Some(found),
        ) = (&mut step.operation, found)
        {
            *fan = found;
        }
    }
    steps
}
