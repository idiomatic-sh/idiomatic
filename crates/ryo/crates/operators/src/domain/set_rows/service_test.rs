//! Set semantics preserve membership order, duplicates and lazy antijoin keys.
use super::service::{positions, values};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::set_rows::types::{SetInterrupted, SetKind, SetPositionsDriver, SetRowDriver};
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy)]
enum Scenario {
    UnionNew,
    UnionRepeated,
    IntersectionAbsent,
    DifferencePresent,
    AntijoinEmptyInvalidKey,
    AntijoinPresent,
    AntijoinAbsent,
    AntijoinInvalidKey,
    PositionUnionDuplicate,
    PositionAntijoinEmpty,
    PositionAntijoinMixed,
    PositionInterrupted,
}
type Input = Scenario;
type Value = [usize; 3];
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("union_emits_first_row"),
        word: Word::Succeeds,
        input: || Scenario::UnionNew,
        expect: Expect::Value([0, 1, 1]),
    },
    Case {
        name: CaseName("union_suppresses_repeated_row"),
        word: Word::Succeeds,
        input: || Scenario::UnionRepeated,
        expect: Expect::Value([0, 1, 0]),
    },
    Case {
        name: CaseName("intersection_skips_absent_before_distinct"),
        word: Word::Succeeds,
        input: || Scenario::IntersectionAbsent,
        expect: Expect::Value([1, 0, 0]),
    },
    Case {
        name: CaseName("difference_skips_present_before_distinct"),
        word: Word::Succeeds,
        input: || Scenario::DifferencePresent,
        expect: Expect::Value([1, 0, 0]),
    },
    Case {
        name: CaseName("empty_right_antijoin_never_reads_invalid_left_key"),
        word: Word::Succeeds,
        input: || Scenario::AntijoinEmptyInvalidKey,
        expect: Expect::Value([0, 0, 1]),
    },
    Case {
        name: CaseName("antijoin_retains_duplicate_when_rhs_absent"),
        word: Word::Succeeds,
        input: || Scenario::AntijoinAbsent,
        expect: Expect::Value([1, 0, 1]),
    },
    Case {
        name: CaseName("antijoin_skips_present_key"),
        word: Word::Succeeds,
        input: || Scenario::AntijoinPresent,
        expect: Expect::Value([1, 0, 0]),
    },
    Case {
        name: CaseName("nonempty_right_antijoin_checks_first_missing_key"),
        word: Word::Refuses,
        input: || Scenario::AntijoinInvalidKey,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: 2 }),
    },
    Case {
        name: CaseName("deferred_union_keeps_only_first_position_row"),
        word: Word::Succeeds,
        input: || Scenario::PositionUnionDuplicate,
        expect: Expect::Value([0, 2, 1]),
    },
    Case {
        name: CaseName("deferred_antijoin_skips_keys_for_empty_right"),
        word: Word::Succeeds,
        input: || Scenario::PositionAntijoinEmpty,
        expect: Expect::Value([0, 0, 2]),
    },
    Case {
        name: CaseName("deferred_antijoin_keeps_unmatched_in_order"),
        word: Word::Succeeds,
        input: || Scenario::PositionAntijoinMixed,
        expect: Expect::Value([2, 0, 1]),
    },
    Case {
        name: CaseName("deferred_delivery_stops_at_first_interruption"),
        word: Word::Refuses,
        input: || Scenario::PositionInterrupted,
        expect: Expect::Error(PortableOperatorFault::Interrupted),
    },
];

struct PositionProbe {
    next: usize,
    current: usize,
    right_empty: bool,
    fail_emit: bool,
    calls: Value,
}
impl SetPositionsDriver for PositionProbe {
    fn next(&mut self) -> Result<bool, SetInterrupted> {
        if self.next == 2 {
            return Ok(false);
        }
        self.current = self.next;
        self.next += 1;
        Ok(true)
    }
    fn right_empty(&mut self) -> Result<bool, SetInterrupted> {
        Ok(self.right_empty)
    }
    fn contains_right(&mut self) -> Result<bool, SetInterrupted> {
        self.calls[0] += 1;
        Ok(self.current == 0)
    }
    fn first_time(&mut self) -> Result<bool, SetInterrupted> {
        self.calls[1] += 1;
        Ok(self.current == 0)
    }
    fn emit(&mut self) -> Result<(), SetInterrupted> {
        self.calls[2] += 1;
        if self.fail_emit {
            return Err(SetInterrupted);
        }
        Ok(())
    }
}

struct Probe {
    right_empty: bool,
    contains: bool,
    first: bool,
    calls: Value,
}
impl SetRowDriver for Probe {
    fn right_empty(&mut self) -> Result<bool, SetInterrupted> {
        Ok(self.right_empty)
    }
    fn contains_right(&mut self, key: &[Cell]) -> Result<bool, SetInterrupted> {
        assert_eq!(key, &[Cell::Number(Integer(7))]);
        self.calls[0] += 1;
        Ok(self.contains)
    }
    fn first_time(&mut self, row: &[Cell]) -> Result<bool, SetInterrupted> {
        assert_eq!(row, &[Cell::Number(Integer(7))]);
        self.calls[1] += 1;
        Ok(self.first)
    }
    fn emit(&mut self, row: &[Cell]) -> Result<(), SetInterrupted> {
        assert_eq!(row, &[Cell::Number(Integer(7))]);
        self.calls[2] += 1;
        Ok(())
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    if matches!(
        input,
        Scenario::PositionUnionDuplicate
            | Scenario::PositionAntijoinEmpty
            | Scenario::PositionAntijoinMixed
            | Scenario::PositionInterrupted
    ) {
        let mut probe = PositionProbe {
            next: 0,
            current: 0,
            right_empty: matches!(input, Scenario::PositionAntijoinEmpty),
            fail_emit: matches!(input, Scenario::PositionInterrupted),
            calls: [0; 3],
        };
        let kind = if matches!(
            input,
            Scenario::PositionUnionDuplicate | Scenario::PositionInterrupted
        ) {
            SetKind::Union
        } else {
            SetKind::Antijoin
        };
        positions(kind, &mut probe)?;
        return Ok(probe.calls);
    }
    let mut probe = Probe {
        right_empty: matches!(input, Scenario::AntijoinEmptyInvalidKey),
        contains: matches!(
            input,
            Scenario::DifferencePresent | Scenario::AntijoinPresent
        ),
        first: !matches!(input, Scenario::UnionRepeated),
        calls: [0; 3],
    };
    let (kind, columns): (SetKind, &[ColumnIndex]) = match input {
        Scenario::UnionNew | Scenario::UnionRepeated => (SetKind::Union, &[]),
        Scenario::IntersectionAbsent => (SetKind::Intersection, &[]),
        Scenario::DifferencePresent => (SetKind::Difference, &[]),
        Scenario::AntijoinEmptyInvalidKey | Scenario::AntijoinInvalidKey => {
            (SetKind::Antijoin, &[ColumnIndex(2)])
        }
        Scenario::AntijoinPresent | Scenario::AntijoinAbsent => {
            (SetKind::Antijoin, &[ColumnIndex(0)])
        }
        Scenario::PositionUnionDuplicate
        | Scenario::PositionAntijoinEmpty
        | Scenario::PositionAntijoinMixed
        | Scenario::PositionInterrupted => unreachable!("handled above"),
    };
    let mut key = Vec::new();
    values(
        kind,
        &[Cell::Number(Integer(7))],
        columns,
        &mut probe,
        &mut key,
    )?;
    Ok(probe.calls)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
