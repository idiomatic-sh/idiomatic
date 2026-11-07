//! The engine's one driver kernel receives borrowed views as parameters; only the owner acquires them.
use super::super::fixtures::{Compilation, Expectation, Probe};

pub(super) const KERNEL: &str = "src/domain/operator/contract/driver.rs";
const KERNEL_NAMES_ACQUISITION: &str = r#"
mod kernel_names_acquisition {
    use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
    fn acquire(view: &AcquiredView) { let _ = view; }
    const _: fn(&AcquiredView) = acquire;
}
"#;

pub(super) fn controls() -> Vec<Probe> {
    vec![Probe {
        compilation: Compilation::Library,
        owner: KERNEL,
        source: KERNEL_NAMES_ACQUISITION.into(),
        expected: Expectation::Refuse {
            code: "E0603",
            first: "AcquiredView",
            second: "private",
        },
    }]
}
