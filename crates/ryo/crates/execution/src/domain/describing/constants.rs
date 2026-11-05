//! The names of the `describing` concept: the description of an operator and of its fields.

use ryo_values::domain::descriptions::types::{
    Example, FieldDescription, FieldName, FieldRequirement, OperatorDescription, OperatorName,
    OptionLine, Purpose,
};

/// A field of an operator's description: its name, what it holds and whether it must be given.
pub(crate) fn field(name: &str, summary: &str, required: FieldRequirement) -> FieldDescription {
    FieldDescription {
        name: FieldName(name.into()),
        summary: Purpose(summary.into()),
        required,
    }
}

/// An operator's description: its name, what it does, its fields, its options and an example of it.
pub(crate) fn described(
    name: &str,
    summary: &str,
    fields: Vec<FieldDescription>,
    options: &[&str],
    example: &str,
) -> OperatorDescription {
    OperatorDescription {
        name: OperatorName(name.into()),
        summary: Purpose(summary.into()),
        fields,
        options: options
            .iter()
            .map(|option| OptionLine((*option).into()))
            .collect(),
        example: Example(example.into()),
    }
}
