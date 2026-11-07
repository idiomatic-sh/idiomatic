//! Real portable use cases with counted calls and one explicit dependency failure.
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::contract::types::BuildOutcome;
use ryo_derived::domain::contract::types::BuildWorkspace;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::contract::types::BuildInputs as DerivedInputs;
use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::request::types::DerivedRequest as ViewRequest;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
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
        match &call {
            OperatorCall::Append { appended, .. } if *appended == [Value::Number(Integer(99))] => {
                self.failed.fetch_add(1, Ordering::Relaxed);
                return Err(PortableOperatorFault::WrongOperation);
            }
            OperatorCall::Append { .. } => {
                self.appended.fetch_add(1, Ordering::Relaxed);
            }
            OperatorCall::Project { .. } => {
                self.projects.fetch_add(1, Ordering::Relaxed);
            }
            OperatorCall::SetRow { .. } => {
                self.unions.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
        OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())).operators(call)
    }
}

#[derive(Clone, Copy, Debug)]
pub(in super::super) enum BuildPolicy {
    Normal,
    RefuseDistinct,
}
#[derive(Debug)]
pub(in super::super) struct Derived {
    policy: BuildPolicy,
    calls: Mutex<Vec<ViewRequest>>,
}
impl Derived {
    pub(in super::super) fn new(policy: BuildPolicy) -> Self {
        Self {
            policy,
            calls: Mutex::new(vec![]),
        }
    }
    pub(in super::super) fn snapshot(&self) -> Vec<ViewRequest> {
        self.calls.lock().unwrap().clone()
    }
    pub(in super::super) fn distinct_calls(&self) -> Calls {
        Calls(
            self.calls
                .lock()
                .unwrap()
                .iter()
                .filter(|request| matches!(request, ViewRequest::DistinctRepresentatives { .. }))
                .count(),
        )
    }
}
impl DerivedPort for Derived {
    fn prepare(&self, request: ViewRequest, schemas: Vec<Vec<CellKind>>) -> PreparedDerived {
        crate::outbound::derived::factory::semantics().prepare(request, schemas)
    }
    fn build(
        &self,
        prepared: &PreparedDerived,
        inputs: DerivedInputs<'_>,
        room: &mut BuildWorkspace,
        recycled: Option<DerivedPayload>,
    ) -> Result<BuildOutcome, DerivedFault> {
        self.calls.lock().unwrap().push(prepared.request().clone());
        if matches!(self.policy, BuildPolicy::RefuseDistinct)
            && matches!(
                prepared.request(),
                ViewRequest::DistinctRepresentatives { .. }
            )
        {
            return Err(DerivedFault::DuplicateOrder);
        }
        crate::outbound::derived::factory::semantics().build(prepared, inputs, room, recycled)
    }
    fn view<'a>(
        &self,
        prepared: &PreparedDerived,
        payload: &'a DerivedPayload,
    ) -> Result<DerivedView<'a>, DerivedFault> {
        crate::outbound::derived::factory::semantics().view(prepared, payload)
    }
}

