//! Read-only binding metadata: scalar output kinds and validated table/configuration shapes.

use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::service::check_columns;
use crate::domain::table_model::types::ParameterBinding;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::service::parameter_in;
use ryo_values::domain::plan_parts::types::{
    Comparison, Operand, ParameterId, QueryLiteral, TableKey,
};

/// Immutable binding metadata; it cannot mint symbols or manufacture execution rows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BindingSchema<'a> {
    /// Configuration values borrowed for schema validation.
    bindings: &'a [ParameterBinding],
}

impl<'a> BindingSchema<'a> {
    /// Borrow configuration for one schema derivation.
    pub(crate) const fn new(bindings: &'a [ParameterBinding]) -> Self {
        Self { bindings }
    }
    /// Append the scalar kinds an extend writes, without making its text names.
    pub(crate) fn extended(
        &self,
        values: &[Operand],
        mut kinds: Vec<CellKind>,
    ) -> Result<Vec<CellKind>, OperatingError> {
        for value in values {
            kinds.push(match value {
                Operand::Literal(QueryLiteral::Number(_)) => CellKind::Number,
                Operand::Literal(QueryLiteral::Text(_)) => CellKind::Symbol,
                Operand::Parameter(id) => {
                    match parameter_in(self.bindings, *id).ok_or(OperatingError::Unbound)? {
                        ParameterBinding::Number(_) => CellKind::Number,
                        ParameterBinding::Flag(_) => CellKind::Flag,
                        ParameterBinding::Text(_) => CellKind::Symbol,
                        ParameterBinding::Numbers(_)
                        | ParameterBinding::Texts(_)
                        | ParameterBinding::TextSets(_)
                        | ParameterBinding::Tables(_) => return Err(OperatingError::WrongBinding),
                    }
                }
                Operand::Column(_) | Operand::Function(_) => {
                    return Err(OperatingError::WrongBinding);
                }
            });
        }
        Ok(kinds)
    }

    /// Validate table bindings and every requested key, without manufacturing binding rows.
    pub(crate) fn rows(
        &self,
        parameter: ParameterId,
        keys: &[TableKey],
    ) -> Result<Vec<CellKind>, OperatingError> {
        match parameter_in(self.bindings, parameter).ok_or(OperatingError::Unbound)? {
            ParameterBinding::Tables(tables) => {
                for table in tables {
                    for key in keys {
                        if !table.0.iter().any(|cell| *cell.key.0 == *key.0) {
                            return Err(OperatingError::WrongBinding);
                        }
                    }
                }
            }
            ParameterBinding::Numbers(items) if items.is_empty() => {}
            ParameterBinding::Texts(items) if items.is_empty() => {}
            ParameterBinding::TextSets(items) if items.is_empty() => {}
            ParameterBinding::Number(_)
            | ParameterBinding::Flag(_)
            | ParameterBinding::Text(_)
            | ParameterBinding::Numbers(_)
            | ParameterBinding::Texts(_)
            | ParameterBinding::TextSets(_) => return Err(OperatingError::WrongBinding),
        }
        Ok(vec![CellKind::Symbol; keys.len()])
    }

    /// Schema validation of operands does not resolve a column or intern a parameter value.
    fn operand_columns(&self, operand: &Operand, kinds: &[CellKind]) -> Result<(), OperatingError> {
        match operand {
            Operand::Column(column) => check_columns(kinds, &[*column])?,
            Operand::Literal(_) => {}
            Operand::Parameter(id) => {
                match parameter_in(self.bindings, *id).ok_or(OperatingError::Unbound)? {
                    ParameterBinding::Number(_)
                    | ParameterBinding::Flag(_)
                    | ParameterBinding::Text(_)
                    | ParameterBinding::Numbers(_)
                    | ParameterBinding::Texts(_) => {}
                    ParameterBinding::TextSets(_) | ParameterBinding::Tables(_) => {
                        return Err(OperatingError::WrongBinding);
                    }
                }
            }
            Operand::Function(applied) => {
                for argument in &applied.arguments {
                    self.operand_columns(argument, kinds)?;
                }
            }
        }
        Ok(())
    }

    /// Validate fixed operand failures, leaving row-dependent matching to execution.
    pub(crate) fn predicates(
        &self,
        comparisons: &[Comparison],
        kinds: &[CellKind],
    ) -> Result<(), OperatingError> {
        for comparison in comparisons {
            self.operand_columns(&comparison.left, kinds)?;
            self.operand_columns(&comparison.right, kinds)?;
        }
        Ok(())
    }
}
