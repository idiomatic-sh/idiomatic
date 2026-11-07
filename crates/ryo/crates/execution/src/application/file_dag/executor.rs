//! One required semantic invocation contract for ordinary and physical kernels.
//! It has no owner, dependency resolver, publication or cache acquisition API.
use crate::domain::operating::errors::OperatingError;
use ryo_operators::inbound::use_cases::operators::interface::{
    OperatorAnswer, PortableOperatorFault,
};

/// Plain semantic responses of a kernel call.
pub(crate) enum KernelAnswer {
    Complete,
    Physical(OperatorAnswer),
    Walk(crate::domain::shared_walk::types::native_batch::value::NativeBatch),
}
/// Original typed refusals of a kernel call.
pub(crate) enum KernelFault {
    Refused(OperatingError),
    Portable(PortableOperatorFault),
}

pub(crate) enum KernelCall<'call, 'resources, 'invocation> {
    Registered {
        request: super::execution::lease::capability::service::ExecutableLease<
            'call,
            'resources,
            'invocation,
        >,
    },
    Walk {
        request: super::program::native_walk::lease::service::PermittedWalk<
            'call,
            'resources,
            'invocation,
        >,
    },
    NativeJoin {
        request: super::program::native_join::lease::service::PermittedNativeJoin<
            'call,
            'resources,
            'invocation,
        >,
    },
    Physical {
        request: crate::application::file_dag::program::runtime::service::PermittedPhysical<
            'call,
            'resources,
            'invocation,
        >,
    },
}
pub(crate) trait PrimitiveExecutor: std::fmt::Debug + Sync {
    fn execute(&self, call: KernelCall<'_, '_, '_>) -> Result<KernelAnswer, KernelFault>;
}
