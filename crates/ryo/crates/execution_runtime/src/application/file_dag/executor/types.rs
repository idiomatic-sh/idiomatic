//! The types of the `executor` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;

/// Plain semantic responses of a kernel call.
pub(crate) enum KernelAnswer {
    Complete,
    Physical(OperatorAnswer),
    Walk(ryo_execution_domain::domain::shared_walk::types::native_batch::value::NativeBatch),
}

/// Original typed refusals of a kernel call.
pub(crate) enum KernelFault {
    Refused(OperatingError),
    Portable(PortableOperatorFault),
}

pub(crate) enum KernelCall<'call, 'resources, 'invocation> {
    Registered {
        request: super::super::execution::lease::capability::service::ExecutableLease<
            'call,
            'resources,
            'invocation,
        >,
    },
    Walk {
        request: super::super::program::native_walk::lease::service::PermittedWalk<
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
