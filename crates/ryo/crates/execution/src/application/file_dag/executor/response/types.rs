//! Plain semantic responses and original typed refusals.
use crate::domain::operating::errors::OperatingError;
use ryo_operators::inbound::use_cases::operators::interface::{
    OperatorAnswer, PortableOperatorFault,
};
pub(crate) enum KernelAnswer {
    Complete,
    Physical(OperatorAnswer),
    Walk(crate::domain::shared_walk::types::native_batch::value::NativeBatch),
}
pub(crate) enum KernelFault {
    Refused(OperatingError),
    Portable(PortableOperatorFault),
}
