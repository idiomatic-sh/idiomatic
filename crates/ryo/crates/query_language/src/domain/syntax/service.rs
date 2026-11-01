//! The `syntax` service: what RyoQL's operations look like, and what a program reads. Free
//! functions over the domain's types, with no IO and no ports.

use ryo_language::domain::relations::service::relation_named;
use ryo_language::domain::relations::types::NameRef;
use ryo_language::domain::relations::types::Relation;

use super::constants::operator_named;
use super::types::{ArgumentRole, ArgumentSlot, OperationShape};
use ryo_values::domain::plans::types::QueryOperator;
use ryo_values::domain::program::types::{
    ArgumentName, ArgumentValue, Expression, Joining, OperationName, Program, SourceReference,
};

/// The operations a query can call, besides `select`.
const SHAPES: [OperationShape; 20] = [
    OperationShape {
        operator: QueryOperator::OrderedDfs,
        inputs: &[ArgumentSlot::From, ArgumentSlot::Edges],
        required: &[
            ArgumentSlot::Root,
            ArgumentSlot::Source,
            ArgumentSlot::Destination,
            ArgumentSlot::Order,
            ArgumentSlot::Edge,
            ArgumentSlot::Output,
        ],
    },
    OperationShape {
        operator: QueryOperator::SpanBetween,
        inputs: &[ArgumentSlot::From],
        required: &[ArgumentSlot::Start, ArgumentSlot::End, ArgumentSlot::Output],
    },
    OperationShape {
        operator: QueryOperator::Replace,
        inputs: &[ArgumentSlot::From],
        required: &[
            ArgumentSlot::Text,
            ArgumentSlot::Pattern,
            ArgumentSlot::Replacement,
            ArgumentSlot::Output,
        ],
    },
    OperationShape {
        operator: QueryOperator::OrderedMatch,
        inputs: &[ArgumentSlot::Left, ArgumentSlot::Right],
        required: &[
            ArgumentSlot::LeftOn,
            ArgumentSlot::RightOn,
            ArgumentSlot::LeftOrder,
            ArgumentSlot::RightOrder,
            ArgumentSlot::Output,
        ],
    },
    OperationShape {
        operator: QueryOperator::Project,
        inputs: &[ArgumentSlot::From],
        required: &[ArgumentSlot::Columns],
    },
    OperationShape {
        operator: QueryOperator::Extend,
        inputs: &[ArgumentSlot::From],
        required: &[ArgumentSlot::Values],
    },
    OperationShape {
        operator: QueryOperator::Join,
        inputs: &[ArgumentSlot::Left, ArgumentSlot::Right],
        required: &[
            ArgumentSlot::LeftOn,
            ArgumentSlot::RightOn,
            ArgumentSlot::Output,
        ],
    },
    OperationShape {
        operator: QueryOperator::Union,
        inputs: &[ArgumentSlot::Left, ArgumentSlot::Right],
        required: &[],
    },
    OperationShape {
        operator: QueryOperator::Intersection,
        inputs: &[ArgumentSlot::Left, ArgumentSlot::Right],
        required: &[],
    },
    OperationShape {
        operator: QueryOperator::Difference,
        inputs: &[ArgumentSlot::Left, ArgumentSlot::Right],
        required: &[],
    },
    OperationShape {
        operator: QueryOperator::Antijoin,
        inputs: &[ArgumentSlot::Left, ArgumentSlot::Right],
        required: &[ArgumentSlot::LeftOn, ArgumentSlot::RightOn],
    },
    OperationShape {
        operator: QueryOperator::GroupCount,
        inputs: &[ArgumentSlot::From],
        required: &[ArgumentSlot::Keys, ArgumentSlot::Count],
    },
    OperationShape {
        operator: QueryOperator::GroupDigest,
        inputs: &[ArgumentSlot::From],
        required: &[
            ArgumentSlot::Keys,
            ArgumentSlot::Distinct,
            ArgumentSlot::Digest,
            ArgumentSlot::Count,
        ],
    },
    OperationShape {
        operator: QueryOperator::Capture,
        inputs: &[ArgumentSlot::From],
        required: &[
            ArgumentSlot::Path,
            ArgumentSlot::Pattern,
            ArgumentSlot::Captures,
        ],
    },
    OperationShape {
        operator: QueryOperator::Words,
        inputs: &[ArgumentSlot::From],
        required: &[ArgumentSlot::Text, ArgumentSlot::Word],
    },
    OperationShape {
        operator: QueryOperator::Rank,
        inputs: &[ArgumentSlot::From],
        required: &[
            ArgumentSlot::Column,
            ArgumentSlot::Sets,
            ArgumentSlot::Output,
        ],
    },
    OperationShape {
        operator: QueryOperator::Axis,
        inputs: &[
            ArgumentSlot::From,
            ArgumentSlot::Relation,
            ArgumentSlot::Domain,
        ],
        required: &[ArgumentSlot::Direction],
    },
    OperationShape {
        operator: QueryOperator::Rows,
        inputs: &[],
        required: &[ArgumentSlot::From, ArgumentSlot::Columns],
    },
    OperationShape {
        operator: QueryOperator::Closure,
        inputs: &[ArgumentSlot::Edges],
        required: &[ArgumentSlot::Source, ArgumentSlot::Destination],
    },
    OperationShape {
        operator: QueryOperator::Cycles,
        inputs: &[ArgumentSlot::Edges],
        required: &[ArgumentSlot::Source, ArgumentSlot::Destination],
    },
];

/// The shape of the operation a query calls `name`, when there is one. `select` is not a call.
#[must_use]
pub(crate) fn shape_named(name: &OperationName) -> Option<&'static OperationShape> {
    let operator = operator_named(&name.0)?;
    SHAPES.iter().find(|shape| shape.operator == operator)
}

/// The relations a program reads from, each once, in the order it first reads them.
#[must_use]
pub(crate) fn relations_read(program: &Program) -> Vec<Relation> {
    let mut relations: Vec<Relation> = Vec::new();
    for step in &program.steps {
        let mut found: Vec<Relation> = Vec::new();
        match &step.expression {
            Expression::Select(select) => {
                if let SourceReference::Relation(text) = &select.from.reference {
                    found.extend(relation_named(NameRef(&text.0)));
                }
                if let Joining::Joined(join) = &select.join
                    && let SourceReference::Relation(text) = &join.source.reference
                {
                    found.extend(relation_named(NameRef(&text.0)));
                }
            }
            Expression::Call(call) => {
                let inputs = shape_named(&call.name).map_or(&[][..], |shape| shape.inputs);
                for argument in &call.arguments {
                    if let (ArgumentRole::Input, ArgumentValue::Text(text)) =
                        (is_input(inputs, &argument.name), &argument.value)
                    {
                        found.extend(relation_named(NameRef(&text.0)));
                    }
                }
            }
        }
        for relation in found {
            if !relations.contains(&relation) {
                relations.push(relation);
            }
        }
    }
    relations
}

/// Whether `name` is one of the arguments in `inputs`.
#[must_use]
pub(crate) fn is_input(inputs: &[ArgumentSlot], name: &ArgumentName) -> ArgumentRole {
    if inputs
        .iter()
        .any(|slot| super::constants::slot_name(*slot) == &*name.0)
    {
        ArgumentRole::Input
    } else {
        ArgumentRole::Other
    }
}

/// The operators a program uses, each once, in the order it first uses them.
#[must_use]
pub(crate) fn operators_used(program: &Program) -> Vec<QueryOperator> {
    let mut operators: Vec<QueryOperator> = Vec::new();
    for step in &program.steps {
        let operator = match &step.expression {
            Expression::Select(select) if matches!(select.join, Joining::Joined(_)) => {
                Some(QueryOperator::Join)
            }
            Expression::Select(_) => Some(QueryOperator::Select),
            Expression::Call(call) => operator_named(&call.name.0),
        };
        if let Some(operator) = operator
            && !operators.contains(&operator)
        {
            operators.push(operator);
        }
    }
    operators
}
