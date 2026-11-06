//! The fake of the `syntax` port: what the core's tests stand in for it with.

use super::super::errors::SyntaxError;
use super::super::interface::SyntaxPort;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::descriptions::types::{
    Example, Grammar, Production, ProductionForm, ProductionName,
};
use ryo_values::domain::program::types::{
    ByteOffset, Call, Expression, LetStep, OperationName, ParsedQueryBundle, QueryBundle,
    QueryOrigin, QuerySpan, QueryView, StepName, WrittenEmit, WrittenProgram,
};

/// What the core's tests stand in for `syntax` with: it reads any text that contains `emit` as the
/// program it was given, and refuses the rest.
#[derive(Debug)]
pub struct SyntaxFake {
    /// The program every readable text is read as.
    pub program: WrittenProgram,
}

impl Default for SyntaxFake {
    /// A fake whose program has one step, so it passes the contract.
    fn default() -> Self {
        Self {
            program: WrittenProgram {
                linked: false,
                imports: Vec::new(),
                origins: Vec::new(),
                fragment_contracts: Vec::new(),
                step_origins: std::collections::BTreeMap::new(),
                observations: Vec::new(),
                emit_origin: ryo_values::domain::program::types::EmitOrigin::Unlocated,
                parameters: Vec::new(),
                steps: vec![LetStep {
                    name: StepName(
                        "functions".into(),
                        ryo_values::domain::program::types::StepIdentified::Unassigned,
                    ),
                    expression: Expression::Call(Call {
                        name: OperationName("union".into()),
                        arguments: Vec::new(),
                        span: QuerySpan {
                            start: ByteOffset(0),
                            end: ByteOffset(0),
                        },
                    }),
                    span: QuerySpan {
                        start: ByteOffset(0),
                        end: ByteOffset(0),
                    },
                }],
                emit: WrittenEmit {
                    step: StepName(
                        "".into(),
                        ryo_values::domain::program::types::StepIdentified::Unassigned,
                    ),
                    subject: ColumnName("".into()),
                    key: Vec::new(),
                    span: QuerySpan {
                        start: ByteOffset(0),
                        end: ByteOffset(0),
                    },
                },
            },
        }
    }
}

impl SyntaxPort for SyntaxFake {
    fn read(&self, text: QueryView<'_>) -> Result<WrittenProgram, SyntaxError> {
        if text.0.contains("emit") {
            return Ok(self.program.clone());
        }
        Err(SyntaxError::Unreadable {
            span: QuerySpan {
                start: ByteOffset(0),
                end: ByteOffset(text.0.len()),
            },
            message: "the fake reads only text that contains `emit`".to_owned(),
        })
    }

    fn read_bundle(&self, bundle: &QueryBundle) -> Result<ParsedQueryBundle, SyntaxError> {
        if !bundle.modules.is_empty() {
            return Err(SyntaxError::Located {
                origin: QueryOrigin {
                    source_id: bundle.root.id.clone(),
                    span: QuerySpan {
                        start: ByteOffset(0),
                        end: ByteOffset(0),
                    },
                    trace: Vec::new(),
                },
                message: "the fake syntax port does not parse query modules".to_owned(),
            });
        }
        self.read(QueryView(&bundle.root.text))
            .map_err(|error| match error {
                SyntaxError::Unreadable { span, message } => SyntaxError::Located {
                    origin: QueryOrigin {
                        source_id: bundle.root.id.clone(),
                        span,
                        trace: Vec::new(),
                    },
                    message,
                },
                SyntaxError::Located { .. } => error,
            })
            .map(|root| ParsedQueryBundle {
                root,
                fragments: std::collections::BTreeMap::default(),
            })
    }

    /// One production: the text the fake reads.
    fn grammar(&self) -> Grammar {
        Grammar(vec![Production {
            name: ProductionName("program".into()),
            form: ProductionForm("any text that contains emit".into()),
            example: Example("emit".into()),
        }])
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
