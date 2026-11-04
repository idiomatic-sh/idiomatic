//! One required semantic invocation contract for ordinary and physical kernels.
//! It has no owner, dependency resolver, publication or cache acquisition API.
use crate::domain::{
    operating::errors::OperatingError,
    operator::contract::types::{PreparedInputs, PrimitiveLease, RowSink},
};
use ryo_operators::domain::contract::types::{OperatorAnswer, PortableOperatorFault};
use ryo_values::domain::plans::types::Operation;

pub(crate) enum KernelCall<'call, 'resources, 'invocation> {
    Registered {
        permit: &'call super::program::admission::KernelPermit<'invocation>,
        operation: &'call Operation,
        inputs: PreparedInputs<'call>,
        lease: &'call mut PrimitiveLease<'resources>,
        output: &'call mut dyn RowSink,
    },
    Walk {
        request: super::program::native_walk::PermittedWalk<'call, 'resources, 'invocation>,
    },
    Physical {
        request: super::program::admission::PermittedPhysical<'call, 'resources, 'invocation>,
    },
}
pub(crate) enum KernelAnswer {
    Complete,
    Physical(OperatorAnswer),
    Walk(crate::domain::shared_walk::types::NativeBatch),
}
pub(crate) enum KernelFault {
    Refused(OperatingError),
    Portable(PortableOperatorFault),
}
pub(crate) trait PrimitiveExecutor: std::fmt::Debug + Sync {
    fn execute(&self, call: KernelCall<'_, '_, '_>) -> Result<KernelAnswer, KernelFault>;
}
