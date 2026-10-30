//! The fake of the `syntax` port: what the core's tests stand in for it with.

use super::super::errors::SyntaxError;
use super::super::interface::SyntaxPort;
use ryo_values::domain::descriptions::types::{
    Example, Grammar, Production, ProductionForm, ProductionName,
};
use ryo_values::domain::program::types::{
    ByteOffset, Call, ColumnName, Expression, LetStep, OperationName, Program, QuerySpan,
    QueryView, StepName, WrittenEmit,
};

/// What the core's tests stand in for `syntax` with: it reads any text that contains `emit` as the
/// program it was given, and refuses the rest.
#[derive(Debug)]
pub struct SyntaxFake {
    /// The program every readable text is read as.
    pub program: Program,
}

impl Default for SyntaxFake {
    /// A fake whose program has one step, so it passes the contract.
    fn default() -> Self {
        Self {
            program: Program {
                parameters: Vec::new(),
                steps: vec![LetStep {
                    name: StepName("functions".into()),
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
                    step: StepName("".into()),
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
    fn read(&self, text: QueryView<'_>) -> Result<Program, SyntaxError> {
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
