//! The `executor` application operation.

use super::types::{KernelAnswer, KernelCall, KernelFault};

pub(crate) trait PrimitiveExecutor: std::fmt::Debug + Sync {
    fn execute(&self, call: KernelCall<'_, '_, '_>) -> Result<KernelAnswer, KernelFault>;
}
