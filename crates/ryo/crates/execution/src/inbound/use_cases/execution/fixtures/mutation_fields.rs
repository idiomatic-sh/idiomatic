//! One mutation of an operation, the collector that makes them field by field, and the values a field is set to: each can
//! reach a fault the operation's schema does not check.
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Direction, JoinColumn, LiteralText, Operand, PlanComparison,
    QueryLiteral, Side, Test, WholeNumber,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// A column past every input's columns.
pub(crate) const BAD: ColumnIndex = ColumnIndex(99);

/// One operation a proof runs: what was changed, the operation and the bindings it reads.
pub(crate) struct Mutation {
    /// The field changed and its value, or the binding swapped.
    pub(crate) label: String,
    /// The operation.
    pub(crate) operation: Operation,
    /// The bindings it reads.
    pub(crate) bindings: Vec<ParameterBinding>,
}

/// A field of an operation, found in a copy of it.
pub(crate) type Field<T> = fn(&mut Operation) -> Option<&mut T>;

/// The mutations being collected from one operation and its bindings.
pub(crate) struct Mutations<'a> {
    /// The operation mutated.
    pub(crate) base: &'a Operation,
    /// The bindings it reads.
    pub(crate) bindings: &'a [ParameterBinding],
    /// The mutations so far.
    pub(crate) out: Vec<Mutation>,
}

impl Mutations<'_> {
    /// The operation with `field` set to each of `values`.
    pub(crate) fn set<T: Clone + std::fmt::Debug>(
        &mut self,
        name: &str,
        field: Field<T>,
        values: &[T],
    ) {
        for value in values {
            let mut operation = self.base.clone();
            if let Some(slot) = field(&mut operation) {
                *slot = value.clone();
                self.out.push(Mutation {
                    label: format!("{name} = {value:?}"),
                    operation,
                    bindings: self.bindings.to_vec(),
                });
            }
        }
    }

    /// The operation with each element of the list `field` set to `value`, and with `value` appended.
    pub(crate) fn elements<T: Clone + std::fmt::Debug>(
        &mut self,
        name: &str,
        field: Field<Vec<T>>,
        values: &[T],
    ) {
        let mut probe = self.base.clone();
        let length = field(&mut probe).map_or(0, |list| list.len());
        for value in values {
            for at in 0..=length {
                let mut operation = self.base.clone();
                if let Some(list) = field(&mut operation) {
                    match list.get_mut(at) {
                        Some(slot) => *slot = value.clone(),
                        None => list.push(value.clone()),
                    }
                    self.out.push(Mutation {
                        label: format!("{name}[{at}] = {value:?}"),
                        operation,
                        bindings: self.bindings.to_vec(),
                    });
                }
            }
        }
    }

    /// The operation with each binding swapped for each other shape.
    pub(crate) fn bindings(&mut self) {
        let shapes = [
            ParameterBinding::Number(WholeNumber(1)),
            ParameterBinding::Text(LiteralText("x".into())),
            ParameterBinding::Numbers(vec![WholeNumber(1)]),
            ParameterBinding::Texts(vec![LiteralText("x".into())]),
            ParameterBinding::TextSets(vec![vec![LiteralText("x".into())]]),
            ParameterBinding::Tables(vec![]),
        ];
        for at in 0..self.bindings.len() {
            for shape in &shapes {
                let mut bindings = self.bindings.to_vec();
                if let Some(slot) = bindings.get_mut(at) {
                    *slot = shape.clone();
                }
                self.out.push(Mutation {
                    label: format!("binding {at} = {shape:?}"),
                    operation: self.base.clone(),
                    bindings,
                });
            }
        }
    }
}

/// Comparisons a filter's schema accepts and its rows may refuse: a refused written pattern, a pattern read from a row, and a
/// column past the input's.
pub(crate) fn comparisons() -> Vec<PlanComparison> {
    let compared = |test, left, right| PlanComparison {
        test,
        left,
        right,
        origin: ComparisonOrigin::Unrecorded,
    };
    vec![
        compared(
            Test::Matches,
            Operand::Column(ColumnIndex(0)),
            Operand::Literal(QueryLiteral::Text(LiteralText("[".into()))),
        ),
        compared(
            Test::Matches,
            Operand::Column(ColumnIndex(0)),
            Operand::Column(ColumnIndex(0)),
        ),
        compared(
            Test::Equal,
            Operand::Column(BAD),
            Operand::Literal(QueryLiteral::Text(LiteralText("x".into()))),
        ),
    ]
}

/// Join columns past either side's columns.
pub(crate) fn outputs() -> Vec<JoinColumn> {
    vec![
        JoinColumn {
            side: Side::Left,
            column: BAD,
        },
        JoinColumn {
            side: Side::Right,
            column: BAD,
        },
    ]
}

/// Every direction a walk can take.
pub(crate) const DIRECTIONS: [Direction; 8] = [
    Direction::Parent,
    Direction::Child,
    Direction::Ancestor,
    Direction::Descendant,
    Direction::NearestAncestor,
    Direction::Deep,
    Direction::PreviousSibling,
    Direction::NextSibling,
];

/// Written patterns a regular expression or a replacement may refuse at its first use.
pub(crate) fn patterns() -> Vec<LiteralText> {
    vec![
        LiteralText("[".into()),
        LiteralText("(".into()),
        LiteralText("$9".into()),
    ]
}
