//! Kernel modules receive a borrowed raw order as a parameter; only the owner acquires it.
use super::super::fixtures::{Compilation, Expectation, Probe};

pub(super) const KERNEL: &str = "src/domain/native_positions/service.rs";
pub(super) const RAW_SCOPE: &str = "src/application/file_dag/derived/owner/raw_scope.rs";
const KERNEL_NAMES_SCOPE: &str = r#"
mod kernel_names_scope {
    use crate::application::file_dag::derived::owner::raw_scope::RawScope;
    fn acquire(scope: &mut RawScope) { let _ = scope; }
    const _: fn(&mut RawScope) = acquire;
}
"#;
const KERNEL_NAMES_ACQUISITION: &str = r#"
mod kernel_names_acquisition {
    use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
    fn acquire(view: &AcquiredView) { let _ = view; }
    const _: fn(&AcquiredView) = acquire;
}
"#;
const OWNER_ORDERS: &str = r#"
mod owner_orders {
    use super::RawScope;
    use crate::domain::operating::errors::OperatingError;
    use crate::domain::positions::types::{Keys, Stride};
    fn order(scope: &mut RawScope, keys: &Keys) -> Result<(), OperatingError> {
        scope.with_raw_order(keys, Stride(1), &mut |_order| Ok(()))
    }
    const _: fn(&mut RawScope, &Keys) -> Result<(), OperatingError> = order;
}
"#;

pub(super) fn controls() -> Vec<Probe> {
    vec![
        Probe {
            compilation: Compilation::Library,
            owner: KERNEL,
            source: KERNEL_NAMES_SCOPE.into(),
            expected: Expectation::Refuse {
                code: "E0603",
                first: "RawScope",
                second: "private",
            },
        },
        Probe {
            compilation: Compilation::Library,
            owner: KERNEL,
            source: KERNEL_NAMES_ACQUISITION.into(),
            expected: Expectation::Refuse {
                code: "E0603",
                first: "AcquiredView",
                second: "private",
            },
        },
        Probe {
            compilation: Compilation::Library,
            owner: RAW_SCOPE,
            source: OWNER_ORDERS.into(),
            expected: Expectation::Accept,
        },
    ]
}
