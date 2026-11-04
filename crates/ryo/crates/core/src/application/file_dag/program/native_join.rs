//! One admitted native Join instruction; this adapter never resolves dependencies.
use super::admission::KernelPermit;
use crate::application::file_dag::{
    executor::PrimitiveExecutor, invocation::NativeJoinLease, relations::RelationOwner,
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

pub(super) enum Eligibility {
    Native,
    Ordinary,
}
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
    pub(super) fn new(
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
    pub(in crate::application::file_dag) fn dispatch(
        self,
        kernels: &dyn PrimitiveExecutor,
    ) -> Result<(), OperatingError> {
        self.permit.validate_registered(self.permit.operation())?;
        let ryo_values::domain::plans::types::Operation::Join {
            left_on,
            filter,
            view,
            output,
            reach: None,
            ..
        } = self.permit.operation()
        else {
            return Err(OperatingError::WrongOperation);
        };
        let failure = OnceLock::new();
        let semantics = crate::application::file_dag::execution::PrimitiveSemanticsBridge {
            port: kernels,
            permit: self.permit.semantics(),
            binding: crate::domain::executable_program::physical::PhysicalBinding {
                native_join: true,
                source_len: self.context.source.0.len(),
            },
            failure: &failure,
        };
        let mut sink = Delivery {
            output: self.output,
            error: self.delivery,
        };
        let result = crate::domain::tree_join::service::join_tree(
            &semantics,
            left_on,
            &self.native.tree(),
            self.left,
            self.work,
            self.native.finding()?,
            TreeShape {
                comparisons: filter,
                view,
                output,
            },
            self.context,
            &mut sink,
        );
        if let Some(error) = failure.get() {
            return Err(error.clone());
        }
        result
    }
}
struct Delivery<'a> {
    output: &'a mut dyn Emit,
    error: &'a OnceLock<OperatingError>,
}
impl Emit for Delivery<'_> {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<(), OperatingError> {
        let result = self.output.row(context, row);
        if let Err(error) = &result {
            let _first = self.error.set(error.clone());
        }
        result
    }
}
