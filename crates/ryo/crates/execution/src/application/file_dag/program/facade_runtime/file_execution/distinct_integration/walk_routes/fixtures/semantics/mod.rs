//! Real portable use cases with counted calls and one explicit dependency failure.
use ryo_execution_store::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::contract::types::BuildOutcome;
use ryo_derived::domain::contract::types::BuildWorkspace;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::contract::types::BuildInputs;
use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) struct Calls(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) struct MeaningSnapshot {
    pub(in super::super) failed: Calls,
    pub(in super::super) appended: Calls,
    pub(in super::super) projects: Calls,
    pub(in super::super) unions: Calls,
}
#[derive(Debug, Default)]
pub(in super::super) struct Meanings {
    failed: AtomicUsize,
    appended: AtomicUsize,
    projects: AtomicUsize,
    unions: AtomicUsize,
}
impl Meanings {
    pub(in super::super) fn snapshot(&self) -> MeaningSnapshot {
        MeaningSnapshot {
            failed: Calls(self.failed.load(Ordering::Relaxed)),
            appended: Calls(self.appended.load(Ordering::Relaxed)),
            projects: Calls(self.projects.load(Ordering::Relaxed)),
            unions: Calls(self.unions.load(Ordering::Relaxed)),
        }
    }
}
impl crate::outbound::operators::interface::OperatorsPort for Meanings {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if let OperatorCall::Evaluate {
            operation, views, ..
        } = &call
        {
            use ryo_values::domain::plans::types::Operation;
            match operation {
                Operation::Extend { .. }
                    if views.appended
                        == ryo_operators::domain::evaluation::types::Lent::Bound(
                            &[Value::Number(Integer(99))][..],
                        ) =>
                {
                    self.failed.fetch_add(1, Ordering::Relaxed);
                    return Err(PortableOperatorFault::WrongOperation);
                }
                Operation::Extend { .. } => {
                    self.appended.fetch_add(1, Ordering::Relaxed);
                }
                Operation::Project { .. } => {
                    self.projects.fetch_add(1, Ordering::Relaxed);
                }
                Operation::Union { .. }
                | Operation::Intersection { .. }
                | Operation::Difference { .. }
                | Operation::Antijoin { .. } => {
                    self.unions.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
        }
        crate::application::file_dag::fixtures::operators().operators(call)
    }
}

#[derive(Debug)]
pub(in super::super) struct Derived {
    calls: Mutex<Vec<DerivedRequest>>,
}
impl Derived {
    pub(in super::super) fn new() -> Self {
        Self {
            calls: Mutex::new(vec![]),
        }
    }
    pub(in super::super) fn snapshot(&self) -> Vec<DerivedRequest> {
        self.calls.lock().unwrap().clone()
    }
}
impl DerivedPort for Derived {
    fn prepare(&self, request: DerivedRequest, schemas: Vec<Vec<CellKind>>) -> PreparedDerived {
        ryo_execution_store::application::file_dag::fixtures::derived().prepare(request, schemas)
    }
    fn build(
        &self,
        prepared: &PreparedDerived,
        inputs: BuildInputs<'_>,
        room: &mut BuildWorkspace,
        recycled: Option<DerivedPayload>,
    ) -> Result<BuildOutcome, DerivedFault> {
        self.calls.lock().unwrap().push(prepared.request().clone());
        ryo_execution_store::application::file_dag::fixtures::derived().build(prepared, inputs, room, recycled)
    }
    fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault> {
        ryo_execution_store::application::file_dag::fixtures::derived().view(prepared, payload)
    }
}

