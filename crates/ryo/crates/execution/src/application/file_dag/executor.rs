//! One required semantic invocation contract for ordinary and physical kernels.
//! It has no owner, dependency resolver, publication or cache acquisition API.
use crate::domain::operating::errors::OperatingError;
use ryo_operators::inbound::use_cases::operators::interface::{
    OperatorAnswer, PortableOperatorFault,
};

pub(crate) enum KernelCall<'call, 'resources, 'invocation> {
    Registered {
        request: super::execution::lease::ExecutableLease<'call, 'resources, 'invocation>,
    },
    Walk {
        request: super::program::native_walk::PermittedWalk<'call, 'resources, 'invocation>,
    },
    NativeJoin {
        request: super::program::native_join::PermittedNativeJoin<'call, 'resources, 'invocation>,
    },
    Physical {
        request: crate::application::file_dag::program::runtime::types::PermittedPhysical<
            'call,
            'resources,
            'invocation,
        >,
    },
}
pub(crate) enum KernelAnswer {
    Complete,
    Physical(OperatorAnswer),
    Walk(crate::domain::shared_walk::types::prepared_walk::value::NativeBatch),
}
pub(crate) enum KernelFault {
    Refused(OperatingError),
    Portable(PortableOperatorFault),
}
pub(crate) trait PrimitiveExecutor: std::fmt::Debug + Sync {
    fn execute(&self, call: KernelCall<'_, '_, '_>) -> Result<KernelAnswer, KernelFault>;
}
