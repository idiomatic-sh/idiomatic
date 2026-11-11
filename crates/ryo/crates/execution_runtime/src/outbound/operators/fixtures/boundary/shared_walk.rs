//! The operators fakes the execution use case's `shared_walk` boundary scenarios inject.
use crate::application::file_dag::program::file_session::types::FileInput;
use crate::application::file_dag::program::file_session::types::SourceLanguage;
use crate::application::file_dag::program::file_session::types::SourceRows;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::application::file_dag::program::rule_outcome::types::RuleOutcome;
use crate::outbound::facts::types::ResidentStability;
use crate::outbound::operators::interface::OperatorsPort as _;
use ryo_language::domain::builders::fixtures::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::{
    completion::fixtures::{completion, name_kind},
    elements::{
        fixtures::{kind_names, real_count, text_at},
        types::Elements,
    },
    kind_index::fixtures::index_kinds,
    symbols::types::{ByteSlice, Symbols},
};
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
use ryo_relations::domain::{
    relations::{
        fixtures::{kinds, node_id},
        types::{FileId, KindBits, Offset, Position, Relation, Span, Value},
    },
    terms::types::NameText,
};
use ryo_values::domain::{
    plan_parts::types::{
        ColumnIndex, Input, LiteralText, Operand, ParameterId, ParameterName, PlanComparison,
        QueryLiteral, Test,
    },
    plans::types::Operation,
    query_bindings::types::{ParameterBinding, ParameterType, PlanParameter},
    rows::types::{RowCount, Table},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, Default)]
pub struct SemanticCalls(pub AtomicUsize);

impl crate::outbound::operators::interface::OperatorsPort for SemanticCalls {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if matches!(
            &call,
            OperatorCall::Evaluate {
                operation: ryo_values::domain::plans::types::Operation::Select { .. },
                ..
            }
        ) {
            let _previous = self.0.fetch_add(1, Ordering::Relaxed);
        }
        OperatorsUseCaseImpl::new(std::sync::Arc::new(
            ryo_operators::outbound::matching::factory::use_case(),
        ))
        .operators(call)
    }
}

impl SemanticCalls {
    pub fn selects(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}
