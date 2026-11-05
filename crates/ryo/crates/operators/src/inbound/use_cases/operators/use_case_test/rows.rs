//! Calls the real public row contracts and observes caller-owned scratch, not private algorithms.
use super::*;
pub(super) fn subject(input: Scenario) -> Result<Observation, Failure> {
    let mut target = Vec::with_capacity(8);
    let original = target.as_ptr();
    match input {
        Scenario::Project | Scenario::Append | Scenario::MissingProjectColumn => {
            let call = match input {
                Scenario::Project => OperatorCall::Project {
                    row: &ROW,
                    columns: &[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
                    target: &mut target,
                },
                Scenario::Append => OperatorCall::Append {
                    row: &ROW,
                    appended: &[ROW[0]],
                    target: &mut target,
                },
                Scenario::MissingProjectColumn => OperatorCall::Project {
                    row: &ROW,
                    columns: &[ColumnIndex(2)],
                    target: &mut target,
                },
                _ => unreachable!(),
            };
            written(call)?;
        }
        Scenario::SelectRepeated
        | Scenario::SelectRejected
        | Scenario::SelectOriginal
        | Scenario::SelectMissing => {
            target.push(ROW[0]);
            let kept: &[ColumnIndex] = match input {
                Scenario::SelectRepeated => &[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
                Scenario::SelectOriginal => &[],
                _ => &[ColumnIndex(9), ColumnIndex(8)],
            };
            let answer = OperatorsUseCaseImpl::new().operators(OperatorCall::Select {
                row: &ROW,
                kept,
                holds: !matches!(input, Scenario::SelectRejected),
                target: &mut target,
            })?;
            let OperatorAnswer::Selected(accepted) = answer else {
                panic!("Select must return its acceptance outcome")
            };
            assert_eq!(
                target.as_ptr(),
                original,
                "selection borrows caller-owned scratch"
            );
            return Ok(Observation::Selected(accepted, Cow::Owned(target)));
        }
        Scenario::CountOutput | Scenario::DigestOutput | Scenario::AggregateMissing => {
            let (keys, count, digest): (&[ColumnIndex], _, _) = match input {
                Scenario::CountOutput => {
                    (&[ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)], 4, None)
                }
                Scenario::DigestOutput => (&[], 3, Some(-5)),
                _ => (&[ColumnIndex(4), ColumnIndex(3)], 0, None),
            };
            written(OperatorCall::Aggregate {
                row: &ROW,
                keys,
                count,
                digest,
                target: &mut target,
            })?;
        }
        _ => unreachable!("row fixture receives only row scenarios"),
    }
    assert_eq!(
        target.as_ptr(),
        original,
        "borrowed output keeps its allocation"
    );
    Ok(Observation::Cells(Cow::Owned(target)))
}
