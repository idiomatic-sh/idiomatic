//! One admitted native Join instruction; this adapter never resolves dependencies.
use crate::application::file_dag::program::runtime::service::KernelPermit;
use crate::application::file_dag::{
    invocation::native::service::NativeJoinLease, relations::RelationOwner,
};
use crate::domain::{
    operating::{
        errors::OperatingError,
        types::{Emit, OperatorContext, TreeShape},
    },
    operator::contract::types::WorkCounters,
    table_model::types::Rows,
};
use std::{rc::Rc, sync::OnceLock};

pub(crate) struct PermittedNativeJoin<'call, 'context, 'scope> {
    permit: &'call KernelPermit<'scope>,
    native: NativeJoinLease<'call>,
    left: &'call Rows<'call>,
    context: &'call mut OperatorContext<'context>,
    work: &'call WorkCounters,
    output: &'call mut dyn Emit,
    delivery: &'call OnceLock<OperatingError>,
}
impl<'call, 'context, 'scope> PermittedNativeJoin<'call, 'context, 'scope> {
    pub(in crate::application::file_dag::program) fn new(
        permit: &'call KernelPermit<'scope>,
        native: NativeJoinLease<'call>,
        left: &'call Rows<'call>,
        owner: &Rc<RelationOwner>,
        context: &'call mut OperatorContext<'context>,
        work: &'call WorkCounters,
        output: &'call mut dyn Emit,
        delivery: &'call OnceLock<OperatingError>,
    ) -> Result<Self, OperatingError> {
        permit.validate_native_join(&native, owner, left)?;
        Ok(Self {
            permit,
            native,
            left,
            context,
            work,
            output,
            delivery,
        })
    }
}

mod dispatch;
