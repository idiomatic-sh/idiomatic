//! One required semantic invocation contract for ordinary and physical kernels.
//! It has no owner, dependency resolver, publication or cache acquisition API.
use crate::domain::{
    operating::errors::OperatingError,
    operator::contract::types::{PreparedInputs, PrimitiveLease, RowSink},
};
use ryo_operators::domain::contract::types::{OperatorAnswer, OperatorCall, PortableOperatorFault};
use ryo_values::domain::plans::types::{Operation, OperationKind};

pub(crate) enum KernelCall<'call, 'resources, 'invocation> {
    Registered {
        permit: &'call super::program::admission::KernelPermit<'invocation>,
        operation: &'call Operation,
        inputs: PreparedInputs<'call>,
        lease: &'call mut PrimitiveLease<'resources>,
        output: &'call mut dyn RowSink,
    },
    Physical {
        permit: &'call super::program::admission::KernelPermit<'invocation>,
        meaning: OperatorCall<'call>,
    },
}
pub(crate) enum KernelAnswer {
    Complete,
    Physical(OperatorAnswer),
}
pub(crate) enum KernelFault {
    Refused(OperatingError),
    Portable(PortableOperatorFault),
}
pub(crate) trait PrimitiveExecutor: std::fmt::Debug + Sync {
    fn execute(&self, call: KernelCall<'_, '_, '_>) -> Result<KernelAnswer, KernelFault>;
}
